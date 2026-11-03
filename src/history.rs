//! Commit history and lazy per-repo detail.
use std::time::{SystemTime, UNIX_EPOCH};

pub const WEEKS: usize = 16;
const WEEK: i64 = 7 * 86_400;

/// Commit counts per week, oldest first; `weeks[WEEKS - 1]` is the current week.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pulse {
    pub last: i64,
    pub weeks: [u32; WEEKS],
}

#[derive(Debug, Default)]
pub struct Info {
    pub remote: String,
    pub subject: String,
    pub author: String,
    pub when: i64,
    pub stat: Vec<(String, u32, u32)>,
}

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

pub fn parse_pulse(log: &str, now: i64) -> Pulse {
    let mut p = Pulse::default();
    for t in log.lines().filter_map(|l| l.trim().parse::<i64>().ok()) {
        p.last = p.last.max(t);
        let age = ((now - t).max(0) / WEEK) as usize;
        if age < WEEKS {
            p.weeks[WEEKS - 1 - age] += 1;
        }
    }
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
    fn buckets_pulse_and_numstat() {
        let now = 100 * WEEK;
        let p = parse_pulse(
            &format!("{}\n{}\n{}\n", now - 10, now - WEEK - 1, now - 40 * WEEK),
            now,
        );
        assert_eq!(
            (
                p.weeks[WEEKS - 1],
                p.weeks[WEEKS - 2],
                p.weeks.iter().sum::<u32>()
            ),
            (1, 1, 2)
        );
        assert_eq!(p.last, now - 10);
        assert_eq!(
            parse_numstat("3\t1\ta b.rs\n-\t-\timg.png\n"),
            [("a b.rs".into(), 3, 1), ("img.png".into(), 0, 0)]
        );
    }
}
