use crate::{
    app::{App, Entry},
    encode::*,
    history::Commit,
    theme::{self, *},
};
use ratatui::{prelude::*, widgets::*};

impl App {
    /// Newest commits under the selection (a repo, or every repo in a folder): who, where, how big, what.
    pub(crate) fn feed(&self, f: &mut Frame, area: Rect, scope: &Entry) {
        let mut all: Vec<Commit> = self
            .under(&scope.rel)
            .filter_map(|r| Some((r, self.pulses.get(r)?)))
            .flat_map(|(r, p)| {
                let repo = r.strip_prefix(&self.cwd).unwrap_or(r).display().to_string();
                p.recent.iter().map(move |c| Commit {
                    repo: repo.clone(),
                    ..c.clone()
                })
            })
            .collect();
        all.sort_by_key(|c| std::cmp::Reverse(c.t));
        let repos = all
            .iter()
            .map(|c| &c.repo)
            .collect::<std::collections::HashSet<_>>()
            .len();
        let title = Line::from(vec![
            " recent commits ".fg(ACCENT).bold(),
            format!("· {} · {repos} repo{} ", scope.name, if repos == 1 { "" } else { "s" }).fg(DIM),
        ]);
        let key = Line::from(
            if scope.repo {
                " age · author · lines · subject "
            } else {
                " age · repo · author · lines · subject "
            }
            .fg(DIM),
        );
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(COLD))
            .title(title)
            .title_bottom(key);
        let h = block.inner(area).height as usize;
        let rw = all
            .iter()
            .take(h)
            .map(|c| c.repo.chars().count())
            .max()
            .unwrap_or(4)
            .min(22);
        let aw = all
            .iter()
            .take(h)
            .map(|c| c.author.chars().count())
            .max()
            .unwrap_or(4)
            .min(12);
        let cut = |s: &str, n: usize| format!("{:<n$}", s.chars().take(n).collect::<String>());
        let lines: Vec<Line> = all
            .iter()
            .take(h)
            .map(|c| {
                Line::from(vec![
                    format!("{:>4} ", theme::age(self.now, c.t)).fg(heat(self.now, c.t)),
                    if scope.repo {
                        "".into()
                    } else {
                        format!("{} ", cut(&c.repo, rw)).fg(ACCENT)
                    },
                    cut(&c.author, aw).fg(self.person(&c.author)),
                    " ".into(),
                    churn_bar(c.churn, 5),
                    format!("{:<w$}", "", w = 5 - churn_bar(c.churn, 5).content.chars().count()).into(),
                    format!("{:>5} ", kilo(c.churn)).fg(DIM),
                    c.subject.clone().fg(TEXT),
                ])
            })
            .collect();
        f.render_widget(Paragraph::new(lines).block(block), area);
    }
}
