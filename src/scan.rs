use crate::{
    git::{self, git, status},
    history::{Info, Pulse},
    porcelain::Status,
};
use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    sync::{Condvar, Mutex, mpsc::Sender},
    thread,
};

/// Operations dispatched across single or marked repositories.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Status,
    Fetch,
    Pull,
    Push,
    Commit(String),
    BranchSwitch(String),
    BranchDelete(String),
    UnifyMain,
    Adopt(String),
}

impl Op {
    pub fn verb(&self) -> &'static str {
        match self {
            Op::Status => "refresh",
            Op::Fetch => "fetch",
            Op::Pull => "smart-pull",
            Op::Push => "smart-push",
            Op::Commit(_) => "commit",
            Op::BranchSwitch(_) => "branch",
            Op::BranchDelete(_) => "del-branch",
            Op::UnifyMain => "unify-main",
            Op::Adopt(_) => "publish-github",
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
    Ai(crate::agent::Event),
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

const SKIP: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    "vendor",
    "__pycache__",
    "venv",
];

const MANIFESTS: &[&str] = &[
    "Cargo.toml",
    "package.json",
    "deno.json",
    "deno.jsonc",
    "pyproject.toml",
    "go.mod",
    "Makefile",
    "CMakeLists.txt",
    "build.zig",
    "pom.xml",
    "flake.nix",
];

const CODE_EXTS: &[&str] = &[
    "rs", "ts", "tsx", "js", "py", "go", "c", "cpp", "h", "cs", "zig", "lua", "sh", "ps1", "html",
];

/// One readdir per folder: `.git` marks a tracked repo; project manifests or leaf source files mark an Unborn app.
fn list(dir: &Path, is_root: bool) -> (bool, Vec<PathBuf>) {
    let mut kids = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else {
        return (false, kids);
    };
    let mut has_manifest = false;
    let mut code_files = 0usize;
    let mut has_src_dir = false;

    for e in rd.flatten() {
        let name = e.file_name();
        let n = name.to_string_lossy();
        if n == ".git" {
            return (true, Vec::new());
        }
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            if n == "src" {
                has_src_dir = true;
            }
            if !n.starts_with('.') && !SKIP.contains(&&*n) {
                kids.push(e.path());
            }
        } else if ft.is_file() {
            if MANIFESTS.contains(&&*n) || n.ends_with(".sln") || n.ends_with(".csproj") {
                has_manifest = true;
            }
            if let Some((_, ext)) = n.rsplit_once('.') {
                if CODE_EXTS.contains(&ext) {
                    code_files += 1;
                }
            }
        }
    }

    if !is_root && (has_manifest || has_src_dir || (kids.is_empty() && code_files >= 2)) {
        return (true, Vec::new());
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

fn execute_op(dir: &Path, op: &Op) -> Result<(), String> {
    match op {
        Op::Status => Ok(()),
        Op::Fetch => git(dir, &["fetch", "--quiet", "--prune"]).map(drop),
        Op::Pull => git::smart_pull(dir),
        Op::Push => git::smart_push(dir),
        Op::Commit(msg) => git::stage_and_commit(dir, msg),
        Op::BranchSwitch(br) => git::switch_or_create_branch(dir, br),
        Op::BranchDelete(br) => git::delete_branch(dir, br),
        Op::UnifyMain => git::unify_main_master(dir),
        Op::Adopt(spec) => git::adopt_and_publish(dir, spec),
    }
}

fn run(root: &Path, task: Task, q: &Queue, tx: &Sender<Msg>) {
    let send = |m| _ = tx.send(m);
    match task {
        Task::Status(rel, op) => {
            let dir = if rel.as_os_str().is_empty() {
                root.to_path_buf()
            } else {
                root.join(&rel)
            };
            let refresh_pulse = !matches!(op, Op::Status | Op::Fetch);
            if op != Op::Status {
                send(Msg::Acted(rel.clone(), execute_op(&dir, &op)));
            }
            send(Msg::Status(rel.clone(), status(&dir)));
            if refresh_pulse {
                push(&mut q.lock().unwrap().0, Task::Pulse(rel));
            }
        }
        Task::Pulse(rel) => {
            let dir = if rel.as_os_str().is_empty() {
                root.to_path_buf()
            } else {
                root.join(&rel)
            };
            let p = crate::git::pulse(&dir);
            send(Msg::Pulse(rel, p));
        }
        Task::Walk(dir) => {
            let (repo, kids) = list(&dir, dir == root);
            let mut g = q.lock().unwrap();
            if repo {
                let rel = dir.strip_prefix(root).unwrap_or(Path::new("")).to_path_buf();
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
    let oversub = if op == Op::Status { 2 } else { 8 };
    let tasks = rels.into_iter().map(|r| Task::Status(r, op.clone())).collect();
    thread::spawn(move || pool(&root, tasks, oversub, &tx));
}
