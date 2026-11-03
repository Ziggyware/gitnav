use crate::{
    github,
    history::{DAYS, Info, Pulse, now},
    log_parse::{parse_numstat, parse_pulse},
    porcelain::{BranchRef, Status, parse},
};
use std::{
    collections::HashSet,
    fs,
    path::Path,
    process::{Command, Stdio},
    time::UNIX_EPOCH,
};

pub fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let o = Command::new("git")
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(dir)
        .args(args)
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

/// Mutating git call that acquires index locks normally.
pub fn git_mut(dir: &Path, args: &[&str]) -> Result<String, String> {
    let o = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
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
    if !dir.join(".git").exists() {
        let changes = scan_unborn_files(dir);
        return Status {
            branch: "(unborn)".into(),
            unborn: true,
            has_remote: false,
            changes,
            ..Default::default()
        };
    }
    let mut s = git(dir, &["status", "--porcelain=v2", "--branch"])
        .map(|o| parse(&o))
        .unwrap_or_else(|e| Status {
            error: Some(e),
            ..Default::default()
        });
    s.has_remote = git(dir, &["remote", "get-url", "origin"]).is_ok();
    s.branches = list_branches(dir);
    s
}

fn scan_unborn_files(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else {
        return out;
    };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !name.starts_with('.') && name != "target" && name != "node_modules" {
            out.push(format!("?? {name}"));
        }
    }
    out.sort();
    out
}

pub fn list_branches(dir: &Path) -> Vec<BranchRef> {
    let fmt = "--format=%(HEAD)\x1f%(refname:short)\x1f%(upstream:short)\x1f%(upstream:track,nobracket)";
    let Ok(out) = git(dir, &["for-each-ref", fmt, "refs/heads", "refs/remotes/origin"]) else {
        return Vec::new();
    };
    let mut local_names = HashSet::new();
    let mut rows = Vec::new();
    for line in out.lines() {
        let mut f = line.splitn(4, '\x1f');
        let (head_s, Some(name), up_s, track_s) = (
            f.next().unwrap_or(""),
            f.next(),
            f.next().unwrap_or(""),
            f.next().unwrap_or(""),
        ) else {
            continue;
        };
        if name == "origin/HEAD" || name == "origin" {
            continue;
        }
        let remote_only = name.starts_with("origin/");
        if !remote_only {
            local_names.insert(name.to_owned());
        }
        let (mut ahead, mut behind) = (0, 0);
        for part in track_s.split(", ") {
            if let Some(a) = part.strip_prefix("ahead ") {
                ahead = a.parse().unwrap_or(0);
            } else if let Some(b) = part.strip_prefix("behind ") {
                behind = b.parse().unwrap_or(0);
            }
        }
        rows.push(BranchRef {
            name: name.to_owned(),
            upstream: (!up_s.is_empty()).then(|| up_s.to_owned()),
            ahead,
            behind,
            head: head_s == "*",
            remote_only,
        });
    }
    rows.retain(|b| {
        if b.remote_only {
            let short = b.name.strip_prefix("origin/").unwrap_or(&b.name);
            !local_names.contains(short)
        } else {
            true
        }
    });
    rows.sort_by(|a, b| {
        b.head
            .cmp(&a.head)
            .then(a.remote_only.cmp(&b.remote_only))
            .then(a.name.cmp(&b.name))
    });
    rows
}

fn synthesize_gitignore(dir: &Path) {
    let gi = dir.join(".gitignore");
    if gi.exists() {
        return;
    }
    let mut rules = Vec::new();
    if dir.join("Cargo.toml").exists() {
        rules.push("/target\n");
    }
    if dir.join("package.json").exists() || dir.join("deno.json").exists() {
        rules.push("node_modules/\n.env\ndist/\n");
    }
    if dir.join("pyproject.toml").exists() || dir.join("requirements.txt").exists() {
        rules.push("__pycache__/\n*.py[cod]\n.venv/\n");
    }
    if rules.is_empty() {
        rules.push("/target\nnode_modules/\n.DS_Store\n.env\n");
    }
    let _ = fs::write(gi, rules.concat());
}

