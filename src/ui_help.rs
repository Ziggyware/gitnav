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
        ("c / C", "stage & commit modal · cd into selection"),
        ("b", "branch deck: switch, create, delete, unify master→main"),
        ("G", "publish & graft to GitHub (unborn folders or local repos)"),
        ("N", "scaffold new project folder (rs:app, ts:app, py:app)"),
        ("u · d · s", "filter unuploaded apps · drift only · cycle sort"),
        ("space a esc", "mark · mark drifted · clear"),
        ("f p P", "fetch · smart-pull (stash+graft) · smart-push"),
        ("r /", "re-check statuses · fuzzy-find any project"),
        ("i tab", "ask the AI · it can run git (changes need your y)"),
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
        "  ◊ Unborn = local project without .git · ⌂ Unpushed = never uploaded to GitHub".fg(DIM),
    ));
    lines.push(Line::default());
    let how = [
        (
            "graft (G)",
            "initializes .git, commits code, creates/binds GitHub repo & grafts disjoint histories",
        ),
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
    ];
    lines.extend(
        how.iter()
            .map(|&(k, d)| Line::from(vec![format!("{k:>12}  ").fg(ACCENT), d.fg(DIM)])),
    );
    let pop = area.centered(Length(94.min(area.width)), Length(lines.len() as u16 + 2));
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(ACCENT))
        .title(" keys · how to read ");
    f.render_widget(Clear, pop);
    f.render_widget(Paragraph::new(lines).block(block).bg(BG), pop);
}
