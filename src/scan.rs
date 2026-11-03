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
}

const SKIP: &[&str] = &["node_modules", "target"];

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
                }
            });
        }
    });
}

pub fn spawn_scan(root: PathBuf, tx: Sender<Msg>) {
    thread::spawn(move || {
        let rels = discover(&root);
        let _ = tx.send(Msg::Found(rels.clone()));
        status_all(&root, rels, false, &tx);
    });
}

pub fn spawn_status(root: PathBuf, rels: Vec<PathBuf>, fetch: bool, tx: Sender<Msg>) {
    thread::spawn(move || status_all(&root, rels, fetch, &tx));
}
