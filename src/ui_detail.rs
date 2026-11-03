use crate::{
    app::{App, Entry},
    history::Pulse,
    porcelain::{Status, Tone},
    theme::{self, *},
    ui_activity::{metrics, rhythm},
    ui_heat::heatmap,
};
use ratatui::prelude::*;
use ratatui::widgets::*;

impl App {
    pub(crate) fn detail(&self, f: &mut Frame, area: Rect, e: &Entry) {
        let title = format!(" {} ", e.name).fg(theme::tone(e.worst())).bold();
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(COLD))
            .title(title);
        let w = block.inner(area).width;
        let lines = match self.repos.get(&e.rel) {
            Some(Some(s)) if e.repo => self.repo_lines(e, s, w),
            _ if e.repo => vec![Line::from(spin(self.tick).fg(DIM))],
            _ => self.folder_lines(e, w),
        };
        f.render_widget(Paragraph::new(lines).block(block).wrap(Wrap { trim: false }), area);
    }

    pub(crate) fn activity(&self, p: &Pulse, w: u16) -> Vec<Line<'static>> {
        let mut v = vec![Line::default()];
        v.extend(heatmap(p, w));
        v.extend(metrics(p, self.now));
        v.push(Line::default());
        v.extend(rhythm(p, w));
        v
    }

    fn repo_lines(&self, e: &Entry, s: &Status, w: u16) -> Vec<Line<'static>> {
        let t = |x| theme::tone(x);
        let info = self.infos.get(&e.rel).and_then(Option::as_ref);
        let upstream = s
            .upstream
            .clone()
            .map_or("no upstream".fg(t(Tone::Unpushed)), |u| u.fg(DIM));
        let mut v = vec![Line::from(vec![
            s.branch.clone().fg(TEXT).bold(),
            " → ".fg(DIM),
            upstream,
        ])];
        let (a, b) = (s.ahead, s.behind);
        let verdict = match () {
            _ if s.error.is_some() || self.failures.contains_key(&e.rel) => {
                let err = self
                    .failures
                    .get(&e.rel)
                    .or(s.error.as_ref())
                    .cloned()
                    .unwrap_or_default();
                (Tone::Error, format!("git error: {}", err.lines().next().unwrap_or("")))
            }
            _ if a > 0 && b > 0 => (
                Tone::Behind,
                format!("diverged: {a} local vs {b} remote commits · pull --ff-only will refuse; rebase or merge"),
            ),
            _ if b > 0 => (
                Tone::Behind,
                format!("{b} commit{} to pull · p fast-forwards", plural(b)),
            ),
            _ if a > 0 => (Tone::Ahead, format!("{a} commit{} to push · P publishes", plural(a))),
            _ if s.upstream.is_none() => (
                Tone::Unpushed,
                format!("branch not published · git push -u origin {}", s.branch),
            ),
            _ => (Tone::Clean, "in sync with upstream".into()),
        };
        v.push(Line::from(verdict.1.fg(t(verdict.0))));
        if a + b > 0 {
            // Merge-base fork: what each side has that the other lacks.
            let dots = |n: u32| format!("{}{}", "• ".repeat(n.min(18) as usize), if n > 18 { "… " } else { "" });
            v.push(Line::from(vec![
                "     ┌─ ".fg(DIM),
                dots(b).fg(t(Tone::Behind)),
                format!("origin +{b}").fg(DIM),
            ]));
            v.push(Line::from(vec![" base┤".fg(DIM)]));
            v.push(Line::from(vec![
                "     └─ ".fg(DIM),
                dots(a).fg(t(Tone::Ahead)),
                format!("local  +{a}").fg(DIM),
            ]));
        }
        v.push(match info {
            Some(i) => Line::from(vec![
                "› ".fg(DIM),
                i.subject.clone().fg(TEXT),
                format!(" · {} · {} · {}", i.author, theme::age(self.now, i.when), i.remote).fg(DIM),
            ]),
            None => Line::from(spin(self.tick).fg(DIM)),
        });
        v.extend(self.change_lines(s, info, w));
        if let Some(p) = self.pulses.get(&e.rel) {
            v.extend(self.activity(p, w));
            v.extend(self.people(p, w));
        }
        v
    }
}

fn plural(n: u32) -> &'static str {
    if n == 1 { "" } else { "s" }
}
