use crate::{
<<<<<<< HEAD
    cache::{self, Pulses, Repos},
    fuzzy::fuzzy,
    history::{Info, WEEKS},
    porcelain::{Tone, tone},
    scan::Op,
};
use ratatui::widgets::TableState;
use std::{
    cmp::Reverse,
    collections::{BTreeSet, HashMap, HashSet},
    path::{Path, PathBuf},
=======
    porcelain::Status,
    scan::{self, Msg},
};
use ratatui::widgets::ListState;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
>>>>>>> ce532fa2a485cac25364c698a4d1c4994bba308d
};

pub struct Entry {
    pub name: String,
    pub rel: PathBuf,
<<<<<<< HEAD
    pub repo: bool,
    pub total: usize,
    pub tones: [usize; 7],
    pub weeks: [u32; WEEKS],
    pub last: i64,
    score: i32,
}

impl Entry {
    fn new(name: String, rel: PathBuf, repo: bool) -> Self {
        Self {
            name,
            rel,
            repo,
            total: 0,
            tones: [0; 7],
            weeks: [0; WEEKS],
            last: 0,
            score: 0,
        }
    }
    pub fn attention(&self) -> usize {
        self.total - self.tones[Tone::Clean as usize] - self.tones[Tone::Pending as usize]
    }
    pub fn worst(&self) -> Tone {
        Tone::ALL
            .into_iter()
            .find(|&t| self.tones[t as usize] > 0)
            .unwrap_or(Tone::Clean)
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum Sort {
    #[default]
    Name,
    Attention,
    Recent,
}

#[derive(Default)]
pub struct App {
    pub root: PathBuf,
    /// Relative repo path -> status (None = never checked). Path order keeps subtrees contiguous.
    pub repos: Repos,
    pub pulses: Pulses,
    /// Lazy per-repo detail; None = requested.
    pub infos: HashMap<PathBuf, Option<Info>>,
    pub failures: HashMap<PathBuf, String>,
    /// Repos the running walk confirmed; cached ones outside it are pruned when it ends.
    pub seen: HashSet<PathBuf>,
    pub scanning: bool,
    pub cwd: PathBuf,
    pub table: TableState,
    pub only_drift: bool,
    pub sort: Sort,
    pub marks: BTreeSet<PathBuf>,
    pub busy: HashSet<PathBuf>,
    pub search: Option<String>,
    pub confirm: Option<(Op, Vec<PathBuf>)>,
    pub batch: Option<crate::events::Batch>,
    pub help: bool,
    pub tick: u64,
    pub now: i64,
=======
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
>>>>>>> ce532fa2a485cac25364c698a4d1c4994bba308d
}

impl App {
    pub fn new(root: PathBuf) -> Self {
<<<<<<< HEAD
        let (repos, pulses) = cache::load(&root);
        Self {
            repos,
            pulses,
            root,
            scanning: true,
            ..Default::default()
        }
    }

    pub fn under<'a>(&'a self, dir: &'a Path) -> impl Iterator<Item = &'a PathBuf> {
        self.repos
            .range(dir.to_path_buf()..)
            .map(|(p, _)| p)
            .take_while(move |p| p.starts_with(dir))
    }

    fn add(&self, e: &mut Entry, rel: &Path) {
        e.total += 1;
        e.tones[tone(self.repos.get(rel).and_then(Option::as_ref)) as usize] += 1;
        if let Some(p) = self.pulses.get(rel) {
            e.weeks.iter_mut().zip(p.weeks).for_each(|(a, b)| *a += b);
            e.last = e.last.max(p.last);
        }
    }

    fn tree(&self) -> Vec<Entry> {
        let mut out: Vec<Entry> = Vec::new();
        for rel in self.under(&self.cwd) {
=======
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
>>>>>>> ce532fa2a485cac25364c698a4d1c4994bba308d
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
<<<<<<< HEAD
                out.push(Entry::new(
                    name,
                    self.cwd.join(head),
                    rest.components().count() <= 1,
                ));
            }
            self.add(out.last_mut().unwrap(), rel);
        }
        out
    }

    fn matches(&self, q: &str) -> Vec<Entry> {
        let mut out = Vec::new();
        for rel in self.repos.keys() {
            let name = rel.to_string_lossy().into_owned();
            let Some(score) = fuzzy(q, &name) else {
                continue;
            };
            let mut e = Entry::new(name, rel.clone(), true);
            e.score = score;
            self.add(&mut e, rel);
            out.push(e);
        }
        out
    }

    pub fn entries(&self) -> Vec<Entry> {
        let mut out = self
            .search
            .as_deref()
            .map_or_else(|| self.tree(), |q| self.matches(q));
        if self.only_drift {
            out.retain(|e| e.attention() + e.tones[Tone::Pending as usize] > 0);
        }
        // Stable sorts: ties keep path order.
        match (&self.search, self.sort) {
            (Some(_), _) => out.sort_by_key(|e| Reverse(e.score)),
            (_, Sort::Attention) => out.sort_by_key(|e| (e.worst(), Reverse(e.attention()))),
            (_, Sort::Recent) => out.sort_by_key(|e| Reverse(e.last)),
            (_, Sort::Name) => {}
=======
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
>>>>>>> ce532fa2a485cac25364c698a4d1c4994bba308d
        }
        out
    }

    pub fn selected(&self) -> Option<Entry> {
<<<<<<< HEAD
        self.entries().into_iter().nth(self.table.selected()?)
=======
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
>>>>>>> ce532fa2a485cac25364c698a4d1c4994bba308d
    }
}
