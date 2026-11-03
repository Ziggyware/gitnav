use crate::{
    app::{App, Entry},
    history::WEEKS,
    theme::*,
};
use ratatui::{layout::Constraint::*, prelude::*, widgets::*};

#[derive(Clone, Copy, PartialEq)]
pub enum Col {
    Lead,
    Name,
    Act,
    Pace,
    Trend,
    Lines,
    People,
    Last,
    State,
}

/// (column, header, width); 0 = flexible. Order is display order.
const COLS: [(Col, &str, u16); 9] = [
    (Col::Lead, "", 2),
    (Col::Name, "repo", 0),
    (Col::Act, "activity · 16w", WEEKS as u16),
    (Col::Pace, "/wk", 4),
    (Col::Trend, "trend", 5),
    (Col::Lines, "lines", 5),
    (Col::People, "ppl", 3),
    (Col::Last, "last", 4),
    (Col::State, "state", 12),
];
/// Dropped first to last when the pane is too narrow; names keep at least 12 cells.
const DROP: [Col; 5] = [Col::People, Col::Lines, Col::Trend, Col::Pace, Col::Act];

fn visible(width: u16) -> Vec<(Col, &'static str, u16)> {
    let mut cols = COLS.to_vec();
    let need = |c: &[(Col, &str, u16)]| c.iter().map(|x| x.2 + 1).sum::<u16>() + 12;
    for d in DROP {
        if need(&cols) <= width {
            break;
        }
        cols.retain(|c| c.0 != d);
    }
    cols
}

impl App {
    pub(crate) fn list(&mut self, f: &mut Frame, area: Rect, entries: &[Entry]) {
        // Leftover height becomes a cross-repo commit feed instead of blank rows.
        let need = entries.len() as u16 + 3;
        let (area, feed) = if area.height >= need + 8 {
            let [a, b] = Layout::vertical([Length(need), Fill(1)]).areas(area);
            (a, Some(b))
        } else {
            (area, None)
        };
        let cols = visible(area.width.saturating_sub(2));
        let max = entries.iter().flat_map(|e| e.pulse.weeks()).max().unwrap_or(1).max(1);
        let rows: Vec<Row> = entries.iter().map(|e| self.row(e, max, &cols)).collect();
        let what = if self.search.is_some() {
            "matches"
        } else {
            self.sort.label()
        };
        let title = Line::from(vec![
            format!(" {} ", entries.len()).fg(ACCENT).bold(),
            format!("· {what} ").fg(DIM),
        ]);
        let legend = Line::from(vec![
            " bars: height commits · ".fg(DIM),
            "R".fg(Color::Rgb(255, 70, 70)).bold(),
            " lines/commit ".fg(DIM),
            "G".fg(Color::Rgb(70, 255, 70)).bold(),
            " others ".fg(DIM),
            "B".fg(Color::Rgb(90, 140, 255)).bold(),
            " off-hours ".fg(DIM),
        ]);
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(COLD))
            .title(title)
            .title_bottom(legend);
        let header = Row::new(cols.iter().map(|c| Cell::from(c.1.fg(DIM))));
        let widths: Vec<Constraint> = cols
            .iter()
            .map(|c| if c.2 == 0 { Fill(1) } else { Length(c.2) })
            .collect();
        let table = Table::new(rows, widths)
            .header(header)
            .block(block)
            .row_highlight_style(Style::new().bg(SEL));
        f.render_stateful_widget(table, area, &mut self.table);
        let sel = self.table.selected().and_then(|i| entries.get(i));
        if let (Some(feed), Some(e)) = (feed, sel) {
            self.feed(f, feed, e);
        }
    }
}
