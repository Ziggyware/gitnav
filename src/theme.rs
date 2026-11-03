//! Palette and glyph primitives: tones are the only semantic colours.
use crate::porcelain::Tone;
use ratatui::{
    style::{Color, Style, Stylize},
    text::Span,
};

pub const BG: Color = Color::Rgb(22, 24, 35);
pub const ACCENT: Color = Color::Rgb(130, 170, 255);
pub const TEXT: Color = Color::Rgb(205, 214, 244);
pub const DIM: Color = Color::Rgb(108, 115, 150);
pub const COLD: Color = Color::Rgb(52, 58, 82);
pub const SEL: Color = Color::Rgb(40, 45, 68);

pub fn tone(t: Tone) -> Color {
    let (r, g, b) = match t {
        Tone::Error => (255, 83, 112),
        Tone::Behind => (199, 146, 234),
        Tone::Dirty => (255, 203, 107),
        Tone::Ahead => (137, 221, 255),
        Tone::Unpushed => (247, 140, 108),
        Tone::Clean => (195, 232, 141),
        Tone::Pending => (108, 115, 150),
    };
    Color::Rgb(r, g, b)
}

pub fn glyph(t: Tone) -> &'static str {
    // Native to common monospace faces and centred on the digit line: no fallback-font drift.
    ["×", "↓", "±", "↑", "⌂", "=", "·"][t as usize]
}

pub fn spin(tick: u64) -> String {
    ["|", "/", "-", "\\"][tick as usize % 4].to_string()
}

pub fn lerp(a: Color, b: Color, t: f32) -> Color {
    let (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) = (a, b) else {
        return a;
    };
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t.clamp(0.0, 1.0)) as u8;
    Color::Rgb(m(r1, r2), m(g1, g2), m(b1, b2))
}

pub fn age(now: i64, t: i64) -> String {
    if t == 0 {
        return "—".into();
    }
    match (now - t).max(0) {
        s @ ..3_600 => format!("{}m", s / 60),
        s @ ..86_400 => format!("{}h", s / 3_600),
        s @ ..1_209_600 => format!("{}d", s / 86_400),
        s @ ..63_072_000 => format!("{}w", s / 604_800),
        s => format!("{}y", s / 31_536_000),
    }
}

/// Recent activity glows, abandoned repos sink into the background.
pub fn heat(now: i64, t: i64) -> Color {
    lerp(ACCENT, COLD, (now - t) as f32 / (120.0 * 86_400.0))
}

/// Proportional stacked bar; every non-empty tone keeps at least one cell.
pub fn bar(counts: &[usize; 7], width: usize) -> Vec<Span<'static>> {
    let total = counts.iter().sum::<usize>();
    if total == 0 {
        return vec!["━".repeat(width).fg(COLD)];
    }
    let mut cells: Vec<usize> = counts
        .iter()
        .map(|&c| if c == 0 { 0 } else { (c * width / total).max(1) })
        .collect();
    let biggest = |v: &[usize]| (0..7).max_by_key(|&i| v[i]).unwrap();
    while cells.iter().sum::<usize>() > width {
        let i = biggest(&cells);
        cells[i] -= 1;
    }
    while cells.iter().sum::<usize>() < width {
        cells[biggest(counts)] += 1;
    }
    Tone::ALL
        .iter()
        .zip(cells)
        .filter(|(_, n)| *n > 0)
        .map(|(&t, n)| "━".repeat(n).fg(tone(t)))
        .collect()
}

pub fn key(k: &'static str, label: &'static str) -> [Span<'static>; 2] {
    [
        Span::styled(format!(" {k}"), Style::new().fg(ACCENT).bold()),
        format!(" {label} ").fg(DIM),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_and_age() {
        let w = |c: [usize; 7]| {
            bar(&c, 10)
                .iter()
                .map(|s| s.content.chars().count())
                .collect::<Vec<_>>()
        };
        assert_eq!(w([1, 0, 0, 0, 0, 299, 0]), [1, 9]);
        assert_eq!(w([0, 0, 5, 0, 0, 5, 0]), [5, 5]);
        assert_eq!(
            (age(100_000, 100_000 - 90), age(10_000_000, 1)),
            ("1m".into(), "16w".into())
        );
    }
}
