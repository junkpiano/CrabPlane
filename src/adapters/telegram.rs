use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::engine::{Engine, ResultSink};
use crate::types::{Job, Message, Response};

pub struct Adapter {
    token: String,
    eng: Arc<dyn Engine>,
}

impl Adapter {
    pub fn new(token: String, eng: Arc<dyn Engine>) -> Self {
        Self { token, eng }
    }

    pub fn run(&self, stop: &AtomicBool) -> Result<(), String> {
        if self.token.is_empty() {
            return Err("TELEGRAM_BOT_TOKEN is empty".to_string());
        }

        let mut offset: i64 = 0;
        while !stop.load(Ordering::Relaxed) {
            let body = get_updates(&self.token, offset)?;
            let updates = parse_updates(&body);
            if updates.is_empty() {
                thread::sleep(Duration::from_millis(250));
                continue;
            }

            for u in updates {
                offset = (u.update_id + 1).max(offset);
                if u.text.is_empty() {
                    continue;
                }
                if is_help_command(&u.text) {
                    let _ = send_message(&self.token, u.chat_id, telegram_help_text());
                    continue;
                }
                if should_send_typing_status(&u.text) {
                    let _ = send_chat_action(&self.token, u.chat_id, "typing");
                }

                let resp = self.eng.handle(Message {
                    user_id: u.user_id,
                    channel: u.chat_id.to_string(),
                    text: u.text,
                    metadata: HashMap::new(),
                });
                if resp.text.is_empty() {
                    continue;
                }
                let _ = send_message(&self.token, u.chat_id, &resp.text);
            }
        }

        Ok(())
    }

    pub fn close(&self) -> Result<(), String> {
        Ok(())
    }
}

impl ResultSink for Adapter {
    fn deliver(&self, job: &Job, resp: &Response) -> Result<(), String> {
        if resp.text.is_empty() {
            return Ok(());
        }
        let chat_id = job
            .channel_id
            .parse::<i64>()
            .map_err(|_| format!("invalid telegram chat id: {}", job.channel_id))?;
        send_message(&self.token, chat_id, &resp.text)
    }
}

#[derive(Debug)]
struct TelegramUpdate {
    update_id: i64,
    chat_id: i64,
    user_id: String,
    text: String,
}

fn get_updates(token: &str, offset: i64) -> Result<String, String> {
    let url = format!(
        "https://api.telegram.org/bot{token}/getUpdates?timeout=25&offset={offset}&allowed_updates=%5B%22message%22%5D"
    );
    run_curl(["-sS", "--max-time", "30", &url])
}

fn send_message(token: &str, chat_id: i64, text: &str) -> Result<(), String> {
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    let chat = format!("chat_id={chat_id}");
    let text_arg = format!("text={text}");
    let _ = run_curl([
        "-sS",
        "--max-time",
        "30",
        "-X",
        "POST",
        &url,
        "-d",
        &chat,
        "--data-urlencode",
        &text_arg,
    ])?;
    Ok(())
}

fn send_chat_action(token: &str, chat_id: i64, action: &str) -> Result<(), String> {
    let url = format!("https://api.telegram.org/bot{token}/sendChatAction");
    let chat = format!("chat_id={chat_id}");
    let action_arg = format!("action={action}");
    let _ = run_curl([
        "-sS",
        "--max-time",
        "30",
        "-X",
        "POST",
        &url,
        "-d",
        &chat,
        "-d",
        &action_arg,
    ])?;
    Ok(())
}

fn should_send_typing_status(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    if t.starts_with("!ask ") {
        return true;
    }
    !t.starts_with('!')
}

fn run_curl<const N: usize>(args: [&str; N]) -> Result<String, String> {
    let out = Command::new("curl")
        .args(args)
        .output()
        .map_err(|e| format!("failed to execute curl: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("curl failed: {}", stderr.trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn parse_updates(body: &str) -> Vec<TelegramUpdate> {
    let mut out = Vec::new();
    let key = "\"update_id\":";
    let mut i = 0usize;
    while let Some(rel) = body[i..].find(key) {
        let start = i + rel;
        let next = body[start + key.len()..]
            .find(key)
            .map(|r| start + key.len() + r)
            .unwrap_or(body.len());
        let chunk = &body[start..next];
        i = next;

        let update_id = extract_i64_after(chunk, key).unwrap_or(0);
        let chat_id = extract_i64_after(chunk, "\"chat\":{\"id\":")
            .or_else(|| extract_i64_after(chunk, "\"chat\": {\"id\":"))
            .or_else(|| extract_i64_after(chunk, "\"id\":"))
            .unwrap_or(0);
        if chat_id == 0 {
            continue;
        }
        let user_id = extract_i64_after(chunk, "\"from\":{\"id\":")
            .or_else(|| extract_i64_after(chunk, "\"from\": {\"id\":"))
            .map(|v| v.to_string())
            .unwrap_or_else(|| "telegram".to_string());
        let text = extract_json_string_after(chunk, "\"text\":").unwrap_or_default();
        out.push(TelegramUpdate {
            update_id,
            chat_id,
            user_id,
            text,
        });
    }
    out
}

fn extract_i64_after(s: &str, marker: &str) -> Option<i64> {
    let idx = s.find(marker)?;
    let mut j = idx + marker.len();
    while let Some(c) = s.as_bytes().get(j) {
        if *c == b' ' {
            j += 1;
            continue;
        }
        break;
    }
    let mut k = j;
    if s.as_bytes().get(k) == Some(&b'-') {
        k += 1;
    }
    while let Some(c) = s.as_bytes().get(k) {
        if c.is_ascii_digit() {
            k += 1;
        } else {
            break;
        }
    }
    if k == j || (k == j + 1 && s.as_bytes().get(j) == Some(&b'-')) {
        return None;
    }
    s[j..k].parse::<i64>().ok()
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

fn is_help_command(text: &str) -> bool {
    let t = text.trim();
    t == "/help" || t.starts_with("/help@") || t.starts_with("/help ")
}

fn telegram_help_text() -> &'static str {
    "Crabplane commands:\n\
     /help - show this help\n\
     !ping - reply with pong\n\
     !echo <text> - echo back text\n\
     !ask <prompt> - run prompt via CRABPLANE_AI_BACKEND\n\
     Any non-command message is routed to !ask."
}