/// Initializes an unborn folder or stages + commits all changes.
pub fn stage_and_commit(dir: &Path, msg: &str) -> Result<(), String> {
    let msg = if msg.trim().is_empty() {
        "chore: checkpoint"
    } else {
        msg.trim()
    };
    if !dir.join(".git").exists() {
        synthesize_gitignore(dir);
        git_mut(dir, &["init", "-b", "main"])?;
    }
    git_mut(dir, &["add", "-A"])?;
    let st = git(dir, &["status", "--porcelain"])?;
    if st.trim().is_empty() {
        return Err("working tree clean: nothing to commit".into());
    }
    git_mut(dir, &["commit", "-m", msg]).map(drop)
}

/// Autonomic branch switch/create with dirty-worktree stash bracket.
pub fn switch_or_create_branch(dir: &Path, target: &str) -> Result<(), String> {
    let target = target.trim();
    if target.is_empty() {
        return Err("empty branch name".into());
    }
    if !dir.join(".git").exists() {
        stage_and_commit(dir, "feat: initial commit")?;
    }
    let dirty = !git(dir, &["status", "--porcelain"])?.trim().is_empty();
    if dirty {
        let _ = git_mut(dir, &["stash", "push", "--include-untracked", "-m", "gitnav-autostash"]);
    }
    let res = if let Some(short) = target.strip_prefix("origin/") {
        git_mut(dir, &["switch", "-c", short, "--track", target])
            .or_else(|_| git_mut(dir, &["switch", short]))
    } else if git(dir, &[" show-ref", "--verify", &format!("refs/heads/{target}")]).is_ok()
        || git(dir, &["rev-parse", "--verify", target]).is_ok()
    {
        git_mut(dir, &["switch", target])
    } else {
        git_mut(dir, &["switch", "-c", target])
    };
    if dirty {
        let _ = git_mut(dir, &["stash", "pop"]);
    }
    res.map(drop)
}

pub fn delete_branch(dir: &Path, target: &str) -> Result<(), String> {
    let target = target.trim();
    if let Some(remote_br) = target.strip_prefix("origin/") {
        git_mut(dir, &["push", "origin", "--delete", remote_br]).map(drop)
    } else {
        git_mut(dir, &["branch", "-d", target])
            .or_else(|_| git_mut(dir, &["branch", "-D", target]))
            .map(drop)
    }
}

/// Unifies disjoint `main` and `master` branches, keeping local files and aligning on `main`.
pub fn unify_main_master(dir: &Path) -> Result<(), String> {
    let branches = list_branches(dir);
    let has_master = branches
        .iter()
        .any(|b| b.name == "master" || b.name == "origin/master");
    if !has_master {
        return Err("no master branch detected to unify".into());
    }
    let dirty = !git(dir, &["status", "--porcelain"])?.trim().is_empty();
    if dirty {
        let _ = git_mut(dir, &["stash", "push", "--include-untracked", "-m", "gitnav-unify"]);
    }
    if git(dir, &["rev-parse", "--verify", "main"]).is_err() {
        let _ = git_mut(dir, &["branch", "-M", "main"]);
    } else {
        let _ = git_mut(dir, &["switch", "main"]);
        let _ = git_mut(
            dir,
            &[
                "merge",
                "master",
                "--allow-unrelated-histories",
                "-s",
                "ours",
                "--no-edit",
            ],
        );
        let _ = git_mut(dir, &["branch", "-D", "master"]);
    }
    if branches.iter().any(|b| b.name == "origin/master") {
        let _ = git_mut(
            dir,
            &[
                "merge",
                "origin/master",
                "--allow-unrelated-histories",
                "-s",
                "ours",
                "--no-edit",
            ],
        );
    }
    if dirty {
        let _ = git_mut(dir, &["stash", "pop"]);
    }
    Ok(())
}

