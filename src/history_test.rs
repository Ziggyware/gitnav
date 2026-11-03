use crate::{history::*, log_parse::*};

const DAY: i64 = 86_400;

#[test]
fn parses_commits_churn_authors_and_time() {
    // 2023-11-14 22:13:20 UTC = 16:13:20 at -0600, a Tuesday; local day 19675.
    let t = 1_700_000_000;
    let log = format!(
        "@{t} -0600\x1fana\x1ffeat: x\n\n3\t1\tsrc/a.rs\n-\t-\timg.png\n\
         @{} -0600\x1fbo\x1ffix: y\n\n10\t0\tsrc/a.rs\n@{} -0600\x1fana\x1finit\n",
        t - DAY,
        t - DAY - 15 * 3_600
    );
    let p = parse_pulse(&log, t + 60);
    assert_eq!((p.today, weekday(p.today)), (19_675, 1));
    assert_eq!((p.total(), p.days[DAYS - 1], p.days[DAYS - 2]), (3, 1, 2));
    assert_eq!(p.authors, [("ana".into(), 2), ("bo".into(), 1)]);
    assert_eq!(p.hot, [("src/a.rs".into(), 14), ("img.png".into(), 0)]);
    assert_eq!((p.churn[WEEKS - 1], p.others[WEEKS - 1]), (14, 1));
    // Monday 01:13 local is off-hours; the two 16:13 commits are not.
    assert_eq!(p.late.iter().sum::<u16>(), 1);
    assert_eq!(
        (p.recent.len(), p.recent[0].subject.as_str(), p.recent[0].churn),
        (3, "feat: x", 4)
    );
    let mut f = Pulse::default();
    f.add(&p, "svc");
    f.add(&p, "web");
    assert_eq!(
        (
            f.total(),
            f.authors[0].1,
            f.hot[0].0.as_str(),
            f.recent[0].repo.as_str()
        ),
        (6, 4, "svc/src/a.rs", "svc")
    );
}

#[test]
fn parses_numstat() {
    assert_eq!(
        parse_numstat("3\t1\ta b.rs\n-\t-\timg.png\n"),
        [("a b.rs".into(), 3, 1), ("img.png".into(), 0, 0)]
    );
}
