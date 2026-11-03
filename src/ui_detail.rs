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
        let upstream = if s.unborn {
            "unborn local folder (never in git)".fg(t(Tone::Unborn))
        } else if !s.has_remote {
            "local git repo · never uploaded to GitHub".fg(t(Tone::Unpushed))
        } else {
            s.upstream
                .clone()
                .map_or("no upstream branch".fg(t(Tone::Unpushed)), |u| u.fg(DIM))
        };
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
            _ if s.unborn => (
                Tone::Unborn,
                "untracked app folder · press G to init, create GitHub repo & push, or c to commit locally".into(),
            ),
            _ if !s.has_remote => (
                Tone::Unpushed,
                "never uploaded to GitHub · press G to provision remote, graft & push".into(),
            ),
            _ if a > 0 && b > 0 => (
                Tone::Behind,
                format!("diverged: {a} local vs {b} remote · press p for smart-pull (rebase/graft)"),
            ),
            _ if b > 0 => (
                Tone::Behind,
                format!("{b} commit{} to pull · p smart-pulls (auto-stashes dirty files)", plural(b)),
            ),
            _ if a > 0 => (Tone::Ahead, format!("{a} commit{} to push · P smart-pushes", plural(a))),
            _ if s.upstream.is_none() => (
                Tone::Unpushed,
                format!("branch not published · press P or G to publish {}", s.branch),
            ),
            _ => (Tone::Clean, "in sync with upstream".into()),
        };
        v.push(Line::from(verdict.1.fg(t(verdict.0))));
        if a + b > 0 {
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
        if !s.branches.is_empty() {
            let mut br_spans = vec!["branches (b): ".fg(DIM)];
            for br in s.branches.iter().take(6) {
                let col = if br.head {
                    t(Tone::Clean)
                } else if br.remote_only {
                    t(Tone::Behind)
                } else {
                    TEXT
                };
                br_spans.push(
                    format!(
                        "{}{} ",
                        if br.head { "●" } else if br.remote_only { "⇣" } else { "○" },
                        br.name
                    )
                    .fg(col),
                );
            }
            if s.branches.len() > 6 {
                br_spans.push(format!("+{}", s.branches.len() - 6).fg(DIM));
            }
            v.push(Line::from(br_spans));
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
        if !s.unborn {
            if let Some(p) = self.pulses.get(&e.rel) {
                v.extend(self.activity(p, w));
                v.extend(self.people(p, w));
            }
        }
        v
    }
}

fn plural(n: u32) -> &'static str {
    if n == 1 { "" } else { "s" }
}