/// Smart Pull / Reconcile: auto-stashes dirty work, fetches, and grafts disjoint histories if needed.
pub fn smart_pull(dir: &Path) -> Result<(), String> {
    if !dir.join(".git").exists() {
        return Err("unborn project: press G to initialize and publish".into());
    }
    let dirty = !git(dir, &["status", "--porcelain"])?.trim().is_empty();
    if dirty {
        git_mut(dir, &["stash", "push", "--include-untracked", "-m", "gitnav-pull"])?;
    }
    let res = (|| -> Result<(), String> {
        git_mut(dir, &["fetch", "--prune"])?;
        if git_mut(dir, &["pull", "--ff-only", "--quiet"]).is_ok() {
            return Ok(());
        }
        // Fallback: rebase or graft unrelated history while preserving local files
        if git_mut(dir, &["pull", "--rebase", "--quiet"]).is_ok() {
            return Ok(());
        }
        let _ = git_mut(dir, &["rebase", "--abort"]);
        let head = git(dir, &["symbolic-ref", "--short", "HEAD"])
            .unwrap_or_else(|_| "main".into())
            .trim()
            .to_owned();
        let remote_ref = format!("origin/{head}");
        graft_remote_ref(dir, &remote_ref)
    })();
    if dirty {
        let _ = git_mut(dir, &["stash", "pop"]);
    }
    res
}

/// Smart Push: sets upstream automatically and grafts remote seed commits if rejected as non-fast-forward.
pub fn smart_push(dir: &Path) -> Result<(), String> {
    if !dir.join(".git").exists() {
        return adopt_and_publish(dir, "");
    }
    let head = git(dir, &["symbolic-ref", "--short", "HEAD"])
        .unwrap_or_else(|_| "main".into())
        .trim()
        .to_owned();
    if git(dir, &["remote", "get-url", "origin"]).is_err() {
        return adopt_and_publish(dir, "");
    }
    if git_mut(dir, &["push", "-u", "origin", &head]).is_ok() {
        return Ok(());
    }
    // Non-fast-forward or unrelated remote seed commit (e.g. README/LICENSE created on GitHub):
    git_mut(dir, &["fetch", "origin"])?;
    let remote_ref = if git(dir, &["rev-parse", "--verify", &format!("origin/{head}")]).is_ok() {
        format!("origin/{head}")
    } else if git(dir, &["rev-parse", "--verify", "origin/main"]).is_ok() {
        "origin/main".into()
    } else if git(dir, &["rev-parse", "--verify", "origin/master"]).is_ok() {
        "origin/master".into()
    } else {
        return Err("push rejected and no remote branch found to graft".into());
    };
    graft_remote_ref(dir, &remote_ref)?;
    git_mut(dir, &["push", "-u", "origin", &head]).map(drop)
}

/// Grafts `remote_ref` (e.g., `origin/main`) into current HEAD even across disjoint root histories,
/// pulling in any non-colliding files from the remote (like LICENSE/README) while local files win.
fn graft_remote_ref(dir: &Path, remote_ref: &str) -> Result<(), String> {
    if let Ok(remote_files) = git(dir, &["ls-tree", "-r", "--name-only", remote_ref]) {
        for f in remote_files.lines() {
            if !f.is_empty() && !dir.join(f).exists() {
                let _ = git_mut(dir, &["checkout", remote_ref, "--", f]);
            }
        }
        let st = git(dir, &["status", "--porcelain"]).unwrap_or_default();
        if !st.trim().is_empty() {
            let _ = git_mut(dir, &[" commit", "-m", "chore: import non-colliding remote files"]);
        }
    }
    git_mut(
        dir,
        &[
            "merge",
            remote_ref,
            "--allow-unrelated-histories",
            "-s",
            "ours",
            "-m",
            "chore: graft remote history onto local tree",
        ],
    )
    .map(drop)
}

