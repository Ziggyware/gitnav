//! Zero-friction GitHub repository provisioning and remote URL synthesis.
use serde_json::{Value, json};
use std::{env, path::Path, process::Command, time::Duration};

pub fn token() -> Option<String> {
    ["GITHUB_TOKEN", "GH_TOKEN"]
        .into_iter()
        .find_map(|k| env::var(k).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            let o = Command::new("gh").args(["auth", "token"]).output().ok()?;
            o.status
                .success()
                .then(|| String::from_utf8_lossy(&o.stdout).trim().to_owned())
                .filter(|s| !s.is_empty())
        })
}

/// Provisions a GitHub repository or resolves an existing one, returning its clone URL.
/// Spec accepts `"name"`, `"owner/name"`, `"public:name"`, `"private:name"`, or a full URL.
pub fn ensure_remote_repo(dir: &Path, spec: &str) -> Result<String, String> {
    let spec = spec.trim();
    if spec.starts_with("https://") || spec.starts_with("git@") || spec.starts_with("ssh://") {
        return Ok(spec.to_owned());
    }
    let (private, rest) = if let Some(r) = spec.strip_prefix("public:") {
        (false, r.trim())
    } else if let Some(r) = spec.strip_prefix("private:") {
        (true, r.trim())
    } else {
        (true, spec)
    };
    let default_name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "repo".into());
    let target = if rest.is_empty() {
        default_name.as_str()
    } else {
        rest
    };

    if let Some(tok) = token() {
        if let Ok(url) = create_via_api(&tok, target, private) {
            return Ok(url);
        }
    }

    // Fallback: GitHub CLI (`gh repo create`)
    let vis = if private { "--private" } else { "--public" };
    let o = Command::new("gh")
        .current_dir(dir)
        .args(["repo", "create", target, vis])
        .output();
    if let Ok(o) = o {
        if o.status.success() {
            let out = String::from_utf8_lossy(&o.stdout).trim().to_owned();
            if let Some(url) = out.lines().find(|l| l.starts_with("https://")) {
                return Ok(format!("{}.git", url.trim_end_matches(".git")));
            }
        }
    }

    if target.contains('/') {
        Ok(format!("https://github.com/{}.git", target.trim_end_matches(".git")))
    } else {
        Err("Set GITHUB_TOKEN / GH_TOKEN, authenticate `gh auth login`, or pass owner/repo or https:// URL".into())
    }
}

fn create_via_api(token: &str, target: &str, private: bool) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .http_status_as_error(false)
        .build()
        .into();

    let (_, repo_name) = target
        .split_once('/')
        .map_or((None, target), |(o, r)| (Some(o), r));

    let body = json!({
        "name": repo_name,
        "private": private,
        "auto_init": false
    })
    .to_string();

    let mut resp = agent
        .post("https://api.github.com/user/repos")
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "gitnav-elite")
        .content_type("application/json")
        .send(body)
        .map_err(|e| e.to_string())?;

    let status = resp.status().as_u16();
    let text = resp.body_mut().read_to_string().map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&text).unwrap_or_default();

    if (200..300).contains(&status) {
        if let Some(url) = v.get("clone_url").and_then(Value::as_str) {
            return Ok(url.to_owned());
        }
    }

    // HTTP 422 means the repository already exists on GitHub: resolve owner login and return URL so graft can bind it!
    if status == 422 {
        if target.contains('/') {
            return Ok(format!("https://github.com/{}.git", target.trim_end_matches(".git")));
        }
        if let Ok(mut u_resp) = agent
            .get("https://api.github.com/user")
            .header("Authorization", format!("Bearer {token}"))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "gitnav-elite")
            .call()
        {
            if let Ok(u_txt) = u_resp.body_mut().read_to_string() {
                if let Ok(uv) = serde_json::from_str::<Value>(&u_txt) {
                    if let Some(login) = uv.get("login").and_then(Value::as_str) {
                        return Ok(format!("https://github.com/{login}/{repo_name}.git"));
                    }
                }
            }
        }
    }

    Err(v
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("GitHub API call failed")
        .to_owned())
}
