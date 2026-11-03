use crate::{
    porcelain::Status,
    scan::{self, Msg},
};
use ratatui::widgets::ListState;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

pub struct Entry {
    pub name: String,
    pub rel: PathBuf,
    pub total: usize,
    pub drift: usize,
    pub pending: usize,
    pub repo: bool,
}

pub struct App {
    pub root: PathBuf,
    /// Relative repo path -> status (None = in flight). Path ordering keeps subtrees contiguous.
    pub repos: BTreeMap<PathBuf, Option<Status>>,
    pub scanning: bool,
    pub only_drift: bool,
    pub cwd: PathBuf,
    pub list: ListState,
}

impl App {
    pub fn new(root: PathBuf) -> Self {
        let (repos, scanning, only_drift, cwd) = (BTreeMap::new(), true, false, PathBuf::new());
        Self {
            root,
            repos,
            scanning,
            only_drift,
            cwd,
            list: ListState::default().with_selected(Some(0)),
        }
    }

    pub fn under<'a>(
        &'a self,
        dir: &'a Path,
    ) -> impl Iterator<Item = (&'a PathBuf, &'a Option<Status>)> {
        self.repos
            .range(dir.to_path_buf()..)
            .take_while(move |(p, _)| p.starts_with(dir))
    }

    pub fn entries(&self) -> Vec<Entry> {
        let mut out: Vec<Entry> = Vec::new();
        for (rel, st) in self.under(&self.cwd) {
            let rest = rel.strip_prefix(&self.cwd).unwrap();
            let head = rest
                .components()
                .next()
                .map_or(Path::new(""), |c| Path::new(c.as_os_str()));
            let name = if rest.as_os_str().is_empty() {
                ".".into()
            } else {
                head.to_string_lossy().into_owned()
            };
            if out.last().is_none_or(|e| e.name != name) {
                let repo = rest.components().count() <= 1;
                out.push(Entry {
                    name,
                    rel: self.cwd.join(head),
                    total: 0,
                    drift: 0,
                    pending: 0,
                    repo,
                });
            }
            let e = out.last_mut().unwrap();
            e.total += 1;
            match st {
                None => e.pending += 1,
                Some(s) if s.drift() => e.drift += 1,
                _ => {}
            }
        }
        if self.only_drift {
            out.retain(|e| e.drift + e.pending > 0);
        }
        out
    }

    pub fn selected(&self) -> Option<Entry> {
        self.entries().into_iter().nth(self.list.selected()?)
    }

    pub fn step(&mut self, d: isize) {
        let n = self.entries().len();
        let i = self.list.selected().unwrap_or(0) as isize + d;
        self.list
            .select(Some(i.clamp(0, n.saturating_sub(1) as isize) as usize));
    }

    /// Drills into a folder; a repo yields its absolute path instead.
    pub fn enter(&mut self) -> Option<PathBuf> {
        let e = self.selected()?;
        if e.repo {
            return Some(self.abs(&e.rel));
        }
        self.cwd = e.rel;
        self.list.select(Some(0));
        None
    }

    /// `root.join("")` would append a separator.
    pub fn abs(&self, rel: &Path) -> PathBuf {
        if rel.as_os_str().is_empty() {
            self.root.clone()
        } else {
            self.root.join(rel)
        }
    }

    pub fn up(&mut self) {
        let Some(parent) = self.cwd.parent().map(Path::to_path_buf) else {
            return;
        };
        let from = std::mem::replace(&mut self.cwd, parent);
        self.list.select(Some(
            self.entries()
                .iter()
                .position(|e| e.rel == from)
                .unwrap_or(0),
        ));
    }

    pub fn refresh(&mut self, fetch: bool, tx: &Sender<Msg>) {
        let rels: Vec<PathBuf> = self.under(&self.cwd).map(|(p, _)| p.clone()).collect();
        rels.iter()
            .for_each(|p| _ = self.repos.insert(p.clone(), None));
        scan::spawn_status(self.root.clone(), rels, fetch, tx.clone());
    }
}
