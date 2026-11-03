//! Last-known statuses per root, painted instantly as stale and revalidated by the scan.
use crate::porcelain::Status;
use std::{
    collections::BTreeMap,
    env, fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
};

type Repos = BTreeMap<PathBuf, Option<Status>>;

const F: char = '\x1f';
const L: char = '\x1e';

fn file(root: &Path) -> Option<PathBuf> {
    let base = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .or_else(|| env::var_os("HOME").map(|h| Path::new(&h).join(".cache")))?;
    let mut h = DefaultHasher::new();
    root.hash(&mut h);
    Some(base.join("gitnav").join(format!("{:016x}", h.finish())))
}

pub fn load(root: &Path) -> Repos {
    file(root)
        .and_then(|f| fs::read_to_string(f).ok())
        .map_or_else(Repos::new, |t| decode(&t))
}

pub fn save(root: &Path, repos: &Repos) {
    if let Some(f) = file(root) {
        let _ = fs::create_dir_all(f.parent().unwrap());
        let _ = fs::write(f, encode(repos));
    }
}

fn encode(repos: &Repos) -> String {
    let clean = |v: &str| v.replace(['\n', F, L], " ");
    let opt = |v: &Option<String>| clean(v.as_deref().unwrap_or(""));
    repos
        .iter()
        .filter_map(|(p, s)| Some((p, s.as_ref()?)))
        .map(|(p, s)| {
            let ch: Vec<String> = s.changes.iter().map(|c| clean(c)).collect();
            let (path, br, up, err) = (
                clean(&p.to_string_lossy()),
                clean(&s.branch),
                opt(&s.upstream),
                opt(&s.error),
            );
            format!(
                "{path}{F}{br}{F}{up}{F}{}{F}{}{F}{err}{F}{}\n",
                s.ahead,
                s.behind,
                ch.join(&L.to_string())
            )
        })
        .collect()
}

fn decode(t: &str) -> Repos {
    let opt = |v: &str| (!v.is_empty()).then(|| v.to_owned());
    t.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split(F).collect();
            let &[p, branch, up, ahead, behind, err, ch] = &f[..] else {
                return None;
            };
            let s = Status {
                branch: branch.into(),
                upstream: opt(up),
                ahead: ahead.parse().ok()?,
                behind: behind.parse().ok()?,
                changes: ch
                    .split(L)
                    .filter(|c| !c.is_empty())
                    .map(Into::into)
                    .collect(),
                error: opt(err),
                stale: true,
            };
            Some((PathBuf::from(p), Some(s)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_as_stale() {
        let s = Status {
            branch: "main".into(),
            upstream: Some("origin/main".into()),
            ahead: 3,
            changes: vec![".M a b.rs".into(), "?? x".into()],
            error: Some("bad\nthing".into()),
            ..Default::default()
        };
        let repos = Repos::from([("w/api".into(), Some(s)), ("pending".into(), None)]);
        let back = decode(&encode(&repos));
        let got = back[Path::new("w/api")].as_ref().unwrap();
        assert_eq!(back.len(), 1);
        assert!(got.stale);
        assert_eq!((got.ahead, got.behind, got.changes.len()), (3, 0, 2));
        assert_eq!(got.error.as_deref(), Some("bad thing"));
    }
}
