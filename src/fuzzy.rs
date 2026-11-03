/// Subsequence match; consecutive runs and segment starts score, long paths pay a little.
pub fn fuzzy(q: &str, s: &str) -> Option<i32> {
    let s: Vec<char> = s.to_lowercase().chars().collect();
    let (mut score, mut i) = (0, 0);
    for c in q.to_lowercase().chars() {
        let j = (i..s.len()).find(|&j| s[j] == c)?;
        let run = j > 0 && i == j && score > 0;
        let start = j == 0 || matches!(s[j - 1], '/' | '\\' | '-' | '_' | '.' | ' ');
        score += 1 + 8 * run as i32 + 6 * start as i32;
        i = j + 1;
    }
    Some(score * 4 - s.len() as i32 / 4)
}

#[cfg(test)]
mod tests {
    use super::fuzzy;

    #[test]
    fn fuzzy_ranks_segments_and_runs() {
        assert!(fuzzy("api", "work/api/dirty") > fuzzy("api", "a/p/i"));
        assert!(fuzzy("wad", "work/api/dirty").is_some());
        assert!(fuzzy("zz", "work/api").is_none());
        assert!(fuzzy("api", "api") > fuzzy("api", "deep/nested/api"));
    }
}
