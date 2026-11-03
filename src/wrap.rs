/// Greedy word wrap by character count; explicit newlines are kept.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(8);
    let mut out = Vec::new();
    for para in text.lines() {
        let mut line = String::new();
        for word in para.split(' ') {
            let (lw, ww) = (line.chars().count(), word.chars().count());
            if lw > 0 && lw + 1 + ww > width {
                out.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            let mut w: Vec<char> = word.chars().collect();
            while w.len() > width {
                out.push(w.drain(..width).collect());
            }
            line.extend(w);
        }
        out.push(line);
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn wraps_words_and_long_tokens() {
        assert_eq!(super::wrap("aa bb cc", 8), ["aa bb cc"]);
        assert_eq!(super::wrap("aaaa bbbb cccc", 9), ["aaaa bbbb", "cccc"]);
        assert_eq!(super::wrap("abcdefghijkl", 8), ["abcdefgh", "ijkl"]);
        assert_eq!(super::wrap("a\nb", 8), ["a", "b"]);
    }
}
