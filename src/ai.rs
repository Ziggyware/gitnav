//! OpenAI-compatible chat client for nokey-router.
use serde_json::{Value, json};
use std::{env, fs, io, path::PathBuf, time::Duration};

const DEFAULT_URL: &str = "https://nokey-router.ziggyware.deno.net";
const DEFAULT_MODEL: &str = "free:smart";

#[derive(Clone)]
pub struct Config {
    pub url: String,
    pub key: Option<String>,
    pub model: String,
}

impl Config {
    pub fn from_env() -> Self {
        let var = |k| env::var(k).ok().filter(|v: &String| !v.trim().is_empty());
        Self {
            url: var("NOKEY_URL")
                .unwrap_or(DEFAULT_URL.into())
                .trim_end_matches('/')
                .to_owned(),
            key: var("ROUTER_API_KEY").or_else(|| {
                fs::read_to_string(key_path()?)
                    .ok()
                    .map(|k| k.trim().to_owned())
                    .filter(|k| !k.is_empty())
            }),
            model: var("NOKEY_MODEL").unwrap_or(DEFAULT_MODEL.into()),
        }
    }
}

/// The env var wins; otherwise the key the user typed into gitnav, kept in their config dir.
pub fn key_path() -> Option<PathBuf> {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("APPDATA").map(PathBuf::from))
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("gitnav").join("router_key"))
}

/// Owner-only file on Unix; on Windows the per-user profile directory already restricts access.
pub fn save_key(key: &str) -> io::Result<PathBuf> {
    let path = key_path().ok_or_else(|| io::Error::other("no config directory (set HOME)"))?;
    fs::create_dir_all(path.parent().unwrap())?;
    let mut o = fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
    io::Write::write_all(&mut o.open(&path)?, key.as_bytes())?;
    Ok(path)
}

/// One non-streaming completion; returns the assistant's text.
pub fn complete(cfg: &Config, messages: &[Value]) -> Result<String, String> {
    let key = cfg.key.as_deref().ok_or("no router key: type /key to enter it")?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(180)))
        .http_status_as_error(false)
        .build()
        .into();
    let body = json!({ "model": cfg.model, "messages": messages, "temperature": 0.2 }).to_string();
    let mut resp = agent
        .post(format!("{}/v1/chat/completions", cfg.url))
        .header("Authorization", format!("Bearer {key}"))
        .content_type("application/json")
        .send(body)
        .map_err(|e| format!("router unreachable: {e}"))?;
    let status = resp.status();
    let text = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("reading reply: {e}"))?;
    let v: Value = serde_json::from_str(&text)
        .map_err(|_| format!("HTTP {status}: {}", text.chars().take(200).collect::<String>()))?;
    if let Some(msg) = v.pointer("/error/message").and_then(Value::as_str) {
        return Err(format!("router: {msg}"));
    }
    v.pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("HTTP {status}: no message in reply"))
}
