use crate::{
    app::{App, Entry},
    history::WEEKS,
    porcelain::{Status, Tone},
    theme::{self, *},
};
use ratatui::{layout::Constraint::*, prelude::*, widgets::*};
use std::iter::repeat_n;

impl App {
    pub(crate) fn detail(&self, f: &mut Frame, area: Rect, e: &Entry) {
        let title = format!(" {} ", e.name).fg(theme::tone(e.worst())).bold();
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(COLD))
            .title(title);
        let inner = block.inner(area);
        f.render_widget(block, area);
        let [top, pulse, rest] =
            Layout::vertical([Length(if e.repo { 5 } else { 3 }), Length(5), Fill(1)]).areas(inner);
        let commits: u32 = e.weeks.iter().sum();
        let caption = format!(
            " pulse · {commits} commits / {WEEKS}w · last {} ",
            theme::age(self.now, e.last)
        );
        let k = (pulse.width as usize / WEEKS).max(1);
        let data: Vec<u64> = e
            .weeks
            .iter()
            .flat_map(|&w| repeat_n(w as u64, k))
            .collect();
        let spark = Sparkline::default()
            .data(&data)
            .style(Style::new().fg(ACCENT));
        f.render_widget(spark.block(Block::new().title(caption.fg(DIM))), pulse);
        match self.repos.get(&e.rel) {
            Some(Some(s)) if e.repo => self.repo_detail(f, top, rest, e, s),
            _ if e.repo => f.render_widget(Paragraph::new(spin(self.tick).fg(DIM)), top),
            _ => self.folder_detail(f, top, rest, e),
        }
    }

    fn repo_detail(&self, f: &mut Frame, top: Rect, rest: Rect, e: &Entry, s: &Status) {
        let t = |x| theme::tone(x);
        let info = self.infos.get(&e.rel).and_then(Option::as_ref);
        let gauge = |n: u32, c| "▮".repeat(n.min(16) as usize).fg(c);
        let upstream = s
            .upstream
            .clone()
            .map_or("not pushed".fg(t(Tone::Unpushed)), |u| u.fg(DIM));
        let mut lines = vec![
            Line::from(vec![
                "⎇ ".fg(DIM),
                s.branch.clone().fg(TEXT).bold(),
                " → ".fg(DIM),
                upstream,
            ]),
            Line::from(vec![
                format!("↑{:<3} ", s.ahead).fg(t(Tone::Ahead)),
                gauge(s.ahead, t(Tone::Ahead)),
                format!("   ↓{:<3} ", s.behind).fg(t(Tone::Behind)),
                gauge(s.behind, t(Tone::Behind)),
            ]),
            match info {
                Some(i) => Line::from(vec![
                    "✎ ".fg(DIM),
                    i.subject.clone().fg(TEXT),
                    format!(" · {} · {}", i.author, theme::age(self.now, i.when)).fg(DIM),
                ]),
                None => Line::from(spin(self.tick).fg(DIM)),
            },
            Line::from(vec![
                "⌁ ".fg(DIM),
                info.map_or(String::new(), |i| i.remote.clone()).fg(DIM),
            ]),
        ];
        if let Some(err) = self.failures.get(&e.rel).or(s.error.as_ref()) {
            lines.push(Line::from(
                err.lines()
                    .next()
                    .unwrap_or("")
                    .to_owned()
                    .fg(t(Tone::Error)),
            ));
        }
        f.render_widget(Paragraph::new(lines), top);

        let mut v = vec![Line::from(
            format!("changes · {} ", s.changes.len()).fg(DIM),
        )];
        if s.changes.is_empty() {
            v.push("✓ working tree clean".fg(t(Tone::Clean)).into());
        }
        let stat = info.map_or(&[][..], |i| &i.stat[..]);
        let max = stat.iter().map(|(_, a, d)| a + d).max().unwrap_or(1).max(1) as usize;
        let cells = |n: u32| (n as usize * 14).div_ceil(max);
        for (p, a, d) in stat {
            v.push(Line::from(vec![
                format!("{:>6}", format!("+{a}")).fg(t(Tone::Clean)),
                format!("{:>6} ", format!("−{d}")).fg(t(Tone::Error)),
                "■".repeat(cells(*a)).fg(t(Tone::Clean)),
                "■".repeat(cells(*d)).fg(t(Tone::Error)),
                format!(" {p}").fg(TEXT),
            ]));
        }
        // numstat omits untracked files and is empty until info arrives.
        for c in s
            .changes
            .iter()
            .filter(|c| stat.is_empty() || c.starts_with("??") || c.starts_with('U'))
        {
            let (xy, p) = c.split_at(2.min(c.len()));
            let tone = if xy == "??" {
                Tone::Ahead
            } else if xy.contains('U') {
                Tone::Error
            } else {
                Tone::Dirty
            };
            v.push(Line::from(vec![
                format!("{:>13} ", xy).fg(t(tone)),
                p.trim().to_owned().fg(TEXT),
            ]));
        }
        f.render_widget(Paragraph::new(v), rest);
    }
}
