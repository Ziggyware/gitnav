//! The system prompt: tool protocol plus a snapshot of every repo the model can reason over.
use crate::{app::App, theme};
use std::fmt::Write;

const PROTOCOL: &str = r#"You are the assistant inside gitnav, a terminal dashboard over every git repo and local project under one root.
Answer questions about the repos and carry out git work on them.
To run git, write a block (any number per reply):
<git repo="REPO">ARGS</git>
- REPO is a path from the snapshot below ("." is the root itself).
- ARGS are git arguments without the word git, split like a shell; no pipes, globs or other programs.
- Inspecting commands run at once. Anything that changes a repo or a remote is shown to the user, who approves or denies it.
- Unborn projects (no .git yet) can be initialized with <git repo="REPO">init -b main</git>.
- Disjoint histories between local folders and newly created GitHub repos can be unified with `merge origin/main --allow-unrelated-histories -s ours`.
- Never force-push, reset --hard, clean or delete branches unless the user explicitly asked for that.
- Results come back in <result> blocks. Keep going until you can answer.
- Final answers: concise plain text, no <git> blocks. Cite repo paths. Say what you changed.
"#;

impl App {
    pub(crate) fn system_prompt(&self) -> String {
        let mut s = format!("{PROTOCOL}\nroot: {}\n", self.root.display());
        s += "snapshot (repo | state | branch -> upstream | ahead/behind | uncommitted | last commit | commits in 16 weeks):\n";
        for (rel, st) in self.repos.iter().take(400) {
            let name = if rel.as_os_str().is_empty() {
                ".".into()
            } else {
                rel.display().to_string()
            };
            let p = self.pulses.get(rel);
            let (last, n) = p.map_or(("?".into(), 0), |p| (theme::age(self.now, p.last), p.total()));
            let _ = match st {
                None => writeln!(s, "{name} | not checked yet"),
                Some(st) => writeln!(
                    s,
                    "{name} | {:?} | {} -> {} | +{}/-{} | {} files | {last} ago | {n}",
                    st.tone(),
                    st.branch,
                    st.upstream.as_deref().unwrap_or(if st.unborn { "unborn" } else { "none" }),
                    st.ahead,
                    st.behind,
                    st.changes.len()
                ),
            };
        }
        if self.repos.len() > 400 {
            let _ = writeln!(s, "... {} more repos not listed", self.repos.len() - 400);
        }
        if let Some(e) = self.selected() {
            let sel = if e.rel.as_os_str().is_empty() {
                ".".into()
            } else {
                e.rel.display().to_string()
            };
            let _ = writeln!(
                s,
                "\nThe user has selected: {sel}{}",
                if e.repo { "" } else { " (a folder)" }
            );
        }
        s
    }
}
