//! Background results in, user intents out.
use crate::{
    app::App,
    porcelain::Tone,
    scan::{self, Msg, Op},
};
use std::{path::PathBuf, sync::mpsc::Sender, thread};

/// Progress of one fetch/pull/push across its targets.
pub struct Batch {
    pub op: Op,
    pub total: usize,
    pub ok: usize,
    pub failed: usize,
    pub last_err: Option<(PathBuf, String)>,
}

impl App {
    pub fn apply(&mut self, m: Msg) {
        match m {
            Msg::Repo(rel) => {
                self.repos.entry(rel.clone()).or_insert(None);
                self.seen.insert(rel);
            }
            Msg::Status(rel, s) => _ = self.repos.insert(rel, Some(s)),
            Msg::Pulse(rel, p) => _ = self.pulses.insert(rel, p),
            Msg::Info(rel, i) => _ = self.infos.insert(rel, Some(i)),
            Msg::Acted(rel, r) => {
                self.busy.remove(&rel);
                self.infos.remove(&rel);
                let Some(b) = &mut self.batch else { return };
                match r {
                    Ok(()) => b.ok += 1,
                    Err(e) => {
                        b.failed += 1;
                        b.last_err = Some((rel.clone(), e.lines().next().unwrap_or("").to_owned()));
                        self.failures.insert(rel, e);
                    }
                }
            }
            Msg::Walked => {
                self.scanning = false;
                let seen = std::mem::take(&mut self.seen);
                self.repos.retain(|p, _| seen.contains(p));
                self.pulses.retain(|p, _| seen.contains(p));
            }
        }
    }

    pub fn want_info(&mut self, tx: &Sender<Msg>) {
        let Some(e) = self
            .selected()
            .filter(|e| e.repo && !self.infos.contains_key(&e.rel))
        else {
            return;
        };
        self.infos.insert(e.rel.clone(), None);
        let (dir, tx) = (self.abs(&e.rel), tx.clone());
        thread::spawn(move || _ = tx.send(Msg::Info(e.rel, crate::git::info(&dir))));
    }

    /// Marked repos, else everything under the selection.
    fn targets(&self) -> Vec<PathBuf> {
        if !self.marks.is_empty() {
            return self.marks.iter().cloned().collect();
        }
        self.selected()
            .map_or_else(Vec::new, |e| self.under(&e.rel).cloned().collect())
    }

    pub fn act(&mut self, op: Op, tx: &Sender<Msg>) {
        let rels = self.targets();
        match rels.len() {
            0 => {}
            // Publishing many repos at once deserves a second look.
            n if op == Op::Push && n > 1 => self.confirm = Some((op, rels)),
            _ => self.launch(op, rels, tx),
        }
    }

    pub fn launch(&mut self, op: Op, rels: Vec<PathBuf>, tx: &Sender<Msg>) {
        for r in &rels {
            self.infos.remove(r);
            if let Some(Some(s)) = self.repos.get_mut(r) {
                s.stale = true;
            }
            if op != Op::Status {
                self.failures.remove(r);
                self.busy.insert(r.clone());
            }
        }
        if op != Op::Status {
            self.batch = Some(Batch {
                op,
                total: rels.len(),
                ok: 0,
                failed: 0,
                last_err: None,
            });
        }
        scan::spawn_ops(self.root.clone(), rels, op, tx.clone());
    }

    /// Folder rows toggle their whole subtree.
    pub fn toggle_mark(&mut self) {
        let Some(e) = self.selected() else { return };
        let rels: Vec<PathBuf> = self.under(&e.rel).cloned().collect();
        if rels.iter().all(|r| self.marks.contains(r)) {
            rels.iter().for_each(|r| _ = self.marks.remove(r));
        } else {
            self.marks.extend(rels);
        }
    }

    pub fn mark_attention(&mut self) {
        let hits: Vec<PathBuf> = self
            .under(&self.cwd)
            .filter(|r| {
                self.repos[*r]
                    .as_ref()
                    .is_some_and(|s| s.tone() < Tone::Clean)
            })
            .cloned()
            .collect();
        self.marks.extend(hits);
    }
}
