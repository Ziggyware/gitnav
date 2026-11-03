<<<<<<< HEAD
use crate::{
    git::{git, status},
    history::{Info, Pulse, now, parse_pulse},
    porcelain::Status,
};
use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    sync::{Condvar, Mutex, mpsc::Sender},
    thread,
};

/// Only non-destructive operations: no force, no reset, fast-forward pulls only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Status,
    Fetch,
    Pull,
    Push,
}

impl Op {
    pub fn verb(self) -> &'static str {
        ["refresh", "fetch", "pull", "push"][self as usize]
    }
    fn args(self) -> &'static [&'static str] {
        match self {
            Op::Status => &[],
            Op::Fetch => &["fetch", "--quiet", "--prune"],
            Op::Pull => &["pull", "--ff-only", "--quiet"],
            Op::Push => &["push", "--quiet"],
        }
    }
}

pub enum Msg {
    Repo(PathBuf),
    Status(PathBuf, Status),
    Pulse(PathBuf, Pulse),
    Info(PathBuf, Info),
    Acted(PathBuf, Result<(), String>),
    Walked,
}

enum Task {
    Walk(PathBuf),
    Status(PathBuf, Op),
    Pulse(PathBuf),
}

/// (tasks per priority tier, tasks in flight)
type Queue = Mutex<([VecDeque<Task>; 3], usize)>;

impl Task {
    /// Badges first, discovery second, history last: pulses never slow the walk.
    fn tier(&self) -> usize {
        match self {
            Task::Status(..) => 0,
            Task::Walk(_) => 1,
            Task::Pulse(_) => 2,
        }
    }
}

fn push(tiers: &mut [VecDeque<Task>; 3], t: Task) {
    tiers[t.tier()].push_back(t);
=======
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::Sender,
    },
    thread,
};

use crate::porcelain::{Status, parse};

pub enum Msg {
    Found(Vec<PathBuf>),
    Status(PathBuf, Status),
>>>>>>> ce532fa2a485cac25364c698a4d1c4994bba308d
}

const SKIP: &[&str] = &["node_modules", "target"];

<<<<<<< HEAD
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
        if let Some(t) = g.0.iter_mut().find_map(VecDeque::pop_front) {
            g.1 += 1;
            return Some(t);
        }
        if g.1 == 0 {
            return None;
        }
        g = cv.wait(g).unwrap();
    }
}

fn run(root: &Path, task: Task, q: &Queue, tx: &Sender<Msg>) {
    let send = |m| _ = tx.send(m);
    match task {
        Task::Status(rel, op) => {
            let dir = root.join(&rel);
            if op != Op::Status {
                send(Msg::Acted(rel.clone(), git(&dir, op.args()).map(drop)));
            }
            send(Msg::Status(rel.clone(), status(&dir)));
            if op == Op::Pull {
                push(&mut q.lock().unwrap().0, Task::Pulse(rel));
            }
        }
        Task::Pulse(rel) => {
            let log = git(
                &root.join(&rel),
                &["log", "--format=%ct", "--max-count=400"],
            )
            .unwrap_or_default();
            send(Msg::Pulse(rel, parse_pulse(&log, now())));
        }
        Task::Walk(dir) => {
            let (repo, kids) = list(&dir);
            let mut g = q.lock().unwrap();
            if repo {
                let rel = dir.strip_prefix(root).unwrap().to_path_buf();
                send(Msg::Repo(rel.clone()));
                push(&mut g.0, Task::Status(rel.clone(), Op::Status));
                push(&mut g.0, Task::Pulse(rel));
            }
            kids.into_iter().for_each(|k| push(&mut g.0, Task::Walk(k)));
        }
    }
}

fn pool(root: &Path, tasks: Vec<Task>, oversubscribe: usize, tx: &Sender<Msg>) {
    let mut tiers = <[VecDeque<Task>; 3]>::default();
    tasks.into_iter().for_each(|t| push(&mut tiers, t));
    let q: Queue = Mutex::new((tiers, 0));
    let cv = Condvar::new();
    let cores = thread::available_parallelism().map_or(4, |n| n.get());
    thread::scope(|sc| {
        for _ in 0..cores * oversubscribe {
            sc.spawn(|| {
                while let Some(task) = next(&q, &cv) {
                    run(root, task, &q, tx);
                    q.lock().unwrap().1 -= 1;
                    cv.notify_all();
=======
/// Repo paths relative to `root`; does not descend into repos or follow symlinks.
pub fn discover(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if dir.join(".git").exists() {
            out.push(dir.strip_prefix(root).unwrap().to_path_buf());
            continue;
        }
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let name = e.file_name();
            let n = name.to_string_lossy();
            if !n.starts_with('.')
                && !SKIP.contains(&&*n)
                && e.file_type().is_ok_and(|t| t.is_dir())
            {
                stack.push(e.path());
            }
        }
    }
    out.sort();
    out
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let o = Command::new("git")
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
    s.remote = git(dir, &["remote", "get-url", "origin"])
        .ok()
        .map(|r| r.trim().to_owned());
    s.error = s.error.or(fetch_err);
    s
}

pub fn status_all(root: &Path, rels: Vec<PathBuf>, fetch: bool, tx: &Sender<Msg>) {
    let next = AtomicUsize::new(0);
    let cores = thread::available_parallelism().map_or(4, |n| n.get());
    let workers = (cores * if fetch { 4 } else { 1 }).min(rels.len());
    thread::scope(|sc| {
        for _ in 0..workers {
            sc.spawn(|| {
                while let Some(rel) = rels.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let _ = tx.send(Msg::Status(rel.clone(), status(&root.join(rel), fetch)));
>>>>>>> ce532fa2a485cac25364c698a4d1c4994bba308d
                }
            });
        }
    });
}

pub fn spawn_scan(root: PathBuf, tx: Sender<Msg>) {
    thread::spawn(move || {
<<<<<<< HEAD
        pool(&root, vec![Task::Walk(root.clone())], 2, &tx);
        let _ = tx.send(Msg::Walked);
    });
}

pub fn spawn_ops(root: PathBuf, rels: Vec<PathBuf>, op: Op, tx: Sender<Msg>) {
    let tasks = rels.into_iter().map(|r| Task::Status(r, op)).collect();
    // Network ops idle on I/O: oversubscribe hard.
    thread::spawn(move || pool(&root, tasks, if op == Op::Status { 2 } else { 8 }, &tx));
=======
        let rels = discover(&root);
        let _ = tx.send(Msg::Found(rels.clone()));
        status_all(&root, rels, false, &tx);
    });
}

pub fn spawn_status(root: PathBuf, rels: Vec<PathBuf>, fetch: bool, tx: Sender<Msg>) {
    thread::spawn(move || status_all(&root, rels, fetch, &tx));
>>>>>>> ce532fa2a485cac25364c698a4d1c4994bba308d
}
