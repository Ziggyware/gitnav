//! Interactive modals for Commit, Branch Deck, GitHub Provisioning/Grafting, and Scaffolding.
use crate::{
    app::Modal,
    porcelain::Tone,
    theme::{self, *},
};
use ratatui::{layout::Constraint::*, prelude::*, widgets::*};

pub fn render_modal(f: &mut Frame, area: Rect, modal: &Modal) {
    match modal {
        Modal::Commit { targets, msg } => {
            let lines = vec![
                Line::from(vec![
                    "Targets: ".fg(DIM),
                    format!("{} project(s) (stages all files & initializes .git if unborn)", targets.len())
                        .fg(TEXT),
                ]),
                Line::default(),
                Line::from(vec![
                    "message › ".fg(ACCENT).bold(),
                    format!("{msg}▏").fg(TEXT).bold(),
                ]),
                Line::default(),
                Line::from(
                    [
                        key("enter", "stage & commit"),
                        key("esc", "cancel"),
                    ]
                    .concat(),
                ),
            ];
            draw_box(f, area, 76, 7, " commit all changes ", ACCENT, lines);
        }
        Modal::Adopt { targets, spec } => {
            let has_tok = crate::github::token().is_some();
            let auth_badge = if has_tok {
                Span::styled("● GitHub auth ready", Style::new().fg(theme::tone(Tone::Clean)))
            } else {
                Span::styled(
                    "○ No GITHUB_TOKEN/gh auth (pass owner/repo or https:// URL)",
                    Style::new().fg(theme::tone(Tone::Dirty)),
                )
            };
            let lines = vec![
                Line::from(vec![
                    "Pipeline: ".fg(DIM),
                    "init .git + .gitignore → commit local code → create/bind GitHub → DAG graft → push -u".fg(TEXT),
                ]),
                Line::from(vec!["Auth:     ".fg(DIM), auth_badge]),
                Line::from(vec![
                    "Format:   ".fg(DIM),
                    "repo-name (private), public:repo-name, owner/repo, or https://...git".fg(DIM),
                ]),
                Line::default(),
                Line::from(vec![
                    format!("publish ({}) › ", targets.len()).fg(theme::tone(Tone::Unborn)).bold(),
                    format!("{spec}▏").fg(TEXT).bold(),
                ]),
                Line::default(),
                Line::from([key("enter", "provision, graft & push"), key("esc", "cancel")].concat()),
            ];
            draw_box(
                f,
                area,
                88,
                9,
                " publish & graft to GitHub ",
                theme::tone(Tone::Unborn),
                lines,
            );
        }
        Modal::NewProject { cwd, spec } => {
            let where_str = if cwd.as_os_str().is_empty() {
                ".".to_owned()
            } else {
                cwd.display().to_string()
            };
            let lines = vec![
                Line::from(vec![
                    "Parent folder: ".fg(DIM),
                    where_str.fg(TEXT).bold(),
                ]),
                Line::from(vec![
                    "Template:      ".fg(DIM),
                    "rs:my-app (Cargo), ts:my-app (Deno), py:my-app (Python), or bare name".fg(DIM),
                ]),
                Line::default(),
                Line::from(vec![
                    "scaffold › ".fg(ACCENT).bold(),
                    format!("{spec}▏").fg(TEXT).bold(),
                ]),
                Line::default(),
                Line::from([key("enter", "create folder, build skeleton & init git"), key("esc", "cancel")].concat()),
            ];
            draw_box(f, area, 82, 8, " new project folder ", ACCENT, lines);
        }
        Modal::Branches {
            rel,
            items,
            sel,
            creating,
        } => {
            let name = if rel.as_os_str().is_empty() {
                ".".to_owned()
            } else {
                rel.display().to_string()
            };
            let mut lines = Vec::new();
            if items.is_empty() {
                lines.push(Line::from(
                    "No branches yet (unborn project). Press 'n' to initialize & create a branch.".fg(DIM),
                ));
            }
            for (i, b) in items.iter().take(12).enumerate() {
                let active = i == *sel;
                let lead = if active { "› " } else { "  " };
                let marker = if b.head {
                    "● "
                } else if b.remote_only {
                    "⇣ "
                } else {
                    "○ "
                };
                let col = if b.head {
                    theme::tone(Tone::Clean)
                } else if b.remote_only {
                    theme::tone(Tone::Behind)
                } else {
                    TEXT
                };
                let mut row = vec![
                    lead.fg(ACCENT).bold(),
                    marker.fg(col),
                    b.name.clone().fg(col).bold(),
                ];
                if let Some(up) = &b.upstream {
                    row.push(format!(" → {up}").fg(DIM));
                } else if !b.remote_only {
                    row.push(" (local only)".fg(theme::tone(Tone::Unpushed)));
                }
                if b.ahead > 0 {
                    row.push(format!(" ↑{}", b.ahead).fg(theme::tone(Tone::Ahead)));
                }
                if b.behind > 0 {
                    row.push(format!(" ↓{}", b.behind).fg(theme::tone(Tone::Behind)));
                }
                let mut l = Line::from(row);
                if active {
                    l = l.bg(SEL);
                }
                lines.push(l);
            }
            lines.push(Line::default());
            if let Some(new_br) = creating {
                lines.push(Line::from(vec![
                    "new branch › ".fg(ACCENT).bold(),
                    format!("{new_br}▏").fg(TEXT).bold(),
                ]));
                lines.push(Line::from(
                    [key("enter", "create & switch (auto-stashes dirty work)"), key("esc", "back")].concat(),
                ));
            } else {
                lines.push(Line::from(
                    [
                        key("enter", "switch/track"),
                        key("n", "new branch"),
                        key("x", "delete"),
                        key("m", "unify master→main"),
                        key("p/P", "pull/push"),
                        key("esc", "close"),
                    ]
                    .concat(),
                ));
            }
            let h = (lines.len() as u16 + 2).clamp(6, area.height.saturating_sub(2));
            draw_box(
                f,
                area,
                88,
                h,
                &format!(" branches · {name} "),
                ACCENT,
                lines,
            );
        }
    }
}

fn draw_box(
    f: &mut Frame,
    area: Rect,
    w: u16,
    h: u16,
    title: &str,
    border_col: Color,
    lines: Vec<Line<'static>>,
) {
    let pop = area.centered(Length(w.min(area.width)), Length(h.min(area.height)));
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(border_col))
        .title(Span::styled(
            title.to_owned(),
            Style::new().fg(BG).bg(border_col).bold(),
        ));
    f.render_widget(Clear, pop);
    f.render_widget(Paragraph::new(lines).block(block).bg(BG), pop);
}
