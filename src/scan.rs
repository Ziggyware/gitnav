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
}

const SKIP: &[&str] = &["node_modules", "target"];

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
                }
            });
        }
    });
}

pub fn spawn_scan(root: PathBuf, tx: Sender<Msg>) {
    thread::spawn(move || {
        pool(&root, vec![Task::Walk(root.clone())], 2, &tx);
        let _ = tx.send(Msg::Walked);
    });
}

pub fn spawn_ops(root: PathBuf, rels: Vec<PathBuf>, op: Op, tx: Sender<Msg>) {
    let tasks = rels.into_iter().map(|r| Task::Status(r, op)).collect();
    // Network ops idle on I/O: oversubscribe hard.
    thread::spawn(move || pool(&root, tasks, if op == Op::Status { 2 } else { 8 }, &tx));
}
