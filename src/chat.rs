//! Chat state, key entry, and agent event handling.
use crate::{
    agent::{self, Event, Job},
    ai::Config,
    app::App,
    risk::Risk,
};
use serde_json::Value;
use std::sync::mpsc::Sender;

pub enum Item {
    You(String),
    Ai(String),
    Ran { cmd: String, out: String, ok: bool },
    Error(String),
}

#[derive(Default)]
pub struct Chat {
    pub open: bool,
    pub focus: bool,
    pub input: String,
    pub items: Vec<Item>,
    pub history: Vec<Value>,
    pub busy: bool,
    /// Lines scrolled up from the bottom.
    pub scroll: usize,
    pub confirm: Option<(String, Risk, Sender<bool>)>,
    pub cfg: Option<Config>,
    /// Input is the router key: masked, saved on enter, never sent anywhere but the router.
    pub key_entry: bool,
}

impl App {
    pub fn open_chat(&mut self) {
        let c = &mut self.chat;
        (c.open, c.focus) = (true, true);
        c.key_entry |= c.cfg.as_ref().is_none_or(|c| c.key.is_none());
    }

    fn save_key(&mut self) {
        let key = std::mem::take(&mut self.chat.input).trim().to_owned();
        let c = &mut self.chat;
        if key.is_empty() {
            return;
        }
        c.key_entry = false;
        match crate::ai::save_key(&key) {
            Ok(p) => c.items.push(Item::Ai(format!(
                "Key saved to {} (owner-only). Ask away.",
                p.display()
            ))),
            Err(e) => c.items.push(Item::Error(format!(
                "key works for this session but was not saved: {e}"
            ))),
        }
        if let Some(cfg) = &mut c.cfg {
            cfg.key = Some(key);
        }
    }

    pub fn ask(&mut self, tx: &Sender<crate::scan::Msg>) {
        if self.chat.key_entry {
            return self.save_key();
        }
        if self.chat.input.trim() == "/key" {
            (self.chat.input, self.chat.key_entry) = (String::new(), true);
            return;
        }
        let q = std::mem::take(&mut self.chat.input).trim().to_owned();
        if q.is_empty() || self.chat.busy {
            return;
        }
        self.chat.items.push(Item::You(q.clone()));
        (self.chat.busy, self.chat.scroll) = (true, 0);
        let job = Job {
            cfg: self.chat.cfg.clone().unwrap_or_else(Config::from_env),
            root: self.root.clone(),
            repos: self.repos.keys().cloned().collect(),
            history: self.chat.history.clone(),
            system: self.system_prompt(),
        };
        agent::spawn(job, q, tx.clone());
    }

    pub fn on_ai(&mut self, e: Event) {
        let c = &mut self.chat;
        match e {
            Event::Reply(t) => c.items.push(Item::Ai(t)),
            Event::Ran { cmd, out, ok } => c.items.push(Item::Ran { cmd, out, ok }),
            Event::Confirm { cmd, risk, answer } => c.confirm = Some((cmd, risk, answer)),
            Event::Error(e) => c.items.push(Item::Error(e)),
            Event::Done(h) => (c.history, c.busy) = (h, false),
        }
        c.scroll = 0;
    }
}
