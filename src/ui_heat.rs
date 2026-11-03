//! GitHub-style calendar: weeks across, weekdays down, two days per text row.
use crate::{
    history::{DAYS, Pulse},
    theme::*,
};
use ratatui::prelude::*;

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Month (1–12) of a day number since the Unix epoch (Hinnant's civil-from-days).
pub fn month(z: i64) -> usize {
    let z = z + 719_468;
    let doe = z - z.div_euclid(146_097) * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mp = (5 * (doe - (365 * yoe + yoe / 4 - yoe / 100)) + 2) / 153;
    (if mp < 10 { mp + 3 } else { mp - 9 }) as usize
}

fn level_color(l: usize) -> Color {
    match l {
        0 => lerp(COLD, BG, 0.35),
        l => lerp(Color::Rgb(46, 74, 128), Color::Rgb(186, 214, 255), (l - 1) as f32 / 3.0),
    }
}

/// Level 0 = no commits, 1–4 = quartile of active days, so one marathon day can't wash out the rest.
fn levels(p: &Pulse) -> impl Fn(u16) -> usize {
    let mut nz: Vec<u16> = p.days.iter().copied().filter(|&d| d > 0).collect();
    nz.sort_unstable();
    let q: Vec<u16> = [1, 2, 3]
        .map(|k| nz.get(nz.len() * k / 4).copied().unwrap_or(u16::MAX))
        .into();
    move |d| {
        if d == 0 {
            0
        } else {
            1 + q.iter().filter(|&&t| d >= t).count().min(3)
        }
    }
}

pub fn heatmap(p: &Pulse, width: u16) -> Vec<Line<'static>> {
    let first = p.today - (DAYS as i64 - 1);
    let week = |d: i64| (d + 3).div_euclid(7);
    let cols = (week(p.today) - week(first) + 1) as usize;
    // Each day is the top half of a cell plus a gap column: square pixels, gaps both ways.
    let cw = if width as usize >= 3 + cols * 2 { 2 } else { 1 };
    let level = levels(p);
    let color = |d: u16| level_color(level(d));
    let at = |col: usize, row: i64| {
        let d = (week(first) + col as i64) * 7 - 3 + row;
        (first..=p.today)
            .contains(&d)
            .then(|| color(p.days[(d - first) as usize]))
    };
    let mut caption = vec![format!("activity · {} commits / 16 weeks   less ", p.total()).fg(DIM)];
    caption.extend((0..5).flat_map(|l| ["▀".fg(level_color(l)), " ".into()]));
    caption.push("more".fg(DIM));
    let mut out = vec![Line::from(caption)];
    for (r, label) in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"].iter().enumerate() {
        let mut l = vec![format!("{label} ").fg(DIM)];
        for c in 0..cols {
            l.push(at(c, r as i64).map_or(" ".into(), |t| "▀".fg(t)));
            l.push(" ".repeat(cw - 1).into());
        }
        out.push(Line::from(l));
    }
    let mut axis = String::from("   ");
    for c in 0..cols {
        let m = month((week(first) + c as i64) * 7 - 3);
        if c == 0 || m != month((week(first) + c as i64 - 1) * 7 - 3) {
            axis.truncate(3 + c * cw);
            axis.push_str(MONTHS[m - 1]);
        }
        while axis.chars().count() < 3 + (c + 1) * cw {
            axis.push(' ');
        }
    }
    out.push(Line::from(axis.fg(DIM)));
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn civil_months() {
        // 2023-11-14, 2024-02-29, 1970-01-01
        assert_eq!(
            [super::month(19_675), super::month(19_782), super::month(0)],
            [11, 2, 1]
        );
    }
}
