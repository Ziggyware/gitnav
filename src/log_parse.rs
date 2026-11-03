//! Parsers for git's machine-readable log and diff output.
use crate::history::{Commit, DAY, DAYS, KEEP, Pulse, top, weekday};

/// Enough to fill a tall feed for a single repo.
const RECENT: usize = 40;
use std::{collections::HashMap, f32::consts::TAU};

fn offset(z: &str) -> Option<i64> {
    let (sign, hm) = z.split_at_checked(1)?;
    let (h, m): (i64, i64) = (hm.get(..2)?.parse().ok()?, hm.get(2..4)?.parse().ok()?);
    Some(if sign == "-" { -1 } else { 1 } * (h * 3_600 + m * 60))
}

/// `git log --numstat --date=format-local:%z --format=@%ct %ad%x1f%an%x1f%s`, newest first.
pub fn parse_pulse(log: &str, now: i64) -> Pulse {
    let mut rows: Vec<(i64, i64, Commit)> = Vec::new();
    let mut hot: HashMap<String, u32> = HashMap::new();
    for l in log.lines() {
        if let Some(h) = l.strip_prefix('@') {
            let mut f = h.splitn(3, '\x1f');
            let (Some((t, z)), Some(a), s) = (f.next().and_then(|x| x.split_once(' ')), f.next(), f.next()) else {
                continue;
            };
            let (Ok(t), Some(off)) = (t.parse(), offset(z)) else {
                continue;
            };
            rows.push((
                t,
                off,
                Commit {
                    t,
                    author: a.into(),
                    subject: s.unwrap_or("").into(),
                    ..Default::default()
                },
            ));
        } else if let Some(row) = rows.last_mut() {
            let f: Vec<&str> = l.splitn(3, '\t').collect();
            let &[a, d, path] = &f[..] else { continue };
            let n = a.parse().unwrap_or(0) + d.parse::<u32>().unwrap_or(0);
            row.2.churn += n;
            *hot.entry(path.to_owned()).or_default() += n;
        }
    }
    // shortcut: today's offset is taken from the newest commit; off by an hour across a DST change.
    let today = (now + rows.first().map_or(0, |r| r.1)).div_euclid(DAY);
    let mut authors: HashMap<String, u32> = HashMap::new();
    rows.iter()
        .for_each(|r| *authors.entry(r.2.author.clone()).or_default() += 1);
    let authors = top(authors, usize::MAX);
    let owner = authors.first().map(|a| a.0.clone()).unwrap_or_default();
    let mut p = Pulse {
        today,
        authors,
        hot: top(hot, KEEP),
        ..Default::default()
    };
    for (t, off, c) in &rows {
        p.last = p.last.max(*t);
        let local = t + off;
        let (age, h) = (today - local.div_euclid(DAY), local.rem_euclid(DAY) as f32 / 3_600.0);
        if (0..DAYS as i64).contains(&age) {
            let (d, w) = (DAYS - 1 - age as usize, (DAYS - 1 - age as usize) / 7);
            p.days[d] += 1;
            p.churn[w] += c.churn;
            p.others[w] += (c.author != owner) as u16;
            p.late[w] += (!(6.0..20.0).contains(&h) || weekday(local.div_euclid(DAY)) >= 5) as u16;
        }
        p.hours[h as usize] += 1;
        let a = h / 24.0 * TAU;
        p.circ = (p.circ.0 + a.cos(), p.circ.1 + a.sin());
    }
    p.burst = crate::stats::burstiness(rows.iter().map(|r| r.0));
    p.recent = rows.into_iter().take(RECENT).map(|r| r.2).collect();
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
