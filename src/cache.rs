//! Last-known state per root, painted instantly as stale and revalidated by the scan.
use crate::{
    history::{Pulse, WEEKS},
    porcelain::Status,
};
use std::{
    collections::{BTreeMap, HashMap},
    env, fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
};

pub type Repos = BTreeMap<PathBuf, Option<Status>>;
pub type Pulses = HashMap<PathBuf, Pulse>;

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

pub fn load(root: &Path) -> (Repos, Pulses) {
    file(root)
        .and_then(|f| fs::read_to_string(f).ok())
        .map_or_else(Default::default, |t| decode(&t))
}

pub fn save(root: &Path, repos: &Repos, pulses: &Pulses) {
    if let Some(f) = file(root) {
        let _ = fs::create_dir_all(f.parent().unwrap());
        let _ = fs::write(f, encode(repos, pulses));
    }
}

fn encode(repos: &Repos, pulses: &Pulses) -> String {
    let clean = |v: &str| v.replace(['\n', F, L], " ");
    let opt = |v: &Option<String>| clean(v.as_deref().unwrap_or(""));
    let mut out = String::new();
    for (p, s) in repos.iter().filter_map(|(p, s)| Some((p, s.as_ref()?))) {
        let ch: Vec<String> = s.changes.iter().map(|c| clean(c)).collect();
        let pulse = pulses.get(p).map_or(String::new(), |u| {
            let w: Vec<String> = u.weeks.iter().map(u32::to_string).collect();
            format!("{}{L}{}", u.last, w.join(","))
        });
        let (path, br, up, err) = (
            clean(&p.to_string_lossy()),
            clean(&s.branch),
            opt(&s.upstream),
            opt(&s.error),
        );
        let (a, b, ch) = (s.ahead, s.behind, ch.join(&L.to_string()));
        out += &format!("{path}{F}{br}{F}{up}{F}{a}{F}{b}{F}{err}{F}{ch}{F}{pulse}\n");
    }
    out
}

fn decode_pulse(v: &str) -> Option<Pulse> {
    let (last, weeks) = v.split_once(L)?;
    let w: Vec<u32> = weeks
        .split(',')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    Some(Pulse {
        last: last.parse().ok()?,
        weeks: w.try_into().ok()?,
    })
}

fn decode(t: &str) -> (Repos, Pulses) {
    let opt = |v: &str| (!v.is_empty()).then(|| v.to_owned());
    let (mut repos, mut pulses) = (Repos::new(), Pulses::new());
    for l in t.lines() {
        let f: Vec<&str> = l.split(F).collect();
        let &[p, branch, up, ahead, behind, err, ch, pulse] = &f[..] else {
            continue;
        };
        let (Ok(ahead), Ok(behind)) = (ahead.parse(), behind.parse()) else {
            continue;
        };
        let changes = ch
            .split(L)
            .filter(|c| !c.is_empty())
            .map(Into::into)
            .collect();
        let (branch, upstream, error) = (branch.into(), opt(up), opt(err));
        let s = Status {
            branch,
            upstream,
            ahead,
            behind,
            changes,
            error,
            stale: true,
        };
        if let Some(u) = decode_pulse(pulse).filter(|u| u.weeks.len() == WEEKS) {
            pulses.insert(p.into(), u);
        }
        repos.insert(p.into(), Some(s));
    }
    (repos, pulses)
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
        let mut weeks = [0; WEEKS];
        weeks[3] = 7;
        let repos = Repos::from([("w/api".into(), Some(s)), ("pending".into(), None)]);
        let pulses = Pulses::from([("w/api".into(), Pulse { last: 42, weeks })]);
        let (back, bp) = decode(&encode(&repos, &pulses));
        let got = back[Path::new("w/api")].as_ref().unwrap();
        assert_eq!(back.len(), 1);
        assert!(got.stale);
        assert_eq!((got.ahead, got.behind, got.changes.len()), (3, 0, 2));
        assert_eq!(got.error.as_deref(), Some("bad thing"));
        assert_eq!(bp[Path::new("w/api")], Pulse { last: 42, weeks });
    }
}
