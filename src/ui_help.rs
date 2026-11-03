use crate::{
    porcelain::Tone,
    theme::{self, *},
};
use ratatui::{layout::Constraint::*, prelude::*, widgets::*};

/// Keys, the tone legend, and how each statistic is computed.
pub fn help(f: &mut Frame, area: Rect) {
    let rows = [
        ("enter → l", "open folder · cd into repo"),
        ("← h bksp", "up a level"),
        ("c", "cd into selection"),
        ("space a esc", "mark · mark drifted · clear"),
        ("f p P", "fetch · pull --ff-only · push"),
        ("r", "re-check statuses here"),
        ("/", "fuzzy-find any repo"),
        ("i tab", "ask the AI · it can run git (changes need your y)"),
        ("s · d", "sort · drift only"),
        ("g G", "top · bottom"),
    ];
    let mut lines: Vec<Line> = rows
        .iter()
        .map(|&(k, d)| Line::from(vec![format!("{k:>12}  ").fg(ACCENT).bold(), d.fg(TEXT)]))
        .collect();
    lines.push(Line::default());
    lines.push(Line::from(
        Tone::ALL
            .iter()
            .map(|&t| format!(" {} {:?}", glyph(t), t).fg(theme::tone(t)))
            .collect::<Vec<_>>(),
    ));
    lines.push(Line::from(
        "  actions hit marked repos, else the selection's subtree".fg(DIM),
    ));
    lines.push(Line::default());
    let how = [
        (
            "pace",
            "commits/wk, weighted so each week counts half as much 4 weeks later",
        ),
        (
            "trend",
            "exact binomial test: last 4w vs prior 12w, reported only at p < 0.05",
        ),
        (
            "quiet",
            "P(a silence this long | Poisson at its 16w rate) = e^(-rate x gap)",
        ),
        (
            "cadence",
            "B = (sd - mean)/(sd + mean) of gaps: -1 regular, 0 random, 1 sprints",
        ),
        (
            "rhythm",
            "circular mean of commit times; R = 1 same minute daily, 0 uniform",
        ),
        ("focus", "fewest repos holding 80% of a folder's commits"),
        ("grid", "weeks across, Mon-Sun down; shade = quartile of active days"),
    ];
    lines.extend(
        how.iter()
            .map(|&(k, d)| Line::from(vec![format!("{k:>12}  ").fg(ACCENT), d.fg(DIM)])),
    );
    let pop = area.centered(Length(90.min(area.width)), Length(lines.len() as u16 + 2));
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(ACCENT))
        .title(" keys · how to read ");
    f.render_widget(Clear, pop);
    f.render_widget(Paragraph::new(lines).block(block).bg(BG), pop);
}
