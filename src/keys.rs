use crate::{
    app::{App, Modal},
    nav::Sort,
    scan::{Msg, Op},
};
use ratatui::crossterm::event::{KeyCode::*, KeyEvent, KeyModifiers};
use std::{path::PathBuf, sync::mpsc::Sender};

pub enum Exit {
    Quit,
    Cd(PathBuf),
}

impl App {
    pub fn key(&mut self, k: KeyEvent, tx: &Sender<Msg>) -> Option<Exit> {
        if k.code == Char('c') && k.modifiers.contains(KeyModifiers::CONTROL) {
            return Some(Exit::Quit);
        }
        // An agent thread is blocked on this answer: nothing else may consume the key.
        if let Some((_, _, answer)) = self.chat.confirm.take() {
            let _ = answer.send(k.code == Char('y'));
            return None;
        }
        if self.chat.open && self.chat.focus {
            let c = &mut self.chat;
            match k.code {
                Esc if c.key_entry => (c.key_entry, c.input) = (false, String::new()),
                Esc => (c.open, c.focus) = (false, false),
                Enter => self.ask(tx),
                Backspace => _ = c.input.pop(),
                Up | PageUp => c.scroll += if k.code == Up { 1 } else { 10 },
                Down | PageDown => c.scroll = c.scroll.saturating_sub(if k.code == Down { 1 } else { 10 }),
                Tab => c.focus = false,
                Char(ch) => c.input.push(ch),
                _ => {}
            }
            return None;
        }
        if let Some(modal) = self.modal.take() {
            self.handle_modal_key(modal, k, tx);
            return None;
        }
        if std::mem::take(&mut self.help) {
            return None;
        }
        if let Some((op, rels)) = self.confirm.take() {
            if k.code == Char('y') {
                self.launch(op, rels, tx);
            }
            return None;
        }
        if let Some(q) = &mut self.search {
            match k.code {
                Esc => self.search = None,
                Enter => _ = self.enter(),
                Up => self.step(-1),
                Down => self.step(1),
                Backspace => _ = q.pop(),
                Char(c) => q.push(c),
                _ => return None,
            }
            if !matches!(k.code, Up | Down) {
                self.table.select(Some(0));
            }
            return None;
        }
        match k.code {
            Char('q') => return Some(Exit::Quit),
            Esc if self.marks.is_empty() => return Some(Exit::Quit),
            Esc => self.marks.clear(),
            Char('j') | Down => self.step(1),
            Char('k') | Up => self.step(-1),
            PageDown => self.step(10),
            PageUp => self.step(-10),
            Char('g') | Home => self.step(isize::MIN / 2),
            Char('G') if k.modifiers.contains(KeyModifiers::SHIFT) => self.open_adopt_modal(),
            End => self.step(isize::MAX / 2),
            Enter | Right | Char('l') => return self.enter().map(Exit::Cd),
            Backspace | Left | Char('h') => self.up(),
            Char('C') => return self.selected().map(|e| Exit::Cd(self.abs(&e.rel))),
            Char('c') => self.open_commit_modal(),
            Char('b') => self.open_branch_modal(),
            Char('N') => self.open_new_project_modal(),
            Char('u') => {
                self.only_unuploaded ^= true;
                self.table.select(Some(0));
            }
            Char(' ') => {
                self.toggle_mark();
                self.step(1);
            }
            Char('a') => self.mark_attention(),
            Char('f') => self.act(Op::Fetch, tx),
            Char('p') => self.act(Op::Pull, tx),
            Char('P') => self.act(Op::Push, tx),
            Char('r') => {
                let rels = self
                    .repos
                    .keys()
                    .filter(|r| r.starts_with(&self.cwd))
                    .cloned()
                    .collect();
                self.launch(Op::Status, rels, tx);
            }
            Char('d') => {
                self.only_drift ^= true;
                self.table.select(Some(0));
            }
            Char('s') => {
                self.sort = match self.sort {
                    Sort::Name => Sort::Attention,
                    Sort::Attention => Sort::Recent,
                    Sort::Recent => Sort::Name,
                };
                self.table.select(Some(0));
            }
            Char('/') => {
                self.search = Some(String::new());
                self.table.select(Some(0));
            }
            Char('?') => self.help = true,
            Char('i') | Tab => self.open_chat(),
            _ => {}
        }
        None
    }

    fn handle_modal_key(&mut self, mut modal: Modal, k: KeyEvent, tx: &Sender<Msg>) {
        match &mut modal {
            Modal::Commit { targets, msg } => match k.code {
                Esc => return,
                Enter => {
                    let m = std::mem::take(msg);
                    let t = std::mem::take(targets);
                    self.launch(Op::Commit(m), t, tx);
                    return;
                }
                Backspace => _ = msg.pop(),
                Char(c) => msg.push(c),
                _ => {}
            },
            Modal::Adopt { targets, spec } => match k.code {
                Esc => return,
                Enter => {
                    let s = std::mem::take(spec);
                    let t = std::mem::take(targets);
                    self.launch(Op::Adopt(s), t, tx);
                    return;
                }
                Backspace => _ = spec.pop(),
                Char(c) => spec.push(c),
                _ => {}
            },
            Modal::NewProject { cwd, spec } => match k.code {
                Esc => return,
                Enter => {
                    let c = cwd.clone();
                    let s = std::mem::take(spec);
                    self.scaffold_project(c, s, tx);
                    return;
                }
                Backspace => _ = spec.pop(),
                Char(c) => spec.push(c),
                _ => {}
            },
            Modal::Branches {
                rel,
                items,
                sel,
                creating,
            } => {
                if let Some(name) = creating {
                    match k.code {
                        Esc => *creating = None,
                        Enter => {
                            let br = std::mem::take(name);
                            let r = rel.clone();
                            if !br.trim().is_empty() {
                                self.launch(Op::BranchSwitch(br), vec![r], tx);
                            }
                            return;
                        }
                        Backspace => _ = name.pop(),
                        Char(c) => name.push(c),
                        _ => {}
                    }
                } else {
                    match k.code {
                        Esc | Char('q') | Char('b') => return,
                        Char('j') | Down => {
                            if !items.is_empty() {
                                *sel = (*sel + 1).min(items.len() - 1);
                            }
                        }
                        Char('k') | Up => {
                            *sel = sel.saturating_sub(1);
                        }
                        Char('n') => *creating = Some(String::new()),
                        Enter => {
                            if let Some(b) = items.get(*sel) {
                                let name = b.name.clone();
                                let r = rel.clone();
                                self.launch(Op::BranchSwitch(name), vec![r], tx);
                                return;
                            }
                        }
                        Char('x') | Delete => {
                            if let Some(b) = items.get(*sel) {
                                if !b.head {
                                    let name = b.name.clone();
                                    let r = rel.clone();
                                    self.launch(Op::BranchDelete(name), vec![r], tx);
                                }
                            }
                        }
                        Char('m') => {
                            let r = rel.clone();
                            self.launch(Op::UnifyMain, vec![r], tx);
                            return;
                        }
                        Char('p') => {
                            let r = rel.clone();
                            self.launch(Op::Pull, vec![r], tx);
                        }
                        Char('P') => {
                            let r = rel.clone();
                            self.launch(Op::Push, vec![r], tx);
                        }
                        _ => {}
                    }
                }
            }
        }
        self.modal = Some(modal);
    }
}
