use crate::{
    app::App,
    events::Batch,
    nav::Sort,
    porcelain::Tone,
    risk::Risk,
    theme::{self, *},
};
use ratatui::prelude::*;

impl App {
    pub(crate) fn footer(&self) -> Line<'static> {
        if let Some((cmd, risk, _)) = &self.chat.confirm {
            let (tone, what) = match risk {
                Risk::Destructive => (Tone::Error, "DESTRUCTIVE"),
                _ => (Tone::Unpushed, "changes repo"),
            };
            let ask = format!(" AI wants to run ({what}): {cmd} ");
            let mut l = vec![ask.fg(BG).bg(theme::tone(tone)).bold()];
            l.extend([key("y", "run"), key("n", "deny")].concat());
            return Line::from(l);
        }
        if self.chat.open && self.chat.focus {
            let keys = [
                ("enter", "send"),
                ("↑↓ pgup", "scroll"),
                ("tab", "back to list"),
                ("esc", "close"),
            ];
            return Line::from(keys.iter().flat_map(|&(k, l)| key(k, l)).collect::<Vec<_>>());
        }
        if let Some((op, rels)) = &self.confirm {
            let ask = format!(" {} {} repos?  y to confirm, any key cancels ", op.verb(), rels.len());
            return Line::from(ask.fg(BG).bg(theme::tone(Tone::Unpushed)).bold());
        }
        if let Some(q) = &self.search {
            let mut l = vec![" / ".fg(BG).bg(ACCENT).bold(), format!(" {q}▏").fg(TEXT)];
            l.extend([key("enter", "jump"), key("↑↓", "move"), key("esc", "cancel")].concat());
            return Line::from(l);
        }
        let keys = [
            ("enter", "open"),
            ("b", "branches"),
            ("c", "commit"),
            ("G", "publish"),
            ("N", "new"),
            ("p/P", "pull/push"),
        ];
        let more = [
            ("u", "unuploaded"),
            ("d", "drift"),
            ("i", "ask AI"),
            ("/", "find"),
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
        format!(" {}", "━".repeat(fill)).fg(theme::tone(if b.failed > 0 { Tone::Error } else { Tone::Clean })),
        "━".repeat(w - fill).fg(COLD),
        format!(" {done}/{}  ", b.total).fg(TEXT),
        format!("={}", b.ok).fg(theme::tone(Tone::Clean)),
    ];
    if b.failed > 0 {
        l.push(format!(" ×{}", b.failed).fg(theme::tone(Tone::Error)));
    }
    if let Some((rel, e)) = &b.last_err {
        l.push(format!("  {}: {e}", rel.display()).fg(DIM));
    }
    Line::from(l)
}

impl Sort {
    pub fn label(self) -> &'static str {
        ["name", "attention", "recent"][self as usize]
    }
}