/// Full local-folder + GitHub bridge:
/// 1. Initializes `.git` and `.gitignore` if unborn.
/// 2. Commits any uncommitted local source code.
/// 3. Provisions or attaches the GitHub remote repository.
/// 4. Grafts any disjoint remote seed commits (README/LICENSE) and pushes `main` with upstream tracking.
pub fn adopt_and_publish(dir: &Path, spec: &str) -> Result<(), String> {
    if !dir.join(".git").exists() {
        synthesize_gitignore(dir);
        git_mut(dir, &["init", "-b", "main"])?;
    }
    git_mut(dir, &["add", "-A"])?;
    if !git(dir, &["status", "--porcelain"])?.trim().is_empty() {
        git_mut(dir, &["commit", "-m", "feat: initial source import"])?;
    } else if git(dir, &["rev-parse", "--verify", "HEAD"]).is_err() {
        git_mut(dir, &["commit", "--allow-empty", "-m", "feat: initialize repository"])?;
    }
    let _ = git_mut(dir, &["branch", "-M", "main"]);

    let url = if let Ok(existing) = git(dir, &["remote", "get-url", "origin"]) {
        let existing = existing.trim().to_owned();
        if !spec.trim().is_empty() {
            let new_url = github::ensure_remote_repo(dir, spec)?;
            let _ = git_mut(dir, &["remote", "set-url", "origin", &new_url]);
            new_url
        } else {
            existing
        }
    } else {
        let new_url = github::ensure_remote_repo(dir, spec)?;
        git_mut(dir, &["remote", "add", "origin", &new_url])?;
        new_url
    };

    let _ = url;
    if git_mut(dir, &["push", "-u", "origin", "main"]).is_err() {
        git_mut(dir, &["fetch", "origin"])?;
        if git(dir, &["rev-parse", "--verify", "origin/main"]).is_ok() {
            graft_remote_ref(dir, "origin/main")?;
        } else if git(dir, &["rev-parse", "--verify", "origin/master"]).is_ok() {
            graft_remote_ref(dir, "origin/master")?;
            let _ = git_mut(dir, &["push", "origin", "--delete", "master"]);
        }
        git_mut(dir, &["push", "-u", "origin", "main"])?;
    }
    Ok(())
}

/// `paths`: the repo's uncommitted files, whose mtimes say how long work has sat uncommitted.
pub fn info(dir: &Path, paths: &[String]) -> Info {
    let mtime = |p: &String| {
        let m = fs::metadata(dir.join(p)).and_then(|m| m.modified()).ok()?;
        Some(m.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64)
    };
    let times: Vec<i64> = paths.iter().filter_map(mtime).collect();
    let edits = (
        times.iter().copied().min().unwrap_or(0),
        times.iter().copied().max().unwrap_or(0),
    );
    if !dir.join(".git").exists() {
        return Info {
            remote: "untracked local project (press G to init + publish)".into(),
            subject: "never initialized with git".into(),
            author: "local".into(),
            when: edits.1,
            stat: Vec::new(),
            edits,
        };
    }
    let head = git(dir, &["log", "-1", "--format=%s%x1f%an%x1f%ct"]).unwrap_or_default();
    let mut h = head.trim_end().splitn(3, '\x1f');
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
    if !dir.join(".git").exists() {
        let last = fs::read_dir(dir)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.metadata().ok()?.modified().ok()?.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .max()
            .unwrap_or(0);
        return Pulse {
            last,
            today: now() / 86_400,
            ..Default::default()
        };
    }
    let since = format!("--since={DAYS}.days");
    let fmt = "--format=@%ct %ad%x1f%an%x1f%s";
    let args = ["log", fmt, "--numstat", "--date=format-local:%z", since.as_str()];
    let mut p = parse_pulse(&git(dir, &args).unwrap_or_default(), now());
    if p.last == 0 {
        p.last = git(dir, &["log", "-1", "--format=%ct"])
            .ok()
            .and_then(|t| t.trim().parse().ok())
            .unwrap_or(0);
    }
    p
}
