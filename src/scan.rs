use crate::porcelain::{Status, parse};
use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Condvar, Mutex, mpsc::Sender},
    thread,
};

pub enum Msg {
    Repo(PathBuf),
    Status(PathBuf, Status),
    Remote(PathBuf, String),
    Done,
}

enum Task {
    Walk(PathBuf),
    Status(PathBuf),
}

/// (pending tasks, tasks in flight)
type Queue = Mutex<(VecDeque<Task>, usize)>;

const SKIP: &[&str] = &["node_modules", "target"];

fn cores() -> usize {
    thread::available_parallelism().map_or(4, |n| n.get())
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let o = Command::new("git")
        // Skip index.lock so background scans never collide with the user's own git.
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(dir)
        .args(args)
        // A credential prompt would block a worker forever.
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().to_owned())
    }
}

pub fn status(dir: &Path, fetch: bool) -> Status {
    let fetch_err = if fetch {
        git(dir, &["fetch", "--quiet"]).err()
    } else {
        None
    };
    let mut s = git(dir, &["status", "--porcelain=v2", "--branch"])
        .map(|o| parse(&o))
        .unwrap_or_else(|e| Status {
            error: Some(e),
            ..Default::default()
        });
    s.error = s.error.or(fetch_err);
    s
}

pub fn remote(dir: &Path) -> String {
    git(dir, &["remote", "get-url", "origin"]).map_or("—".into(), |r| r.trim().into())
}

/// One readdir per folder: `.git` among the entries marks a repo, so no extra stat.
fn list(dir: &Path) -> (bool, Vec<PathBuf>) {
    let mut kids = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else {
        return (false, kids);
    };
    for e in rd.flatten() {
        let name = e.file_name();
        let n = name.to_string_lossy();
        if n == ".git" {
            return (true, Vec::new());
        }
        if !n.starts_with('.') && !SKIP.contains(&&*n) && e.file_type().is_ok_and(|t| t.is_dir()) {
            kids.push(e.path());
        }
    }
    (false, kids)
}

fn next(q: &Queue, cv: &Condvar) -> Option<Task> {
    let mut g = q.lock().unwrap();
    loop {
        if let Some(t) = g.0.pop_front() {
            g.1 += 1;
            return Some(t);
        }
        if g.1 == 0 {
            return None;
        }
        g = cv.wait(g).unwrap();
    }
}

/// Walk and status share one pool; status jobs jump the queue so badges stream in mid-walk.
fn pool(root: &Path, tasks: VecDeque<Task>, fetch: bool, tx: &Sender<Msg>) {
    let q: Queue = Mutex::new((tasks, 0));
    let cv = Condvar::new();
    // git is IO-bound, fetch network-bound: oversubscribe.
    let workers = cores() * if fetch { 8 } else { 2 };
    thread::scope(|sc| {
        for _ in 0..workers {
            sc.spawn(|| {
                while let Some(task) = next(&q, &cv) {
                    let mut g = match task {
                        Task::Status(rel) => {
                            let s = status(&root.join(&rel), fetch);
                            let _ = tx.send(Msg::Status(rel, s));
                            q.lock().unwrap()
                        }
                        Task::Walk(dir) => {
                            let (repo, kids) = list(&dir);
                            let mut g = q.lock().unwrap();
                            if repo {
                                let rel = dir.strip_prefix(root).unwrap().to_path_buf();
                                let _ = tx.send(Msg::Repo(rel.clone()));
                                g.0.push_front(Task::Status(rel));
                            }
                            g.0.extend(kids.into_iter().map(Task::Walk));
                            g
                        }
                    };
                    g.1 -= 1;
                    cv.notify_all();
                }
            });
        }
    });
}

pub fn spawn_scan(root: PathBuf, tx: Sender<Msg>) {
    thread::spawn(move || {
        pool(&root, [Task::Walk(root.clone())].into(), false, &tx);
        let _ = tx.send(Msg::Done);
    });
}

pub fn spawn_status(root: PathBuf, rels: Vec<PathBuf>, fetch: bool, tx: Sender<Msg>) {
    let tasks = rels.into_iter().map(Task::Status).collect();
    thread::spawn(move || pool(&root, tasks, fetch, &tx));
}
