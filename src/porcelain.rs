#[derive(Debug, Default, PartialEq)]
pub struct Status {
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub changes: Vec<String>,
    pub error: Option<String>,
    /// Loaded from cache, not yet revalidated.
    pub stale: bool,
}

impl Status {
    pub fn dirty(&self) -> bool {
        !self.changes.is_empty()
    }
    /// Anything that makes local differ from (or be unknown to) the remote.
    pub fn drift(&self) -> bool {
        self.dirty()
            || self.ahead + self.behind > 0
            || self.upstream.is_none()
            || self.error.is_some()
    }
}

pub fn parse(porcelain: &str) -> Status {
    let mut s = Status::default();
    for line in porcelain.lines() {
        if let Some(h) = line.strip_prefix("# branch.head ") {
            s.branch = h.to_owned();
        } else if let Some(u) = line.strip_prefix("# branch.upstream ") {
            s.upstream = Some(u.to_owned());
        } else if let Some(ab) = line.strip_prefix("# branch.ab ") {
            let mut it = ab.split(' ').map(|n| n[1..].parse().unwrap_or(0));
            (s.ahead, s.behind) = (it.next().unwrap_or(0), it.next().unwrap_or(0));
        } else if !line.starts_with('#') && line.len() > 2 {
            s.changes.push(change(line));
        }
    }
    s
}

/// porcelain v2 entry -> "XY path"; field count before the path differs per entry kind.
fn change(line: &str) -> String {
    let n = match &line[..1] {
        "1" => 8,
        "2" => 9,
        "u" => 10,
        k => return format!("{k}{k} {}", &line[2..]),
    };
    let f: Vec<&str> = line[2..].splitn(n, ' ').collect();
    match (f.first(), f.get(n - 1)) {
        (Some(xy), Some(p)) => format!("{xy} {}", p.replace('\t', " <- ")),
        _ => line.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_porcelain_v2() {
        let s = parse(
            "# branch.oid abc\n# branch.head main\n# branch.upstream origin/main\n# branch.ab +2 -1\n\
             1 .M N... 100644 100644 100644 a b src/my file.rs\n\
             2 R. N... 100644 100644 100644 a b R100 new.rs\told.rs\n\
             u UU N... 100644 100644 100644 100644 a b c conflict.rs\n\
             ? notes.txt\n",
        );
        assert_eq!(s.branch, "main");
        assert_eq!(s.upstream.as_deref(), Some("origin/main"));
        assert_eq!((s.ahead, s.behind), (2, 1));
        assert_eq!(
            s.changes,
            [
                ".M src/my file.rs",
                "R. new.rs <- old.rs",
                "UU conflict.rs",
                "?? notes.txt"
            ]
        );
        assert!(s.drift());
    }
}
