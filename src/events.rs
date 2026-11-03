//! Background results in, user intents out.
use crate::{
    app::{App, Modal},
    porcelain::Tone,
    scan::{self, Msg, Op},
};
use std::{fs, path::PathBuf, process::Command, sync::mpsc::Sender, thread};

/// Progress of one fetch/pull/push/commit/branch/adopt across its targets.
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
            Msg::Status(rel, s) => {
                if let Some(Modal::Branches {
                    rel: m_rel,
                    items,
                    sel,
                    ..
                }) = &mut self.modal
                {
                    if *m_rel == rel {
                        *items = s.branches.clone();
                        *sel = (*sel).min(items.len().saturating_sub(1));
                    }
                }
                self.repos.insert(rel, Some(s));
            }
            Msg::Pulse(rel, p) => {
                self.pulses.insert(rel, p);
                self.rerank();
            }
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
            Msg::Ai(e) => self.on_ai(e),
            Msg::Walked => {
                self.scanning = false;
                let seen = std::mem::take(&mut self.seen);
                self.repos.retain(|p, _| seen.contains(p));
                self.pulses.retain(|p, _| seen.contains(p));
            }
        }
    }

    pub fn want_info(&mut self, tx: &Sender<Msg>) {
        let Some(e) = self.selected().filter(|e| e.repo && !self.infos.contains_key(&e.rel)) else {
            return;
        };
        self.infos.insert(e.rel.clone(), None);
        let changes = self.repos[&e.rel].as_ref().map_or(&[][..], |s| &s.changes[..]);
        let paths: Vec<String> = changes
            .iter()
            .filter_map(|c| c.get(3..)?.split(" <- ").next().map(Into::into))
            .collect();
        let (dir, tx) = (self.abs(&e.rel), tx.clone());
        thread::spawn(move || _ = tx.send(Msg::Info(e.rel, crate::git::info(&dir, &paths))));
    }

    /// Marked repos, else everything under the selection.
    pub fn targets(&self) -> Vec<PathBuf> {
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
                op: op.clone(),
                total: rels.len(),
                ok: 0,
                failed: 0,
                last_err: None,
            });
        }
        scan::spawn_ops(self.root.clone(), rels, op, tx.clone());
    }

    pub fn open_commit_modal(&mut self) {
        let targets = self.targets();
        if !targets.is_empty() {
            self.modal = Some(Modal::Commit {
                targets,
                msg: String::new(),
            });
        }
    }

    pub fn open_adopt_modal(&mut self) {
        let targets = self.targets();
        if !targets.is_empty() {
            let default_spec = if targets.len() == 1 {
                targets[0]
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            } else {
                String::new()
            };
            self.modal = Some(Modal::Adopt {
                targets,
                spec: default_spec,
            });
        }
    }

    pub fn open_branch_modal(&mut self) {
        let Some(e) = self.selected() else { return };
        let rel = if e.repo {
            e.rel
        } else {
            let Some(first) = self.under(&e.rel).next().cloned() else {
                return;
            };
            first
        };
        let dir = self.abs(&rel);
        let items = crate::git::list_branches(&dir);
        let sel = items.iter().position(|b| b.head).unwrap_or(0);
        self.modal = Some(Modal::Branches {
            rel,
            items,
            sel,
            creating: None,
        });
    }

    pub fn open_new_project_modal(&mut self) {
        self.modal = Some(Modal::NewProject {
            cwd: self.cwd.clone(),
            spec: String::new(),
        });
    }

    /// Scaffolds a new project folder (`name`, `rs:name`, `ts:name`, `py:name`), initializes git, and registers it.
    pub fn scaffold_project(&mut self, cwd: PathBuf, spec: String, tx: &Sender<Msg>) {
        let spec = spec.trim();
        if spec.is_empty() {
            return;
        }
        let (kind, name) = spec
            .split_once(':')
            .map_or(("rs", spec), |(k, n)| (k.trim(), n.trim()));
        if name.is_empty() {
            return;
        }
        let rel = if cwd.as_os_str().is_empty() {
            PathBuf::from(name)
        } else {
            cwd.join(name)
        };
        let abs = self.abs(&rel);
        let _ = fs::create_dir_all(&abs);
        match kind {
            "rs" | "rust" => {
                if !abs.join("Cargo.toml").exists() {
                    let _ = Command::new("cargo").arg("init").arg(&abs).output();
                }
            }
            "ts" | "deno" => {
                let _ = fs::write(
                    abs.join("deno.json"),
                    "{\n  \"tasks\": {\n    \"dev\": \"deno run --watch main.ts\"\n  }\n}\n",
                );
                let _ = fs::write(abs.join("main.ts"), "console.log(\"ready\");\n");
            }
            "py" | "python" => {
                let _ = fs::write(
                    abs.join("pyproject.toml"),
                    format!("[project]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
                );
                let _ = fs::write(abs.join("main.py"), "if __name__ == \"__main__\":\n    pass\n");
            }
            _ => {
                let _ = fs::create_dir_all(abs.join("src"));
                let _ = fs::write(abs.join("README.md"), format!("# {name}\n"));
            }
        }
        self.repos.entry(rel.clone()).or_insert(None);
        self.launch(Op::Commit("feat: scaffold project".into()), vec![rel], tx);
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
            .filter(|r| self.repos[*r].as_ref().is_some_and(|s| s.tone() < Tone::Clean))
            .cloned()
            .collect();
        self.marks.extend(hits);
    }
}
