//! Applies background results to the model and requests lazy data.
use crate::{
    app::App,
    scan::{self, Msg},
};
use std::{sync::mpsc::Sender, thread};

impl App {
    pub fn apply(&mut self, m: Msg) {
        match m {
            Msg::Repo(rel) => {
                self.repos.entry(rel.clone()).or_insert(None);
                self.seen.insert(rel);
            }
            Msg::Status(rel, s) => _ = self.repos.insert(rel, Some(s)),
            Msg::Remote(rel, url) => _ = self.remotes.insert(rel, url),
            Msg::Done => {
                self.scanning = false;
                let seen = std::mem::take(&mut self.seen);
                self.repos.retain(|p, _| seen.contains(p));
            }
        }
    }

    pub fn want_remote(&mut self, tx: &Sender<Msg>) {
        let Some(e) = self
            .selected()
            .filter(|e| e.repo && !self.remotes.contains_key(&e.rel))
        else {
            return;
        };
        self.remotes.insert(e.rel.clone(), "…".into());
        let (dir, tx) = (self.abs(&e.rel), tx.clone());
        thread::spawn(move || _ = tx.send(Msg::Remote(e.rel, scan::remote(&dir))));
    }
}
