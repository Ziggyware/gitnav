use crate::{
    app::{App, Sort},
    events::Batch,
    porcelain::Tone,
    theme::{self, *},
};
use ratatui::{layout::Constraint::*, prelude::*, widgets::*};

impl App {
    pub(crate) fn footer(&self) -> Line<'static> {
        if let Some((op, rels)) = &self.confirm {
            let ask = format!(
                " {} {} repos?  y to confirm, any key cancels ",
                op.verb(),
                rels.len()
            );
            return Line::from(ask.fg(BG).bg(theme::tone(Tone::Unpushed)).bold());
        }
        if let Some(q) = &self.search {
            let mut l = vec![" / ".fg(BG).bg(ACCENT).bold(), format!(" {q}▏").fg(TEXT)];
            l.extend([key("⏎", "jump"), key("↑↓", "move"), key("esc", "cancel")].concat());
            return Line::from(l);
        }
        let keys = [
            ("⏎", "open"),
            ("←", "up"),
            ("␣", "mark"),
            ("f", "fetch"),
            ("p", "pull"),
            ("P", "push"),
        ];
        let more = [
            ("/", "find"),
            ("s", "sort"),
            ("d", "drift"),
            ("?", "help"),
            ("q", "quit"),
        ];
        Line::from(
            keys.iter()
                .chain(&more)
                .flat_map(|&(k, l)| key(k, l))
                .collect::<Vec<_>>(),
        )
    }
}

pub fn batch_line(b: &Batch) -> Line<'static> {
    let (done, w) = (b.ok + b.failed, 24);
    let fill = done * w / b.total.max(1);
    let mut l = vec![
        format!(" {} ", b.op.verb()).fg(BG).bg(ACCENT).bold(),
        format!(" {}", "━".repeat(fill)).fg(theme::tone(if b.failed > 0 {
            Tone::Error
        } else {
            Tone::Clean
        })),
        "━".repeat(w - fill).fg(COLD),
        format!(" {done}/{}  ", b.total).fg(TEXT),
        format!("✓{}", b.ok).fg(theme::tone(Tone::Clean)),
    ];
    if b.failed > 0 {
        l.push(format!(" ✖{}", b.failed).fg(theme::tone(Tone::Error)));
    }
    if let Some((rel, e)) = &b.last_err {
        l.push(format!("  {}: {e}", rel.display()).fg(DIM));
    }
    Line::from(l)
}

pub fn help(f: &mut Frame, area: Rect) {
    let rows = [
        ("⏎ → l", "open folder · cd into repo"),
        ("← h ⌫", "up a level"),
        ("c", "cd into selection"),
        ("␣ / a / esc", "mark · mark drifted · clear"),
        ("f p P", "fetch · pull --ff-only · push"),
        ("r", "re-check statuses here"),
        ("/", "fuzzy-find any repo"),
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
    let pop = area.centered(Length(72), Length(lines.len() as u16 + 2));
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(ACCENT))
        .title(" keys ");
    f.render_widget(Clear, pop);
    f.render_widget(Paragraph::new(lines).block(block).bg(BG), pop);
}

impl Sort {
    pub fn label(self) -> &'static str {
        ["name", "attention", "recent"][self as usize]
    }
}
