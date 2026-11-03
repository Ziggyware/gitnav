use crate::{
    history::{Info, parse_numstat},
    porcelain::{Status, parse},
};
use std::{
    path::Path,
    process::{Command, Stdio},
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

pub fn info(dir: &Path) -> Info {
    let head = git(dir, &["log", "-1", "--format=%s%x1f%an%x1f%ct"]).unwrap_or_default();
    let mut h = head.trim_end().splitn(3, '\x1f');
    Info {
        remote: git(dir, &["remote", "get-url", "origin"])
            .map_or("no remote".into(), |r| r.trim().into()),
        subject: h.next().unwrap_or("").into(),
        author: h.next().unwrap_or("").into(),
        when: h.next().and_then(|t| t.parse().ok()).unwrap_or(0),
        stat: git(dir, &["diff", "HEAD", "--numstat"])
            .map_or_else(|_| Vec::new(), |o| parse_numstat(&o)),
    }
}
