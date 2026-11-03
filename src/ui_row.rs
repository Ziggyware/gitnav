use crate::{
    app::{App, Entry},
    encode::*,
    porcelain::{Status, Tone},
    stats,
    theme::{self, *},
    ui_list::Col,
};
use ratatui::{prelude::*, widgets::*};

/// Significant trend only: last 4 weeks vs the 12 before, exact test at p < 0.05.
fn trend(w: &[u32]) -> Span<'static> {
    let (recent, base) = (w[12..].iter().sum::<u32>(), w[..12].iter().sum::<u32>());
    match stats::trend(recent, base, 4.0, 12.0) {
        Some(_) if base == 0 => "new".fg(theme::tone(Tone::Clean)),
        Some(t) if t.p < 0.05 => {
            let pct = ((t.ratio - 1.0) * 100.0).round();
            let tone = if pct > 0.0 { Tone::Clean } else { Tone::Unpushed };
            let label = if t.ratio >= 10.0 {
                format!("×{:.0}", t.ratio)
            } else {
                format!("{pct:+.0}%")
            };
            label.fg(theme::tone(tone))
        }
        _ => "·".fg(COLD),
    }
}

impl App {
    pub(crate) fn row(&self, e: &Entry, max: u32, cols: &[(Col, &str, u16)]) -> Row<'static> {
        let st = self.repos.get(&e.rel).and_then(Option::as_ref);
        let marked = self
            .marks
            .range(e.rel.clone()..)
            .next()
            .is_some_and(|m| m.starts_with(&e.rel));
        let busy = self.busy.iter().any(|b| b.starts_with(&e.rel));
        let icon = match () {
            _ if busy || e.tones[Tone::Pending as usize] > 0 => spin(self.tick),
            _ if e.repo && st.is_some_and(|s| s.unborn) => "◊".into(),
            _ if e.repo => "•".into(),
            _ => "›".into(),
        };
        let p = &e.pulse;
        let w = p.weeks();
        let pace = stats::rate(&w, 4.0);
        let lines = p.churn[12..].iter().sum::<u32>() / 4;
        let ppl = p.authors.len();
        let cell = |c: Col| -> Line<'static> {
            match c {
                Col::Lead => Line::from(vec![
                    if marked { "▌".fg(ACCENT) } else { " ".into() },
                    icon.clone().fg(theme::tone(e.worst())),
                ]),
                Col::Name => match (e.repo, st.is_some_and(|s| s.stale)) {
                    (false, _) => format!("{}/", e.name).fg(TEXT).bold().into(),
                    (true, stale) => e.name.clone().fg(if stale { DIM } else { TEXT }).into(),
                },
                Col::Act => Line::from(spark(p, max)),
                Col::Pace => format!("{pace:>4.1}").fg(if pace < 0.05 { COLD } else { TEXT }).into(),
                Col::Trend => trend(&w).into(),
                Col::Lines => {
                    let x = (lines as f32 + 1.0).ln() / 5_001f32.ln();
                    format!("{:>5}", kilo(lines))
                        .fg(lerp(DIM, Color::Rgb(255, 90, 90), x))
                        .into()
                }
                Col::People => {
                    let c = if ppl <= 1 {
                        DIM
                    } else {
                        lerp(DIM, Color::Rgb(90, 255, 120), (ppl - 1) as f32 / 4.0)
                    };
                    format!("{ppl:>3}").fg(c).into()
                }
                Col::Last => theme::age(self.now, p.last).fg(heat(self.now, p.last)).into(),
                Col::State if e.repo => Line::from(st.map_or_else(Vec::new, badges)),
                Col::State => {
                    let mut v = bar(&e.tones, 8);
                    v.push(format!(" {:>3}", e.total).fg(DIM));
                    Line::from(v)
                }
            }
        };
        Row::new(cols.iter().map(|c| Cell::from(cell(c.0))))
    }
}

pub fn badges(s: &Status) -> Vec<Span<'static>> {
    let t = |x| theme::tone(x);
    let mut v = Vec::new();
    if s.error.is_some() {
        v.push(" ×".fg(t(Tone::Error)));
    }
    if s.unborn {
        v.push(format!(" ◊unborn({})", s.changes.len()).fg(t(Tone::Unborn)));
        return v;
    }
    if s.dirty() {
        v.push(format!(" ±{}", s.changes.len()).fg(t(Tone::Dirty)));
    }
    if s.ahead > 0 {
        v.push(format!(" ↑{}", s.ahead).fg(t(Tone::Ahead)));
    }
    if s.behind > 0 {
        v.push(format!(" ↓{}", s.behind).fg(t(Tone::Behind)));
    }
    if !s.has_remote {
        v.push(" ⌂local".fg(t(Tone::Unpushed)));
    } else if s.upstream.is_none() {
        v.push(" ⌂".fg(t(Tone::Unpushed)));
    }
    if s.branches.iter().filter(|b| !b.remote_only).count() > 1 {
        let n = s.branches.iter().filter(|b| !b.remote_only).count();
        v.push(format!(" ⎇{n}").fg(ACCENT));
    }
    if v.is_empty() {
        v.push(" =".fg(t(Tone::Clean)));
    }
    if s.stale {
        v = v.into_iter().map(Stylize::dim).collect();
    }
    v
}
