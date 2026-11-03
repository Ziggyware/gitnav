use crate::{
    history::{DAYS, Info, Pulse, now, parse_numstat, parse_pulse},
    porcelain::{Status, parse},
};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
    time::UNIX_EPOCH,
};

pub fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
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

pub fn status(dir: &Path) -> Status {
    git(dir, &["status", "--porcelain=v2", "--branch"])
        .map(|o| parse(&o))
        .unwrap_or_else(|e| Status {
            error: Some(e),
            ..Default::default()
        })
}

/// `paths`: the repo's uncommitted files, whose mtimes say how long work has sat uncommitted.
pub fn info(dir: &Path, paths: &[String]) -> Info {
    let head = git(dir, &["log", "-1", "--format=%s%x1f%an%x1f%ct"]).unwrap_or_default();
    let mut h = head.trim_end().splitn(3, '\x1f');
    let mtime = |p: &String| {
        let m = fs::metadata(dir.join(p)).and_then(|m| m.modified()).ok()?;
        Some(m.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64)
    };
    let times: Vec<i64> = paths.iter().filter_map(mtime).collect();
    let edits = (
        times.iter().copied().min().unwrap_or(0),
        times.iter().copied().max().unwrap_or(0),
    );
    Info {
        remote: git(dir, &["remote", "get-url", "origin"]).map_or("no remote".into(), |r| r.trim().into()),
        subject: h.next().unwrap_or("").into(),
        author: h.next().unwrap_or("").into(),
        when: h.next().and_then(|t| t.parse().ok()).unwrap_or(0),
        stat: git(dir, &["diff", "HEAD", "--numstat"]).map_or_else(|_| Vec::new(), |o| parse_numstat(&o)),
        edits,
    }
}

/// Windowed history; one extra call only when the window is empty, to date the last commit.
pub fn pulse(dir: &Path) -> Pulse {
    let since = format!("--since={DAYS}.days");
    let args = ["log", "--format=%ct %ad", "--date=format-local:%z", since.as_str()];
    let mut p = parse_pulse(&git(dir, &args).unwrap_or_default(), now());
    if p.last == 0 {
        p.last = git(dir, &["log", "-1", "--format=%ct"])
            .ok()
            .and_then(|t| t.trim().parse().ok())
            .unwrap_or(0);
    }
    p
}
