use crate::{app::App, encode::*, history::Pulse, theme::*};
use ratatui::prelude::*;
use std::collections::HashMap;

impl App {
    pub fn person(&self, name: &str) -> ratatui::style::Color {
        crate::encode::person(self.ranks.get(name).copied())
    }

    pub fn rerank(&mut self) {
        let mut n: HashMap<&str, u32> = HashMap::new();
        self.pulses
            .values()
            .flat_map(|p| &p.authors)
            .for_each(|(a, c)| *n.entry(a).or_default() += c);
        let mut v: Vec<(&str, u32)> = n.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        self.ranks = v.into_iter().enumerate().map(|(i, (a, _))| (a.to_owned(), i)).collect();
    }

    /// Who did the work and where it landed.
    pub(crate) fn people(&self, p: &Pulse, w: u16) -> Vec<Line<'static>> {
        let mut v = Vec::new();
        let total: u32 = p.authors.iter().map(|a| a.1).sum();
        if total > 0 {
            let width = w as usize;
            v.push(Line::default());
            v.push(Line::from(
                format!("authors · {} people · 16 weeks", p.authors.len()).fg(DIM),
            ));
            let mut cells: Vec<usize> = p
                .authors
                .iter()
                .map(|a| (a.1 as usize * width / total as usize).max(1))
                .collect();
            while cells.iter().sum::<usize>() > width {
                let i = (0..cells.len()).max_by_key(|&i| cells[i]).unwrap();
                cells[i] -= 1;
            }
            let pad = width.saturating_sub(cells.iter().sum());
            cells[0] += pad;
            v.push(Line::from(
                p.authors
                    .iter()
                    .zip(&cells)
                    .map(|(a, &n)| "━".repeat(n).fg(self.person(&a.0)))
                    .collect::<Vec<_>>(),
            ));
            let mut legend: Vec<Span> = p
                .authors
                .iter()
                .take(5)
                .map(|(name, n)| format!("{name} {}%  ", n * 100 / total).fg(self.person(name)))
                .collect();
            if p.authors.len() > 5 {
                legend.push(format!("+{}", p.authors.len() - 5).fg(DIM));
            }
            v.push(Line::from(legend));
        }
        if !p.hot.is_empty() {
            v.push(Line::default());
            v.push(Line::from("hotspots · lines changed per file, 16 weeks".fg(DIM)));
            let max = p.hot.first().map_or(1, |h| h.1.max(1)) as usize;
            for (path, n) in p.hot.iter().take(6) {
                let k = (*n as usize * 8).div_ceil(max);
                let (bar, gap) = (
                    "━"
                        .repeat(k)
                        .fg(lerp(Color::Rgb(120, 90, 90), Color::Rgb(255, 90, 90), k as f32 / 8.0)),
                    " ".repeat(8 - k),
                );
                v.push(Line::from(vec![
                    bar,
                    gap.into(),
                    format!("{:>6} ", kilo(*n)).fg(DIM),
                    path.clone().fg(TEXT),
                ]));
            }
        }
        v
    }
}
