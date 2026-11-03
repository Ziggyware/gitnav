use crate::{
    app::{App, Entry},
    porcelain::Status,
};
use ratatui::{layout::Constraint::*, prelude::*, widgets::*};

impl App {
    pub fn draw(&mut self, f: &mut Frame) {
        let entries = self.entries();
        let sel = self
            .list
            .selected()
            .unwrap_or(0)
            .min(entries.len().saturating_sub(1));
        self.list.select(Some(sel));
        let [head, body, foot] = Layout::vertical([Length(1), Fill(1), Length(1)]).areas(f.area());
        let [left, right] = Layout::horizontal([Percentage(45), Fill(1)]).areas(body);

        let pending = self
            .repos
            .values()
            .filter(|s| s.as_ref().is_none_or(|s| s.stale))
            .count();
        let drift = self.repos.values().flatten().filter(|s| s.drift()).count();
        let walk = if self.scanning { " · scanning" } else { "" };
        let title = format!(
            " {}  {} repos · {drift} drift · {pending} pending{walk}",
            self.abs(&self.cwd).display(),
            self.repos.len()
        );
        f.render_widget(Line::from(title).bold(), head);

        let items: Vec<ListItem> = entries
            .iter()
            .map(|e| ListItem::new(self.entry_line(e)))
            .collect();
        let block = Block::bordered().title(if self.only_drift {
            " drift only "
        } else {
            " all "
        });
        f.render_stateful_widget(
            List::new(items)
                .block(block)
                .highlight_style(Style::new().reversed()),
            left,
            &mut self.list,
        );
        let detail = entries.get(sel).map(|e| self.detail(e)).unwrap_or_default();
        f.render_widget(
            Paragraph::new(detail)
                .block(Block::bordered())
                .wrap(Wrap { trim: false }),
            right,
        );
        f.render_widget(
            Line::from(" ↑↓ move  →/⏎ open  ← up  c cd  f fetch  r refresh  d filter  q quit")
                .dim(),
            foot,
        );
    }

    fn entry_line(&self, e: &Entry) -> Line<'static> {
        let mut l = vec![
            Span::raw(if e.repo { "● " } else { "▸ " }),
            Span::raw(e.name.clone()).bold(),
        ];
        if e.repo {
            match self.repos.get(&e.rel) {
                Some(Some(s)) => l.extend(badges(s)),
                _ => l.push(" …".dim()),
            }
        } else {
            l.push(format!("  {}/{}", e.drift, e.total).fg(if e.drift > 0 {
                Color::Yellow
            } else {
                Color::Green
            }));
            if e.pending > 0 {
                l.push(format!(" …{}", e.pending).dim());
            }
        }
        Line::from(l)
    }

    fn detail(&self, e: &Entry) -> Vec<Line<'static>> {
        if !e.repo {
            let v: Vec<Line> = self
                .under(&e.rel)
                .filter_map(|(p, s)| s.as_ref().filter(|s| s.drift()).map(|s| (p, s)))
                .map(|(p, s)| {
                    let mut l = vec![Span::raw(
                        p.strip_prefix(&e.rel).unwrap().display().to_string(),
                    )];
                    l.extend(badges(s));
                    Line::from(l)
                })
                .collect();
            return if v.is_empty() {
                vec![format!("{} repos, no drift", e.total).green().into()]
            } else {
                v
            };
        }
        let Some(Some(s)) = self.repos.get(&e.rel) else {
            return vec!["…".dim().into()];
        };
        let kv = |k: &'static str, v: String| Line::from(vec![k.dim(), v.into()]);
        let mut v = vec![
            kv("branch   ", s.branch.clone()),
            kv("upstream ", s.upstream.clone().unwrap_or("—".into())),
            kv("ahead    ", s.ahead.to_string()),
            kv("behind   ", s.behind.to_string()),
            kv(
                "remote   ",
                self.remotes.get(&e.rel).cloned().unwrap_or_default(),
            ),
        ];
        if let Some(err) = &s.error {
            v.push(Line::from(err.clone()).red());
        }
        v.push(Line::default());
        v.extend(s.changes.iter().map(|c| Line::from(c.clone()).yellow()));
        v
    }
}

fn badges(s: &Status) -> Vec<Span<'static>> {
    let mut v = Vec::new();
    if s.error.is_some() {
        v.push(" !err".red());
    }
    if s.dirty() {
        v.push(format!(" *{}", s.changes.len()).yellow());
    }
    if s.ahead > 0 {
        v.push(format!(" ↑{}", s.ahead).cyan());
    }
    if s.behind > 0 {
        v.push(format!(" ↓{}", s.behind).magenta());
    }
    if s.upstream.is_none() {
        v.push(" no-upstream".red());
    }
    if v.is_empty() {
        v.push(" ✓".green());
    }
    if s.stale {
        v = v.into_iter().map(Stylize::dim).collect();
    }
    v
}
