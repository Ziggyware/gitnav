use crate::{
<<<<<<< HEAD
    app::App,
    history::now,
    porcelain::{Tone, tone},
    theme::{self, *},
    ui_bar::{batch_line, help},
    ui_mosaic::tile,
};
use ratatui::{layout::Constraint::*, prelude::*};

impl App {
    pub fn draw(&mut self, f: &mut Frame) {
        self.now = now();
        let entries = self.entries();
        let sel = self
            .table
            .selected()
            .unwrap_or(0)
            .min(entries.len().saturating_sub(1));
        self.table.select(Some(sel));
        let area = f.area();
        let (n, w) = (self.repos.len(), area.width.max(1) as usize);
        let mosaic_h = (n * tile(n, w)).div_ceil(w * 2).clamp(1, 4) as u16;
        let log_h = self.batch.is_some() as u16;
        let rows = [
            Length(1),
            Length(mosaic_h),
            Fill(1),
            Length(log_h),
            Length(1),
        ];
        let [head, mosaic, body, log, foot] = Layout::vertical(rows).areas(area);
        // Phones get a stacked layout; wide terminals get side-by-side panes.
        let [left, right] = if area.width >= 100 {
            Layout::horizontal([Percentage(56), Fill(1)]).areas(body)
        } else {
            Layout::vertical([Fill(1), Length(body.height / 2)]).areas(body)
        };
        f.render_widget(self.header(), head);
        f.render_widget(self.mosaic(mosaic, entries.get(sel)), mosaic);
        self.list(f, left, &entries);
        if let Some(e) = entries.get(sel) {
            self.detail(f, right, e);
        }
        if let Some(b) = &self.batch {
            f.render_widget(batch_line(b), log);
        }
        f.render_widget(self.footer(), foot);
        if self.help {
            help(f, area);
        }
    }

    fn header(&self) -> Line<'static> {
        let mut counts = [0usize; 7];
        self.repos
            .values()
            .for_each(|s| counts[tone(s.as_ref()) as usize] += 1);
        let path = format!("  {}  ", self.abs(&self.cwd).display());
        let mut l = vec![" ◆ gitnav ".fg(BG).bg(ACCENT).bold(), path.fg(TEXT)];
        for t in Tone::ALL.into_iter().filter(|&t| counts[t as usize] > 0) {
            l.push(format!(" {}{}", glyph(t), counts[t as usize]).fg(theme::tone(t)));
        }
        if self.scanning || counts[Tone::Pending as usize] > 0 || !self.busy.is_empty() {
            l.push(format!("  {}", spin(self.tick)).fg(ACCENT));
        }
        if !self.marks.is_empty() {
            l.push(format!("  ▌{} marked", self.marks.len()).fg(ACCENT).bold());
        }
        let sort = ["", "  ⇅ attention", "  ⇅ recent"][self.sort as usize];
        l.extend([
            sort.fg(DIM),
            if self.only_drift { "  ⧩ drift" } else { "" }.fg(DIM),
        ]);
        Line::from(l)
    }
=======
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

        let pending = self.repos.values().filter(|s| s.is_none()).count();
        let drift = self.repos.values().flatten().filter(|s| s.drift()).count();
        let state = if self.scanning {
            " · scanning".into()
        } else if pending > 0 {
            format!(" · {pending} pending")
        } else {
            String::new()
        };
        let title = format!(
            " {}  {} repos · {drift} drift{state}",
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
            kv("remote   ", s.remote.clone().unwrap_or("—".into())),
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
        v.push(
            if s.remote.is_some() {
                " no-upstream"
            } else {
                " local-only"
            }
            .red(),
        );
    }
    if v.is_empty() {
        v.push(" ✓".green());
    }
    v
>>>>>>> ce532fa2a485cac25364c698a4d1c4994bba308d
}
