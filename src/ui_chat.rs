use crate::{
    app::App,
    chat::Item,
    porcelain::Tone,
    theme::{self, *},
    wrap::wrap,
};
use ratatui::{layout::Constraint::*, prelude::*, widgets::*};

impl App {
    pub(crate) fn chat_view(&self, f: &mut Frame, area: Rect) {
        let c = &self.chat;
        let cfg = c.cfg.as_ref();
        let model = cfg.map_or("?", |c| c.model.as_str());
        let title = Line::from(vec![" ask ".fg(BG).bg(ACCENT).bold(), format!(" {model} ").fg(DIM)]);
        let edge = if c.focus { ACCENT } else { COLD };
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(edge))
            .title(title);
        let inner = block.inner(area);
        f.render_widget(block, area);
        let w = inner.width as usize;
        let shown = if c.key_entry {
            format!("key › {}▏", "•".repeat(c.input.chars().count()))
        } else {
            format!("› {}▏", c.input)
        };
        let input = wrap(&shown, w);
        let [log, rule, field] = Layout::vertical([Fill(1), Length(1), Length(input.len().min(4) as u16)]).areas(inner);

        let t = |x| theme::tone(x);
        let mut lines: Vec<Line> = Vec::new();
        let mut para = |prefix: Span<'static>, text: &str, style: Style| {
            for (i, l) in wrap(text, w.saturating_sub(5)).into_iter().enumerate() {
                let lead = if i == 0 { prefix.clone() } else { "     ".into() };
                lines.push(Line::from(vec![lead, Span::styled(l, style)]));
            }
        };
        if c.key_entry {
            let at = crate::ai::key_path().map_or("your config dir".into(), |p| p.display().to_string());
            let host = cfg.map_or("the router", |c| c.url.as_str());
            let msg = format!(
                "Paste your nokey-router key (ROUTER_API_KEY) and press enter. Stored in {at}, readable only by you, sent only to {host}. Esc cancels; /key changes it later."
            );
            para(" key ".fg(BG).bg(ACCENT), &msg, Style::new().fg(TEXT));
        } else if c.items.is_empty() {
            let hint = "Ask about your repos, or tell it what to do: \"which repos have unpushed work?\", \
                        \"summarise this week's commits under dev/\", \"pull everything that is behind\". \
                        Inspecting git runs on its own; changes wait for your y.";
            para("     ".into(), hint, Style::new().fg(DIM));
        }
        for item in &c.items {
            match item {
                Item::You(q) => para("you  ".fg(ACCENT).bold(), q, Style::new().fg(TEXT).bold()),
                Item::Ai(a) => para("ai   ".fg(t(Tone::Clean)).bold(), a, Style::new().fg(TEXT)),
                Item::Error(e) => para(" !   ".fg(t(Tone::Error)), e, Style::new().fg(t(Tone::Error))),
                Item::Ran { cmd, out, ok } => {
                    let tone = if *ok { ACCENT } else { t(Tone::Error) };
                    para("  $  ".fg(tone), cmd, Style::new().fg(tone));
                    let body: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
                    for l in body.iter().take(4) {
                        para("     ".into(), l, Style::new().fg(DIM));
                    }
                    if body.len() > 4 {
                        para(
                            "     ".into(),
                            &format!("… {} more lines", body.len() - 4),
                            Style::new().fg(DIM),
                        );
                    }
                }
            }
        }
        if c.busy {
            para(
                format!("  {}  ", spin(self.tick)).fg(ACCENT),
                "thinking",
                Style::new().fg(DIM),
            );
        }
        let h = log.height as usize;
        let top = lines
            .len()
            .saturating_sub(h + c.scroll.min(lines.len().saturating_sub(h)));
        f.render_widget(
            Paragraph::new(lines.into_iter().skip(top).take(h).collect::<Vec<_>>()),
            log,
        );
        f.render_widget(Line::from("─".repeat(w).fg(COLD)), rule);
        let field_lines: Vec<Line> = input
            .into_iter()
            .rev()
            .take(4)
            .rev()
            .map(|l| Line::from(l.fg(TEXT)))
            .collect();
        f.render_widget(Paragraph::new(field_lines), field);
    }
}
