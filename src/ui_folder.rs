use crate::{
    app::{App, Entry},
    porcelain::{Status, Tone},
    stats,
    theme::{self, *},
    ui_row::badges,
};
use ratatui::prelude::*;
use std::path::{Path, PathBuf};

impl App {
    pub(crate) fn folder_lines(&self, e: &Entry, w: u16) -> Vec<Line<'static>> {
        let legend = Tone::ALL.iter().filter(|&&t| e.tones[t as usize] > 0);
        let legend = legend.map(|&t| format!("{}{}  ", glyph(t), e.tones[t as usize]).fg(theme::tone(t)));
        let clean = 100 * e.tones[Tone::Clean as usize] / e.total.max(1);
        let mut v = vec![
            Line::from(bar(&e.tones, w as usize)),
            Line::from(
                legend
                    .chain([format!("· {} repos", e.total).fg(DIM)])
                    .collect::<Vec<_>>(),
            ),
            Line::from(format!("{clean}% clean · {} need attention", e.attention()).fg(TEXT)),
        ];
        let rel = |p: &Path| p.strip_prefix(&e.rel).unwrap_or(p).display().to_string();
        let mut by: Vec<(u32, &PathBuf)> = self
            .under(&e.rel)
            .filter_map(|r| Some((self.pulses.get(r)?.total(), r)))
            .collect();
        by.sort_by_key(|b| std::cmp::Reverse(b.0));
        let k = stats::focus(by.iter().map(|x| x.0).collect(), 0.8);
        if e.pulse.total() > 0 {
            let names: Vec<String> = by.iter().take(k.min(3)).map(|x| rel(x.1)).collect();
            let more = if k > 3 { format!(" +{}", k - 3) } else { String::new() };
            let line = format!("80% of commits in {k} of {} repos: {}{more}", e.total, names.join(", "));
            v.push(Line::from(vec![format!("{:<8}", "focus").fg(ACCENT), line.fg(TEXT)]));
        }
        v.extend(self.activity(&e.pulse, w));
        v.extend(self.people(&e.pulse, w));

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
        v.push(Line::default());
        v.push(Line::from(format!("needs attention · {}", hits.len()).fg(DIM)));
        if hits.is_empty() {
            v.push("all synced and clean".fg(theme::tone(Tone::Clean)).into());
        }
        for (r, s) in hits {
            let mut l = vec!["• ".fg(theme::tone(s.tone())), rel(r).fg(TEXT)];
            l.extend(badges(s));
            v.push(Line::from(l));
        }
        v
    }
}
