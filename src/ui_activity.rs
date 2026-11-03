//! Circadian rhythm and the statistics that interpret commit activity.
use crate::{
    history::{DAYS, Pulse, weekday},
    stats,
    theme::*,
};
use ratatui::prelude::*;

pub fn rhythm(p: &Pulse, width: u16) -> Vec<Line<'static>> {
    let n = p.hours.iter().map(|&h| h as u32).sum::<u32>();
    let Some((peak, r)) = stats::circular(p.circ.0, p.circ.1, n) else {
        return vec![Line::from("rhythm · too few commits to read a daily pattern".fg(DIM))];
    };
    let shape = match r {
        r if r >= 0.6 => "tight schedule",
        r if r >= 0.3 => "loose schedule",
        _ => "round the clock",
    };
    let (h, m) = (peak as u32, ((peak.fract()) * 60.0) as u32);
    let title = format!("rhythm · peak {h:02}:{m:02} · R {r:.2} {shape}");
    let cw = if width >= 52 { 2 } else { 1 };
    let max = *p.hours.iter().max().unwrap_or(&1) as f32;
    const BARS: [&str; 9] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    let bars = p.hours.iter().enumerate().map(|(i, &c)| {
        let b = if c == 0 {
            "▁"
        } else {
            BARS[(1.0 + 7.0 * c as f32 / max).round().min(8.0) as usize]
        };
        let near = ((i as f32 + 0.5 - peak).rem_euclid(24.0)).min((peak - i as f32 - 0.5).rem_euclid(24.0)) < 1.0;
        b.repeat(cw).fg(if c == 0 {
            COLD
        } else if near {
            tone(crate::porcelain::Tone::Clean)
        } else {
            ACCENT
        })
    });
    let axis: String = (0..24).step_by(6).map(|h| format!("{:<w$}", h, w = 6 * cw)).collect();
    vec![
        Line::from(title.fg(DIM)),
        Line::from(bars.collect::<Vec<_>>()),
        Line::from(axis.fg(DIM)),
    ]
}

fn pval(p: f64) -> String {
    if p < 0.001 {
        "p<0.001".into()
    } else {
        format!("p={p:.3}")
    }
}

/// Pace, trend significance and how surprising the current silence is.
pub fn metrics(p: &Pulse, now: i64) -> Vec<Line<'static>> {
    let w = p.weeks();
    let (recent, base): (u32, u32) = (w[12..].iter().sum(), w[..12].iter().sum());
    let pace = stats::rate(&w, 4.0);
    let trend = match stats::trend(recent, base, 4.0, 12.0) {
        None => "too few commits to test for a trend".to_owned(),
        Some(_) if base == 0 => "all of it in the last 4 weeks: new activity".to_owned(),
        Some(t) if t.p < 0.05 => {
            let dir = if t.ratio > 1.0 { "speeding up" } else { "slowing down" };
            format!("×{:.1} vs prior 12w, {}: {dir}", t.ratio, pval(t.p))
        }
        Some(t) => format!("steady: no significant change ({})", pval(t.p)),
    };
    let gap = (now - p.last).max(0) as f64 / 86_400.0;
    let quiet = match p.total() {
        _ if p.last == 0 => "no commits yet".to_owned(),
        0 => format!("silent {}: dormant", age(now, p.last)),
        _ if gap < 1.0 => "committed today".to_owned(),
        n => {
            let pr = stats::quiet(n, DAYS as f64, gap);
            let verdict = if pr < 0.01 {
                "unusually long: likely paused"
            } else if pr < 0.05 {
                "longer than usual"
            } else {
                "a normal gap"
            };
            format!(
                "silent {} · P={:.1}% at its usual rate: {verdict}",
                age(now, p.last),
                pr * 100.0
            )
        }
    };
    let weekend: u32 = (0..DAYS)
        .filter(|&i| weekday(p.today - (DAYS - 1 - i) as i64) >= 5)
        .map(|i| p.days[i] as u32)
        .sum();
    let share = 100.0 * weekend as f64 / p.total().max(1) as f64;
    let row = |k: &'static str, v: String| Line::from(vec![format!("{k:<8}").fg(ACCENT), v.fg(TEXT)]);
    let mut v = vec![
        row("pace", format!("{pace:.1} commits/wk recently · {trend}")),
        row("quiet", quiet),
    ];
    if let Some(b) = p.burst {
        let kind = if b < -0.3 {
            "regular, metronomic"
        } else if b <= 0.3 {
            "random, Poisson-like"
        } else {
            "bursty: works in sprints"
        };
        v.push(row("cadence", format!("B={b:+.2} {kind}")));
    }
    v.push(row("weekend", format!("{share:.0}% of commits on Sat/Sun")));
    v
}
