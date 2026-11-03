//! Commit history over a rolling window, in the viewer's local time.
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

pub const DAYS: usize = 112;
pub const WEEKS: usize = DAYS / 7;
pub const DAY: i64 = 86_400;
pub const KEEP: usize = 12;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Commit {
    pub t: i64,
    pub author: String,
    pub subject: String,
    pub churn: u32,
    /// Filled when commits from several repos are merged into one feed.
    pub repo: String,
}

/// Per-week series are oldest first; `days[DAYS - 1]` is today.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pulse {
    pub last: i64,
    pub today: i64,
    pub days: Vec<u16>,
    pub hours: Vec<u16>,
    /// Σcos, Σsin of time-of-day angles: summable across repos.
    pub circ: (f32, f32),
    /// Goh–Barabási burstiness of gaps; None below 10 gaps.
    pub burst: Option<f32>,
    /// Lines added + deleted per week.
    pub churn: Vec<u32>,
    /// Commits per week not by the repo's most frequent author.
    pub others: Vec<u16>,
    /// Commits per week at 20:00–06:00 local or on weekends.
    pub late: Vec<u16>,
    /// Commit counts, largest first.
    pub authors: Vec<(String, u32)>,
    /// Files by churn, largest first.
    pub hot: Vec<(String, u32)>,
    pub recent: Vec<Commit>,
}

impl Default for Pulse {
    fn default() -> Self {
        let (z16, z32) = (vec![0u16; WEEKS], vec![0u32; WEEKS]);
        Self {
            last: 0,
            today: 0,
            days: vec![0; DAYS],
            hours: vec![0; 24],
            circ: (0.0, 0.0),
            burst: None,
            churn: z32,
            others: z16.clone(),
            late: z16,
            authors: vec![],
            hot: vec![],
            recent: vec![],
        }
    }
}

pub fn top(m: HashMap<String, u32>, n: usize) -> Vec<(String, u32)> {
    let mut v: Vec<_> = m.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v.truncate(n);
    v
}

fn sum<T: Copy + std::ops::Add<Output = T>>(a: &mut [T], b: &[T]) {
    a.iter_mut().zip(b).for_each(|(x, &y)| *x = *x + y);
}

impl Pulse {
    pub fn total(&self) -> u32 {
        self.days.iter().map(|&d| d as u32).sum()
    }
    pub fn weeks(&self) -> [u32; WEEKS] {
        std::array::from_fn(|w| self.days[w * 7..w * 7 + 7].iter().map(|&d| d as u32).sum())
    }
    /// Folder aggregate. Burstiness needs raw gaps, so it does not sum; paths gain the repo prefix.
    pub fn add(&mut self, o: &Pulse, repo: &str) {
        let pre = |s: &str| {
            if repo.is_empty() {
                s.to_owned()
            } else {
                format!("{repo}/{s}")
            }
        };
        self.days
            .iter_mut()
            .zip(&o.days)
            .for_each(|(a, &b)| *a = a.saturating_add(b));
        self.hours
            .iter_mut()
            .zip(&o.hours)
            .for_each(|(a, &b)| *a = a.saturating_add(b));
        sum(&mut self.churn, &o.churn);
        sum(&mut self.others, &o.others);
        sum(&mut self.late, &o.late);
        self.circ = (self.circ.0 + o.circ.0, self.circ.1 + o.circ.1);
        (self.last, self.today) = (self.last.max(o.last), self.today.max(o.today));
        let mut a: HashMap<String, u32> = self.authors.drain(..).collect();
        o.authors
            .iter()
            .for_each(|(k, n)| *a.entry(k.clone()).or_default() += n);
        self.authors = top(a, usize::MAX);
        let mut h: HashMap<String, u32> = self.hot.drain(..).collect();
        o.hot.iter().for_each(|(k, n)| *h.entry(pre(k)).or_default() += n);
        self.hot = top(h, KEEP);
        self.recent.extend(o.recent.iter().map(|c| Commit {
            repo: pre(&c.repo).trim_end_matches('/').to_owned(),
            ..c.clone()
        }));
        self.recent.sort_by_key(|c| std::cmp::Reverse(c.t));
        self.recent.truncate(64);
    }
}

/// Monday = 0, from a day number counted from the Unix epoch (a Thursday).
pub fn weekday(day: i64) -> usize {
    (day + 3).rem_euclid(7) as usize
}

#[derive(Debug, Default)]
pub struct Info {
    pub remote: String,
    pub subject: String,
    pub author: String,
    pub when: i64,
    pub stat: Vec<(String, u32, u32)>,
    /// (oldest, newest) mtime of uncommitted files; (0, 0) when clean.
    pub edits: (i64, i64),
}

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}
