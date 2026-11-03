//! The git tool the model can call: block parsing and shell-style argument splitting.

/// One `<git repo="…">args</git>` block.
#[derive(Debug, PartialEq)]
pub struct Call {
    pub repo: String,
    pub args: Vec<String>,
}

pub fn parse_calls(text: &str) -> Vec<Result<Call, String>> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("<git") {
        let tail = &rest[i..];
        let Some(close) = tail.find("</git>") else { break };
        let block = &tail[..close];
        rest = &tail[close + 6..];
        let Some((head, body)) = block.split_once('>') else {
            continue;
        };
        let repo = head
            .split_once("repo=\"")
            .and_then(|(_, r)| r.split_once('"'))
            .map(|(r, _)| r.to_owned());
        out.push(match repo {
            None => Err("missing repo=\"…\" attribute".into()),
            Some(repo) => split(body.trim()).map(|args| Call { repo, args }),
        });
    }
    out
}

/// Shell-style word splitting (quotes, backslash escapes) with no expansion of any kind.
pub fn split(s: &str) -> Result<Vec<String>, String> {
    let (mut out, mut cur, mut quote, mut any) = (Vec::new(), String::new(), None, false);
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (None, c) if c.is_whitespace() => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            (None, '\'' | '"') => (quote, any) = (Some(c), true),
            (Some(q), c) if c == q => quote = None,
            (Some('\''), c) => cur.push(c),
            (_, '\\') => {
                cur.push(chars.next().ok_or("dangling backslash")?);
                any = true;
            }
            (_, c) => {
                cur.push(c);
                any = true;
            }
        }
    }
    if quote.is_some() {
        return Err("unterminated quote".into());
    }
    if any {
        out.push(cur);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_blocks_and_words() {
        let t = r#"ok <git repo="dev/a b">log --format="%h %s" -3</git> then <git repo="x">commit -m "it's"</git>"#;
        let calls = parse_calls(t);
        assert_eq!(
            calls[0],
            Ok(Call {
                repo: "dev/a b".into(),
                args: vec!["log".into(), "--format=%h %s".into(), "-3".into()]
            })
        );
        assert_eq!(calls[1].as_ref().unwrap().args, ["commit", "-m", "it's"]);
        assert_eq!(split(r#"a "" 'b c' d\ e"#).unwrap(), ["a", "", "b c", "d e"]);
        assert!(split("'open").is_err());
        assert!(parse_calls("<git>status</git>")[0].is_err());
    }
}
