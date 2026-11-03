use crate::app::App;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Default, PartialEq)]
pub enum Sort {
    #[default]
    Name,
    Attention,
    Recent,
}

impl App {
    pub fn step(&mut self, d: isize) {
        let n = self.entries().len() as isize;
        let i = self.table.selected().unwrap_or(0) as isize + d;
        self.table.select(Some(i.clamp(0, (n - 1).max(0)) as usize));
    }

    pub fn select_rel(&mut self, rel: &Path) {
        let i = self.entries().iter().position(|e| e.rel == rel);
        self.table.select(Some(i.unwrap_or(0)));
    }

    /// Search hit: jump to it. Folder: drill in. Repo: yield its path to cd into.
    pub fn enter(&mut self) -> Option<PathBuf> {
        let e = self.selected()?;
        if self.search.take().is_some() {
            self.cwd = e.rel.parent().map(Path::to_path_buf).unwrap_or_default();
            self.select_rel(&e.rel);
        } else if e.repo {
            return Some(self.abs(&e.rel));
        } else {
            self.cwd = e.rel;
            self.table.select(Some(0));
        }
        None
    }

    pub fn up(&mut self) {
        let Some(parent) = self.cwd.parent().map(Path::to_path_buf) else {
            return;
        };
        let from = std::mem::replace(&mut self.cwd, parent);
        self.select_rel(&from);
    }

    /// `root.join("")` would append a separator.
    pub fn abs(&self, rel: &Path) -> PathBuf {
        if rel.as_os_str().is_empty() {
            self.root.clone()
        } else {
            self.root.join(rel)
        }
    }
}
