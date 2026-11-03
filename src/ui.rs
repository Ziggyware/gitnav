use crate::{
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
}
