//! Which git commands the assistant may run, and which need the user's yes.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Risk {
    /// Inspects only: runs without asking.
    Read,
    /// Changes a repo or a remote: needs the user's yes.
    Write,
    /// Can discard work or rewrite shared history: needs a yes, shown in red.
    Destructive,
}

const READ: &[&str] = &[
    "status",
    "log",
    "diff",
    "show",
    "blame",
    "shortlog",
    "rev-parse",
    "rev-list",
    "ls-files",
    "ls-tree",
    "describe",
    "grep",
    "cat-file",
    "count-objects",
    "for-each-ref",
    "name-rev",
    "merge-base",
    "cherry",
    "whatchanged",
];
const WRITE: &[&str] = &[
    "fetch",
    "pull",
    "push",
    "commit",
    "add",
    "restore",
    "checkout",
    "switch",
    "merge",
    "rebase",
    "stash",
    "reset",
    "tag",
    "branch",
    "cherry-pick",
    "revert",
    "clean",
    "rm",
    "mv",
    "remote",
    "reflog",
    "gc",
    "prune",
];
/// Options that write files or start programs. Top-level git options can't appear: args[0] is the subcommand.
const BANNED: &[&str] = &[
    "--output",
    "--upload-pack",
    "--receive-pack",
    "--exec",
    "-O",
    "--open-files-in-pager",
];

pub fn classify(args: &[String]) -> Result<Risk, String> {
    let (sub, rest) = args.split_first().ok_or("empty command")?;
    let has = |f: &[&str]| {
        rest.iter()
            .any(|a| f.iter().any(|x| a == x || a.starts_with(&format!("{x}="))))
    };
    if args
        .iter()
        .any(|a| BANNED.iter().any(|b| a == b || a.starts_with(&format!("{b}="))))
    {
        return Err(format!("option not allowed: one of {}", BANNED.join(" ")));
    }
    let flags_only = rest.iter().all(|a| a.starts_with('-'));
    let read = READ.contains(&sub.as_str())
        || (sub == "branch"
            && flags_only
            && !has(&[
                "-d",
                "-D",
                "-m",
                "-M",
                "-c",
                "-C",
                "--delete",
                "--move",
                "-u",
                "--set-upstream-to",
                "--unset-upstream",
            ]))
        || (sub == "tag" && (rest.is_empty() || has(&["-l", "--list"])) && !has(&["-d", "--delete"]))
        || (sub == "remote" && (flags_only || matches!(rest[0].as_str(), "show" | "get-url")))
        || (sub == "stash" && matches!(rest.first().map(String::as_str), Some("list" | "show")))
        || (sub == "reflog" && matches!(rest.first().map(String::as_str), None | Some("show")));
    if read {
        return Ok(Risk::Read);
    }
    if !WRITE.contains(&sub.as_str()) {
        return Err(format!("git {sub} is not available to the assistant"));
    }
    let destructive = match sub.as_str() {
        "reset" => has(&["--hard", "--merge", "--keep"]),
        "push" => has(&[
            "-f",
            "--force",
            "--force-with-lease",
            "--delete",
            "-d",
            "--mirror",
            "--prune",
        ]),
        "branch" => has(&["-D", "--delete"]) || rest.iter().any(|a| a == "-d"),
        "checkout" => rest
            .iter()
            .any(|a| a == "--" || a == "." || a == "-f" || a == "--force"),
        "stash" => matches!(rest.first().map(String::as_str), Some("drop" | "clear")),
        "clean" | "rebase" | "gc" | "prune" | "reflog" => true,
        "restore" => !has(&["--staged"]) || has(&["--worktree", "-W"]),
        "tag" => has(&["-d", "--delete", "-f", "--force"]),
        _ => false,
    };
    Ok(if destructive { Risk::Destructive } else { Risk::Write })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::split;

    fn c(s: &str) -> Result<Risk, String> {
        classify(&split(s).unwrap())
    }

    #[test]
    fn gates_by_risk() {
        assert_eq!(c("log --oneline -5"), Ok(Risk::Read));
        assert_eq!(c("branch -vv"), Ok(Risk::Read));
        assert_eq!(c("remote -v"), Ok(Risk::Read));
        assert_eq!(c("branch feature"), Ok(Risk::Write));
        assert_eq!(c("switch -c topic"), Ok(Risk::Write));
        assert_eq!(c("pull --ff-only"), Ok(Risk::Write));
        assert_eq!(c("push --force-with-lease"), Ok(Risk::Destructive));
        assert_eq!(c("reset --hard HEAD~1"), Ok(Risk::Destructive));
        assert_eq!(c("checkout -- src/a.rs"), Ok(Risk::Destructive));
        assert_eq!(c("restore --staged a.rs"), Ok(Risk::Write));
        assert!(c("diff --output=/etc/x").is_err());
        assert!(c("config core.pager x").is_err());
        assert!(c("fetch --upload-pack=evil").is_err());
    }
}
