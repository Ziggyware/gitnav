#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Status {
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub changes: Vec<String>,
    pub error: Option<String>,
    /// Loaded from cache or awaiting a re-check.
    #[serde(skip)]
    pub stale: bool,
}

/// One-colour summary of a repo; declaration order is precedence (worst first).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tone {
    Error,
    Behind,
    Dirty,
    Ahead,
    Unpushed,
    Clean,
    Pending,
}

impl Tone {
    pub const ALL: [Tone; 7] = [
        Tone::Error,
        Tone::Behind,
        Tone::Dirty,
        Tone::Ahead,
        Tone::Unpushed,
        Tone::Clean,
        Tone::Pending,
    ];
}

pub fn tone(s: Option<&Status>) -> Tone {
    s.map_or(Tone::Pending, Status::tone)
}

impl Status {
    pub fn dirty(&self) -> bool {
        !self.changes.is_empty()
    }
    pub fn tone(&self) -> Tone {
        match () {
            _ if self.error.is_some() => Tone::Error,
            _ if self.behind > 0 => Tone::Behind,
            _ if self.dirty() => Tone::Dirty,
            _ if self.ahead > 0 => Tone::Ahead,
            _ if self.upstream.is_none() => Tone::Unpushed,
            _ => Tone::Clean,
        }
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
        assert_eq!(
            (s.branch.as_str(), s.upstream.as_deref()),
            ("main", Some("origin/main"))
        );
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
        assert_eq!(s.tone(), Tone::Behind);
    }
}
