use std::env;
use std::process::Command;

use crate::tasks::{Task, TaskContext, TaskOutput};
use crate::types::TaskInput;

#[derive(Default)]
pub struct OpenAiTask;

impl OpenAiTask {
    pub fn new() -> Self {
        Self
    }
}

impl Task for OpenAiTask {
    fn name(&self) -> &'static str {
        "ask"
    }

    fn validate(&self, input: &TaskInput) -> Result<(), String> {
        match input {
            TaskInput::Text(t) if !t.trim().is_empty() => Ok(()),
            TaskInput::Text(_) => Err("prompt is empty".to_string()),
            _ => Err("invalid input".to_string()),
        }
    }

    fn run(&self, _ctx: &TaskContext, input: TaskInput) -> Result<TaskOutput, String> {
        let prompt = match input {
            TaskInput::Text(t) => t,
            _ => return Err("invalid input".to_string()),
        };

        let backend = env::var("CRABPLANE_AI_BACKEND").unwrap_or_else(|_| "codex".to_string());
        let out = match backend.trim().to_ascii_lowercase().as_str() {
            "openai" => ask_openai_api(&prompt),
            "anthropic" | "claude-api" | "claude_api" => ask_anthropic_api(&prompt),
            "codex" => ask_cli_backend(
                &prompt,
                "CRABPLANE_CODEX_CMD",
                "codex exec --skip-git-repo-check",
                "codex",
            ),
            "claude-code" | "claude_code" => ask_cli_backend(
                &prompt,
                "CRABPLANE_CLAUDE_CODE_CMD",
                "claude -p",
                "claude code",
            ),
            other => Err(format!(
                "unknown CRABPLANE_AI_BACKEND: {other} (expected: openai|anthropic|codex|claude-code)"
            )),
        }?;

        let trimmed = out.trim();
        if trimmed.is_empty() {
            return Err("backend returned empty output".to_string());
        }
        Ok(TaskOutput::Text(trimmed.to_string()))
    }
}

fn ask_openai_api(prompt: &str) -> Result<String, String> {
    let api_key = env::var("OPENAI_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Err("OPENAI_API_KEY is empty".to_string());
    }
    let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-5.3-codex".to_string());

    let body = format!(
        "{{\"model\":\"{}\",\"input\":\"{}\"}}",
        escape_json(&model),
        escape_json(prompt)
    );

    let auth = format!("Authorization: Bearer {api_key}");
    let out = Command::new("curl")
        .args([
            "-sS",
            "--max-time",
            "60",
            "https://api.openai.com/v1/responses",
            "-H",
            &auth,
            "-H",
            "Content-Type: application/json",
            "-d",
            &body,
        ])
        .output()
        .map_err(|e| format!("failed to execute curl: {e}"))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("openai request failed: {}", stderr.trim()));
    }

    let raw = String::from_utf8_lossy(&out.stdout);
    extract_first_text(&raw).ok_or_else(|| "openai response did not include text output".to_string())
}

fn ask_anthropic_api(prompt: &str) -> Result<String, String> {
    let api_key = env::var("ANTHROPIC_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Err("ANTHROPIC_API_KEY is empty".to_string());
    }
    let model = env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-3-5-sonnet-latest".to_string());

    let body = format!(
        "{{\"model\":\"{}\",\"max_tokens\":1024,\"messages\":[{{\"role\":\"user\",\"content\":\"{}\"}}]}}",
        escape_json(&model),
        escape_json(prompt)
    );

    let key_header = format!("x-api-key: {api_key}");
    let out = Command::new("curl")
        .args([
            "-sS",
            "--max-time",
            "60",
            "https://api.anthropic.com/v1/messages",
            "-H",
            &key_header,
            "-H",
            "anthropic-version: 2023-06-01",
            "-H",
            "content-type: application/json",
            "-d",
            &body,
        ])
        .output()
        .map_err(|e| format!("failed to execute curl: {e}"))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("claude request failed: {}", stderr.trim()));
    }

    let raw = String::from_utf8_lossy(&out.stdout);
    extract_first_text(&raw).ok_or_else(|| "claude response did not include text output".to_string())
}

fn ask_cli_backend(
    prompt: &str,
    cmd_var: &str,
    default_cmd: &str,
    label: &str,
) -> Result<String, String> {
    let cmd = env::var(cmd_var).unwrap_or_else(|_| default_cmd.to_string());
    let full = format!("{} '{}'", cmd, escape_single_quotes(prompt));
    let out = Command::new("sh")
        .args(["-lc", &full])
        .output()
        .map_err(|e| format!("failed to execute {label} command: {e}"))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let msg = stderr.trim();
        return Err(if msg.is_empty() {
            format!("{label} command failed with status {}", out.status)
        } else {
            format!("{label} command failed: {msg}")
        });
    }

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    if !stdout.trim().is_empty() {
        return Ok(stdout);
    }

    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !stderr.trim().is_empty() {
        return Ok(stderr);
    }

    Ok(String::new())
}

fn escape_single_quotes(s: &str) -> String {
    s.replace('\'', "'\"'\"'")
}

fn extract_first_text(body: &str) -> Option<String> {
    if let Some(text) = extract_json_string_after(body, "\"output_text\":") {
        return Some(text);
    }

    let mut i = 0usize;
    while let Some(rel) = body[i..].find("\"type\":\"output_text\"") {
        let start = i + rel;
        let text = extract_json_string_after(&body[start..], "\"text\":");
        if text.is_some() {
            return text;
        }
        i = start + "\"type\":\"output_text\"".len();
    }

    let mut j = 0usize;
    while let Some(rel) = body[j..].find("\"type\":\"text\"") {
        let start = j + rel;
        let text = extract_json_string_after(&body[start..], "\"text\":");
        if text.is_some() {
            return text;
        }
        j = start + "\"type\":\"text\"".len();
    }

    None
}

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

fn extract_json_string_after(s: &str, marker: &str) -> Option<String> {
    let idx = s.find(marker)?;
    let bytes = s.as_bytes();
    let mut i = idx + marker.len();
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b'"' {
        return None;
    }
    i += 1;
    let mut out = String::new();
    let mut esc = false;
    while i < bytes.len() {
        let b = bytes[i];
        if esc {
            match b {
                b'"' => out.push('"'),
                b'\\' => out.push('\\'),
                b'/' => out.push('/'),
                b'b' => out.push('\u{0008}'),
                b'f' => out.push('\u{000c}'),
                b'n' => out.push('\n'),
                b'r' => out.push('\r'),
                b't' => out.push('\t'),
                _ => out.push(b as char),
            }
            esc = false;
            i += 1;
            continue;
        }
        if b == b'\\' {
            esc = true;
            i += 1;
            continue;
        }
        if b == b'"' {
            return Some(out);
        }
        out.push(b as char);
        i += 1;
    }
    None
}
