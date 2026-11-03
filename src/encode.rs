//! Information-carrying visual encodings: every colour channel and bar length means something.
use crate::{
    history::{Pulse, WEEKS},
    porcelain::Tone,
    theme::{COLD, lerp, tone},
};
use ratatui::{
    style::{Color, Stylize},
    text::Span,
};

/// Bar height: commits per week, log-scaled to `max` so one hyperactive repo can't flatten the rest.
/// Colour is additive RGB, one measure per channel: R = lines per commit (full at 1000),
/// G = share by people other than the main author, B = share off-hours. Grey = small solo daytime work.
pub fn spark(p: &Pulse, max: u32) -> Vec<Span<'static>> {
    const BARS: [&str; 9] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    let (w, scale) = (p.weeks(), (max as f32 + 1.0).ln());
    (0..WEEKS)
        .map(|i| match w[i] {
            0 => "▁".fg(COLD),
            n => BARS[(1.0 + 7.0 * (n as f32 + 1.0).ln() / scale).round() as usize].fg(rgb(p, i, n)),
        })
        .collect()
}

pub fn rgb(p: &Pulse, week: usize, n: u32) -> Color {
    let ch = |x: f32| (70.0 + 185.0 * x.clamp(0.0, 1.0)) as u8;
    let r = (p.churn[week] as f32 / n as f32 + 1.0).ln() / 1001f32.ln();
    Color::Rgb(
        ch(r),
        ch(p.others[week] as f32 / n as f32),
        ch(p.late[week] as f32 / n as f32),
    )
}

/// Ten mutually distinct hues; authors take them in order of commits across the whole root.
const PEOPLE: [(u8, u8, u8); 10] = [
    (106, 168, 255),
    (255, 159, 67),
    (120, 224, 143),
    (255, 107, 129),
    (196, 140, 255),
    (255, 214, 102),
    (72, 219, 251),
    (255, 140, 200),
    (170, 200, 120),
    (230, 230, 230),
];

pub fn person(rank: Option<usize>) -> Color {
    let (r, g, b) = rank.map_or((140, 140, 160), |i| PEOPLE[i % PEOPLE.len()]);
    Color::Rgb(r, g, b)
}

/// Log-scaled churn as a run of cells, so 10 and 10 000 lines both read.
pub fn churn_bar(n: u32, cells: usize) -> Span<'static> {
    let k = ((n as f32 + 1.0).ln() / 10_001f32.ln() * cells as f32)
        .ceil()
        .min(cells as f32) as usize;
    "━".repeat(k).fg(lerp(
        Color::Rgb(120, 90, 90),
        tone(Tone::Error),
        k as f32 / cells as f32,
    ))
}

pub fn kilo(n: u32) -> String {
    match n {
        0..1_000 => n.to_string(),
        1_000..999_500 => format!("{:.1}k", n as f32 / 1e3),
        _ => format!("{:.1}M", n as f32 / 1e6),
    }
}
