//! Commit history and lazy per-repo detail.
use std::{
    f32::consts::TAU,
    time::{SystemTime, UNIX_EPOCH},
};

pub const DAYS: usize = 112;
pub const WEEKS: usize = DAYS / 7;
const DAY: i64 = 86_400;

/// Commit timing over the last `DAYS` days, in the viewer's local time.
#[derive(Clone, Debug, PartialEq)]
pub struct Pulse {
    pub last: i64,
    /// Local day number of `days[DAYS - 1]`; anchors weekday alignment.
    pub today: i64,
    /// Oldest first; the last slot is today.
    pub days: [u16; DAYS],
    pub hours: [u16; 24],
    /// Σcos, Σsin of each commit's time-of-day angle: summable across repos.
    pub circ: (f32, f32),
    /// Goh–Barabási burstiness of inter-commit gaps; None below 10 gaps.
    pub burst: Option<f32>,
}

impl Default for Pulse {
    fn default() -> Self {
        Self {
            last: 0,
            today: 0,
            days: [0; DAYS],
            hours: [0; 24],
            circ: (0.0, 0.0),
            burst: None,
        }
    }
}

impl Pulse {
    pub fn total(&self) -> u32 {
        self.days.iter().map(|&d| d as u32).sum()
    }
    pub fn weeks(&self) -> [u32; WEEKS] {
        std::array::from_fn(|w| self.days[w * 7..w * 7 + 7].iter().map(|&d| d as u32).sum())
    }
    /// Folder aggregate; burstiness needs raw gaps, so it does not sum.
    pub fn add(&mut self, o: &Pulse) {
        self.days
            .iter_mut()
            .zip(o.days)
            .for_each(|(a, b)| *a = a.saturating_add(b));
        self.hours
            .iter_mut()
            .zip(o.hours)
            .for_each(|(a, b)| *a = a.saturating_add(b));
        self.circ = (self.circ.0 + o.circ.0, self.circ.1 + o.circ.1);
        (self.last, self.today) = (self.last.max(o.last), self.today.max(o.today));
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

fn offset(z: &str) -> Option<i64> {
    let (sign, hm) = z.split_at_checked(1)?;
    let (h, m): (i64, i64) = (hm.get(..2)?.parse().ok()?, hm.get(2..4)?.parse().ok()?);
    Some(if sign == "-" { -1 } else { 1 } * (h * 3_600 + m * 60))
}

/// Lines of `%ct %ad` with `--date=format-local:%z`, newest first.
pub fn parse_pulse(log: &str, now: i64) -> Pulse {
    let rows: Vec<(i64, i64)> = log
        .lines()
        .filter_map(|l| {
            let (t, z) = l.trim().split_once(' ')?;
            Some((t.parse().ok()?, offset(z)?))
        })
        .collect();
    // shortcut: today's offset is taken from the newest commit; off by an hour across a DST change.
    let today = (now + rows.first().map_or(0, |r| r.1)).div_euclid(DAY);
    let mut p = Pulse {
        today,
        ..Default::default()
    };
    for &(t, off) in &rows {
        p.last = p.last.max(t);
        let local = t + off;
        let age = today - local.div_euclid(DAY);
        if (0..DAYS as i64).contains(&age) {
            p.days[DAYS - 1 - age as usize] += 1;
        }
        let h = local.rem_euclid(DAY) as f32 / 3_600.0;
        p.hours[h as usize] += 1;
        let a = h / 24.0 * TAU;
        p.circ = (p.circ.0 + a.cos(), p.circ.1 + a.sin());
    }
    p.burst = crate::stats::burstiness(rows.iter().map(|r| r.0));
    p
}

/// `git diff --numstat`; binary files report "-" and count as zero.
pub fn parse_numstat(s: &str) -> Vec<(String, u32, u32)> {
    s.lines()
        .filter_map(|l| {
            let mut f = l.splitn(3, '\t');
            let (a, d, p) = (f.next()?, f.next()?, f.next()?);
            Some((p.to_owned(), a.parse().unwrap_or(0), d.parse().unwrap_or(0)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_local_days_and_hours() {
        // 2023-11-14 22:13:20 UTC = 16:13:20 at -0600; local day 19675, a Tuesday.
        let t = 1_700_000_000;
        let p = parse_pulse(&format!("{t} -0600\n{} -0600\n", t - 3 * DAY), t + 60);
        assert_eq!((p.today, weekday(p.today)), (19_675, 1));
        assert_eq!((p.days[DAYS - 1], p.days[DAYS - 4], p.total()), (1, 1, 2));
        assert_eq!((p.hours[16], p.last), (2, t));
        assert_eq!(p.weeks()[WEEKS - 1], 2);
        assert_eq!(offset("+0530"), Some(19_800));
        assert_eq!(
            parse_numstat("3\t1\ta b.rs\n-\t-\timg.png\n"),
            [("a b.rs".into(), 3, 1), ("img.png".into(), 0, 0)]
        );
    }
}
