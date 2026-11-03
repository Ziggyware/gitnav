use crate::{
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
};

pub struct Entry {
    pub name: String,
    pub rel: PathBuf,
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
}

impl App {
    pub fn new(root: PathBuf) -> Self {
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
        }
        out
    }

    pub fn selected(&self) -> Option<Entry> {
        self.entries().into_iter().nth(self.table.selected()?)
    }
}
