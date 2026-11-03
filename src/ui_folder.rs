use crate::{
    app::{App, Entry},
    porcelain::{Status, Tone},
    theme::{self, *},
    ui_list::badges,
};
use ratatui::{prelude::*, widgets::*};
use std::path::PathBuf;

impl App {
    pub(crate) fn folder_detail(&self, f: &mut Frame, top: Rect, rest: Rect, e: &Entry) {
        let legend = Tone::ALL.iter().filter(|&&t| e.tones[t as usize] > 0);
        let legend =
            legend.map(|&t| format!("{}{}  ", glyph(t), e.tones[t as usize]).fg(theme::tone(t)));
        let lines = vec![
            Line::from(bar(&e.tones, top.width as usize)),
            Line::from(
                legend
                    .chain([format!("· {} repos", e.total).fg(DIM)])
                    .collect::<Vec<_>>(),
            ),
        ];
        f.render_widget(Paragraph::new(lines), top);

        let mut hits: Vec<(&PathBuf, &Status)> = self
            .under(&e.rel)
            .filter_map(|r| {
                self.repos[r]
                    .as_ref()
                    .filter(|s| s.tone() < Tone::Clean)
                    .map(|s| (r, s))
            })
            .collect();
        hits.sort_by_key(|(_, s)| s.tone());
        let mut v = vec![Line::from(
            format!("needs attention · {}", hits.len()).fg(DIM),
        )];
        if hits.is_empty() {
            v.push("✓ all synced and clean".fg(theme::tone(Tone::Clean)).into());
        }
        for (r, s) in hits.into_iter().take(rest.height as usize) {
            let mut l = vec![
                "● ".fg(theme::tone(s.tone())),
                r.strip_prefix(&e.rel)
                    .unwrap_or(r)
                    .display()
                    .to_string()
                    .fg(TEXT),
            ];
            l.extend(badges(s));
            v.push(Line::from(l));
        }
        f.render_widget(Paragraph::new(v), rest);
    }
}
