//! Ask, then (git calls, results)*, then answer: runs on a background thread.
use crate::{
    ai::{self, Config},
    risk::Risk,
    scan::Msg,
    tools,
};
use serde_json::{Value, json};
use std::{collections::HashSet, path::PathBuf, sync::mpsc::Sender, thread};

pub enum Event {
    Reply(String),
    Ran {
        cmd: String,
        out: String,
        ok: bool,
    },
    /// Blocks the agent until the user answers on `answer`.
    Confirm {
        cmd: String,
        risk: Risk,
        answer: Sender<bool>,
    },
    Error(String),
    Done(Vec<Value>),
}

pub struct Job {
    pub cfg: Config,
    pub root: PathBuf,
    pub repos: HashSet<PathBuf>,
    pub history: Vec<Value>,
    pub system: String,
}

const ROUNDS: usize = 8;

pub fn spawn(job: Job, question: String, tx: Sender<Msg>) {
    thread::spawn(move || {
        let send = |e| _ = tx.send(Msg::Ai(e));
        let mut history = job.history.clone();
        history.push(json!({ "role": "user", "content": question }));
        let mut finished = false;
        for _ in 0..ROUNDS {
            let mut msgs = vec![json!({ "role": "system", "content": job.system })];
            msgs.extend(history.iter().cloned());
            let text = match ai::complete(&job.cfg, &msgs) {
                Ok(t) => t,
                Err(e) => {
                    send(Event::Error(e));
                    finished = true;
                    break;
                }
            };
            history.push(json!({ "role": "assistant", "content": text }));
            let prose = strip_blocks(&text);
            if !prose.trim().is_empty() {
                send(Event::Reply(prose.trim().to_owned()));
            }
            let calls = tools::parse_calls(&text);
            if calls.is_empty() {
                finished = true;
                break;
            }
            let results: String = calls.into_iter().map(|c| crate::exec::run(&job, c, &tx)).collect();
            history.push(json!({ "role": "user", "content": results }));
        }
        if !finished {
            send(Event::Error(format!("stopped after {ROUNDS} rounds of git calls")));
        }
        send(Event::Done(history));
    });
}

fn strip_blocks(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find("<git") {
        out.push_str(&rest[..i]);
        rest = rest[i..].find("</git>").map_or("", |j| &rest[i + j + 6..]);
    }
    out + rest
}

#[cfg(test)]
mod tests {
    #[test]
    fn strips_git_blocks_from_prose() {
        let t = "Checking.\n<git repo=\"a\">status</git>\nthen <git repo=\"b\">log</git>done";
        assert_eq!(super::strip_blocks(t), "Checking.\n\nthen done");
    }
}
