//! Runs one model-requested git call: validate, gate on risk, execute, report.
use crate::{
    agent::{Event, Job},
    git,
    risk::{self, Risk},
    scan::Msg,
    tools::Call,
};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Sender},
};

const OUTPUT_CAP: usize = 6_000;

fn result(repo: &str, cmd: &str, exit: &str, body: &str) -> String {
    format!("<result repo=\"{repo}\" cmd=\"{cmd}\" exit=\"{exit}\">\n{body}\n</result>\n")
}

pub fn run(job: &Job, call: Result<Call, String>, tx: &Sender<Msg>) -> String {
    let send = |e| _ = tx.send(Msg::Ai(e));
    let call = match call {
        Ok(c) => c,
        Err(e) => return result("?", "?", "rejected", &e),
    };
    let (line, shown) = (
        call.args.join(" "),
        format!("git -C {} {}", call.repo, call.args.join(" ")),
    );
    let reject = |why: String| {
        send(Event::Ran {
            cmd: shown.clone(),
            out: why.clone(),
            ok: false,
        });
        result(&call.repo, &line, "rejected", &why)
    };
    let rel = if call.repo == "." {
        PathBuf::new()
    } else {
        PathBuf::from(&call.repo)
    };
    if !job.repos.contains(&rel) {
        return reject("unknown repo: use a path from the snapshot".into());
    }
    let risk = match risk::classify(&call.args) {
        Ok(r) => r,
        Err(e) => return reject(e),
    };
    if risk != Risk::Read {
        let (answer, wait) = mpsc::channel();
        send(Event::Confirm {
            cmd: shown.clone(),
            risk,
            answer,
        });
        if !wait.recv().unwrap_or(false) {
            send(Event::Ran {
                cmd: shown,
                out: "denied".into(),
                ok: false,
            });
            return result(
                &call.repo,
                &line,
                "denied",
                "the user denied this command; do not retry it unless asked",
            );
        }
    }
    let dir = if rel.as_os_str().is_empty() {
        job.root.clone()
    } else {
        job.root.join(&rel)
    };
    let args: Vec<&str> = call.args.iter().map(String::as_str).collect();
    let (ok, out) = match git::git(&dir, &args) {
        Ok(o) => (true, o),
        Err(e) => (false, e),
    };
    if risk != Risk::Read {
        let _ = tx.send(Msg::Status(rel, git::status(&dir)));
    }
    let capped: String = out.chars().take(OUTPUT_CAP).collect();
    let note = if capped.len() < out.len() {
        "\n[output truncated]"
    } else {
        ""
    };
    let body = if out.trim().is_empty() {
        "(no output)".to_owned()
    } else {
        capped + note
    };
    send(Event::Ran { cmd: shown, out, ok });
    result(&call.repo, &line, if ok { "0" } else { "error" }, &body)
}
