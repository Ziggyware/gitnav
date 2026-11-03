use crate::{
    app::App,
    history::Info,
    porcelain::{Status, Tone},
    theme::{self, *},
};
use ratatui::prelude::*;

impl App {
    /// Uncommitted work: size, age, and per-file churn scaled to the largest file.
    pub(crate) fn change_lines(&self, s: &Status, info: Option<&Info>, w: u16) -> Vec<Line<'static>> {
        let t = |x| theme::tone(x);
        if s.changes.is_empty() {
            return vec![Line::default(), Line::from("working tree clean".fg(t(Tone::Clean)))];
        }
        let stat = info.map_or(&[][..], |i| &i.stat[..]);
        let (add, del) = stat.iter().fold((0, 0), |(x, y), (_, a, d)| (x + a, y + d));
        let sat = info.filter(|i| i.edits.0 > 0).map_or(String::new(), |i| {
            format!(" · oldest edit {}", theme::age(self.now, i.edits.0))
        });
        let head = format!("uncommitted · {} files · +{add} −{del}{sat}", s.changes.len());
        let mut v = vec![Line::default(), Line::from(head.fg(t(Tone::Dirty)))];
        let max = stat.iter().map(|(_, a, d)| a + d).max().unwrap_or(1).max(1) as usize;
        let span = (w as usize / 4).clamp(6, 16);
        let cells = |n: u32| (n as usize * span).div_ceil(max);
        let mut rows: Vec<Line> = stat
            .iter()
            .map(|(p, a, d)| {
                Line::from(vec![
                    format!("{:>5}", format!("+{a}")).fg(t(Tone::Clean)),
                    format!("{:>5} ", format!("−{d}")).fg(t(Tone::Error)),
                    "━".repeat(cells(*a)).fg(t(Tone::Clean)),
                    "━".repeat(cells(*d)).fg(t(Tone::Error)),
                    format!(" {p}").fg(TEXT),
                ])
            })
            .collect();
        // numstat omits untracked files and is empty until info arrives.
        for c in s
            .changes
            .iter()
            .filter(|c| stat.is_empty() || c.starts_with("??") || c.starts_with('U'))
        {
            let (xy, p) = c.split_at(2.min(c.len()));
            let (label, tone) = match xy {
                "??" => ("new", Tone::Ahead),
                x if x.contains('U') => ("conflict", Tone::Error),
                _ => ("changed", Tone::Dirty),
            };
            rows.push(Line::from(vec![
                format!("{label:>11} ").fg(t(tone)),
                p.trim().to_owned().fg(TEXT),
            ]));
        }
        let extra = rows.len().saturating_sub(6);
        v.extend(rows.into_iter().take(6));
        if extra > 0 {
            v.push(Line::from(format!("{:>11} {extra} more", "+").fg(DIM)));
        }
        v
    }
}
