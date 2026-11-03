use crate::{
    app::{App, Entry},
    porcelain::{Tone, tone},
    theme::{self, *},
};
use ratatui::{prelude::*, widgets::*};

impl App {
    /// Every repo as a half-block pixel in tree order: the selection glows, the current folder
    /// is lit, everything else recedes. Overflow folds chunks into their worst tone.
    pub(crate) fn mosaic(&self, area: Rect, sel: Option<&Entry>) -> Paragraph<'static> {
        let (w, h) = (area.width as usize, area.height as usize);
        let px: Vec<(Tone, u8)> = self
            .repos
            .iter()
            .map(|(rel, s)| {
                let focus = if sel.is_some_and(|e| rel.starts_with(&e.rel)) {
                    2
                } else {
                    rel.starts_with(&self.cwd) as u8
                };
                (tone(s.as_ref()), focus)
            })
            .collect();
        let tw = tile(px.len(), w);
        let chunk = px.len().div_ceil(w * h * 2).max(1);
        let shade =
            |&(t, focus): &(Tone, u8)| lerp(theme::tone(t), BG, [0.82, 0.5, 0.0][focus as usize]);
        let cells: Vec<Option<Color>> = if tw > 1 {
            // Few repos: wide tiles with a one-cell gutter.
            px.iter()
                .flat_map(|p| (0..tw).map(move |i| (i + 1 < tw || tw < 3).then(|| shade(p))))
                .collect()
        } else {
            let fold = |c: &[(Tone, u8)]| {
                c.iter()
                    .fold((Tone::Pending, 0), |(t, f), &(u, g)| (t.min(u), f.max(g)))
            };
            px.chunks(chunk).map(|c| Some(shade(&fold(c)))).collect()
        };
        let lines = (0..h).map(|r| {
            let at = |i: usize| cells.get(i).copied().flatten();
            let spans = (0..w).map(|x| match (at(2 * r * w + x), at((2 * r + 1) * w + x)) {
                (Some(t), Some(b)) => "▀".fg(t).bg(b),
                (Some(t), None) => "▀".fg(t),
                (None, Some(b)) => "▄".fg(b),
                _ => " ".into(),
            });
            Line::from(spans.collect::<Vec<_>>())
        });
        Paragraph::new(lines.collect::<Vec<_>>())
    }
}

/// Mosaic tile width: grow tiles while the whole set fits on one text row.
pub fn tile(n: usize, w: usize) -> usize {
    (w * 2 / n.max(1)).clamp(1, 6)
}
