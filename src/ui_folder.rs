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
        let unuploaded = e.unuploaded();
        let mut v = vec![
            Line::from(bar(&e.tones, w as usize)),
            Line::from(
                legend
                    .chain([format!("· {} projects", e.total).fg(DIM)])
                    .collect::<Vec<_>>(),
            ),
            Line::from(
                format!(
                    "{clean}% clean · {} need attention · {unuploaded} never uploaded to GitHub",
                    e.attention()
                )
                .fg(TEXT),
            ),
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

        let unup_list: Vec<(&PathBuf, &Status)> = self
            .under(&e.rel)
            .filter_map(|r| {
                self.repos[r]
                    .as_ref()
                    .filter(|s| s.never_uploaded())
                    .map(|s| (r, s))
            })
            .collect();
        if !unup_list.is_empty() {
            v.push(Line::default());
            v.push(Line::from(
                format!(
                    "potential apps never uploaded to GitHub · {} (press u to filter, G to publish)",
                    unup_list.len()
                )
                .fg(theme::tone(Tone::Unborn))
                .bold(),
            ));
            for (r, s) in unup_list.iter().take(8) {
                let kind = if s.unborn {
                    "◊ unborn folder"
                } else if !s.has_remote {
                    "⌂ no remote"
                } else {
                    "⌂ unpushed branch"
                };
                v.push(Line::from(vec![
                    format!("  {kind:<17} ").fg(theme::tone(s.tone())),
                    rel(r).fg(TEXT).bold(),
                    format!(" ({} files)", s.changes.len()).fg(DIM),
                ]));
            }
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
