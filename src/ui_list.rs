use crate::{
    app::{App, Entry},
    history::WEEKS,
    porcelain::{Status, Tone},
    theme::{self, *},
};
use ratatui::{layout::Constraint::*, prelude::*, widgets::*};

impl App {
    pub(crate) fn list(&mut self, f: &mut Frame, area: Rect, entries: &[Entry]) {
        let max = entries
            .iter()
            .flat_map(|e| e.weeks)
            .max()
            .unwrap_or(1)
            .max(1);
        let rows: Vec<Row> = entries.iter().map(|e| self.row(e, max)).collect();
        let what = if self.search.is_some() {
            "matches"
        } else {
            self.sort.label()
        };
        let title = Line::from(vec![
            format!(" {} ", entries.len()).fg(ACCENT).bold(),
            format!("· {what} ").fg(DIM),
        ]);
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(COLD))
            .title(title);
        // Narrow panes drop the pulse column before squeezing names.
        let pulse = if area.width >= 64 { WEEKS as u16 } else { 0 };
        let widths = [Length(2), Fill(1), Length(pulse), Length(4), Length(13)];
        let table = Table::new(rows, widths)
            .block(block)
            .row_highlight_style(Style::new().bg(SEL));
        f.render_stateful_widget(table, area, &mut self.table);
    }

    fn row(&self, e: &Entry, max: u32) -> Row<'static> {
        let st = self.repos.get(&e.rel).and_then(Option::as_ref);
        let marked = self
            .marks
            .range(e.rel.clone()..)
            .next()
            .is_some_and(|m| m.starts_with(&e.rel));
        let busy = self.busy.iter().any(|b| b.starts_with(&e.rel));
        let pending = e.tones[Tone::Pending as usize] > 0;
        let icon = match () {
            _ if busy || pending => spin(self.tick),
            _ if e.repo => "●".into(),
            _ => "▸".into(),
        };
        let lead = Line::from(vec![
            if marked { "▌".fg(ACCENT) } else { " ".into() },
            icon.fg(theme::tone(e.worst())),
        ]);
        let stale = st.is_some_and(|s| s.stale);
        let name = match (e.repo, stale) {
            (false, _) => format!("{}/", e.name).fg(TEXT).bold(),
            (true, false) => e.name.clone().fg(TEXT),
            (true, true) => e.name.clone().fg(DIM),
        };
        let age = theme::age(self.now, e.last).fg(heat(self.now, e.last));
        let tail = if e.repo {
            st.map_or_else(Vec::new, badges)
        } else {
            let mut v = bar(&e.tones, 8);
            v.push(format!(" {:>3}", e.total).fg(DIM));
            v
        };
        let cells = [
            lead,
            name.into(),
            Line::from(spark(&e.weeks, max)),
            age.into(),
            Line::from(tail),
        ];
        Row::new(cells.map(Cell::from))
    }
}

pub fn badges(s: &Status) -> Vec<Span<'static>> {
    let t = |x| theme::tone(x);
    let mut v = Vec::new();
    if s.error.is_some() {
        v.push(" ✖".fg(t(Tone::Error)));
    }
    if s.dirty() {
        v.push(format!(" ✎{}", s.changes.len()).fg(t(Tone::Dirty)));
    }
    if s.ahead > 0 {
        v.push(format!(" ↑{}", s.ahead).fg(t(Tone::Ahead)));
    }
    if s.behind > 0 {
        v.push(format!(" ↓{}", s.behind).fg(t(Tone::Behind)));
    }
    if s.upstream.is_none() {
        v.push(" ◌".fg(t(Tone::Unpushed)));
    }
    if v.is_empty() {
        v.push(" ✓".fg(t(Tone::Clean)));
    }
    if s.stale {
        v = v.into_iter().map(Stylize::dim).collect();
    }
    v
}
