//! Last-known state per root: painted instantly as stale, then revalidated by the scan.
use crate::{
    history::{DAYS, Pulse, WEEKS},
    porcelain::Status,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    env, fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
};

pub type Repos = BTreeMap<PathBuf, Option<Status>>;
pub type Pulses = HashMap<PathBuf, Pulse>;

/// Bumped whenever the shape changes; older files are ignored, not misread.
const VERSION: u32 = 3;

#[derive(Serialize, Deserialize)]
struct Snapshot {
    version: u32,
    repos: Vec<(String, Status)>,
    pulses: Vec<(String, Pulse)>,
}

fn file(root: &Path) -> Option<PathBuf> {
    let base = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .or_else(|| env::var_os("HOME").map(|h| Path::new(&h).join(".cache")))?;
    let mut h = DefaultHasher::new();
    root.hash(&mut h);
    Some(base.join("gitnav").join(format!("{:016x}.json", h.finish())))
}

/// Series lengths are indexed directly by the UI, so a file that disagrees is dropped.
fn sound(p: &Pulse) -> bool {
    p.days.len() == DAYS && p.hours.len() == 24 && [p.churn.len(), p.others.len(), p.late.len()] == [WEEKS; 3]
}

fn decode(bytes: &[u8]) -> (Repos, Pulses) {
    let Some(s) = serde_json::from_slice::<Snapshot>(bytes)
        .ok()
        .filter(|s| s.version == VERSION)
    else {
        return Default::default();
    };
    let repos = s
        .repos
        .into_iter()
        .map(|(p, st)| (PathBuf::from(p), Some(Status { stale: true, ..st })))
        .collect();
    let pulses = s
        .pulses
        .into_iter()
        .filter(|(_, p)| sound(p))
        .map(|(k, p)| (PathBuf::from(k), p))
        .collect();
    (repos, pulses)
}

fn encode(repos: &Repos, pulses: &Pulses) -> Vec<u8> {
    let key = |p: &Path| p.to_string_lossy().into_owned();
    let repos = repos
        .iter()
        .filter_map(|(p, s)| Some((key(p), s.as_ref()?.clone())))
        .collect();
    let pulses = pulses.iter().map(|(p, u)| (key(p), u.clone())).collect();
    serde_json::to_vec(&Snapshot {
        version: VERSION,
        repos,
        pulses,
    })
    .unwrap_or_default()
}

pub fn load(root: &Path) -> (Repos, Pulses) {
    file(root)
        .and_then(|f| fs::read(f).ok())
        .map_or_else(Default::default, |b| decode(&b))
}

pub fn save(root: &Path, repos: &Repos, pulses: &Pulses) {
    if let Some(f) = file(root) {
        let _ = fs::create_dir_all(f.parent().unwrap());
        let _ = fs::write(f, encode(repos, pulses));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_as_stale_and_rejects_bad_shapes() {
        let s = Status {
            branch: "main".into(),
            ahead: 3,
            changes: vec!["?? x".into()],
            ..Default::default()
        };
        let mut u = Pulse {
            last: 42,
            burst: Some(0.5),
            authors: vec![("ana".into(), 3)],
            ..Default::default()
        };
        u.days[3] = 7;
        let repos = Repos::from([("w/api".into(), Some(s)), ("pending".into(), None)]);
        let (back, bp) = decode(&encode(&repos, &Pulses::from([("w/api".into(), u.clone())])));
        let got = back[Path::new("w/api")].as_ref().unwrap();
        assert_eq!(
            (back.len(), got.stale, got.ahead, &got.branch[..]),
            (1, true, 3, "main")
        );
        assert_eq!(bp[Path::new("w/api")], u);
        let bad = br#"{"version":3,"repos":[],"pulses":[["x",{"last":0,"today":0,"days":[1],"hours":[],"circ":[0,0],"burst":null,"churn":[],"others":[],"late":[],"authors":[],"hot":[],"recent":[]}]]}"#;
        assert!(decode(bad).1.is_empty());
        assert!(decode(b"not json").0.is_empty());
    }
}
