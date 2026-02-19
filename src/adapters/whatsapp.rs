use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::engine::{Engine, ResultSink};
use crate::types::{Job, Message, Response};

pub struct Adapter {
    account_sid: String,
    auth_token: String,
    from_number: String,
    eng: Arc<dyn Engine>,
}

impl Adapter {
    pub fn new(account_sid: String, auth_token: String, from_number: String, eng: Arc<dyn Engine>) -> Self {
        Self { account_sid, auth_token, from_number, eng }
    }

    pub fn run(&self, stop: &AtomicBool) -> Result<(), String> {
        if self.account_sid.is_empty() {
            return Err("TWILIO_ACCOUNT_SID is empty".to_string());
        }
        if self.auth_token.is_empty() {
            return Err("TWILIO_AUTH_TOKEN is empty".to_string());
        }
        if self.from_number.is_empty() {
            return Err("TWILIO_WHATSAPP_NUMBER is empty".to_string());
        }

        let mut last_sid = String::new();
        while !stop.load(Ordering::Relaxed) {
            let body = get_messages(&self.account_sid, &self.auth_token, &self.from_number)?;
            let messages = parse_messages(&body);
            if messages.is_empty() {
                thread::sleep(Duration::from_millis(250));
                continue;
            }

            for msg in messages {
                // Skip already processed messages
                if !last_sid.is_empty() && msg.sid <= last_sid {
                    continue;
                }
                last_sid = msg.sid.clone();

                if msg.body.is_empty() {
                    continue;
                }
                if is_help_command(&msg.body) {
                    let _ = send_message(&self.account_sid, &self.auth_token, &self.from_number, &msg.from, whatsapp_help_text());
                    continue;
                }

                // For WhatsApp, we don't have a typing indicator API like Telegram
                // Twilio doesn't support WhatsApp typing indicators

                let resp = self.eng.handle(Message {
                    user_id: msg.from.clone(),
                    channel: msg.from.clone(),
                    text: msg.body,
                    metadata: HashMap::new(),
                });
                if resp.text.is_empty() {
                    continue;
                }
                let _ = send_message(&self.account_sid, &self.auth_token, &self.from_number, &msg.from, &resp.text);
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
        // channel_id should be the WhatsApp number in E.164 format
        send_message(&self.account_sid, &self.auth_token, &self.from_number, &job.channel_id, &resp.text)
    }
}

#[derive(Debug, Clone)]
struct WhatsAppMessage {
    sid: String,
    from: String,
    body: String,
}

fn get_messages(account_sid: &str, auth_token: &str, from_number: &str) -> Result<String, String> {
    let url = format!(
        "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json?To={}&PageSize=20",
        account_sid,
        urlencoded(from_number)
    );
    let auth = format!("{}:{}", account_sid, auth_token);
    run_curl(["-sS", "--max-time", "30", "-u", &auth, &url])
}

fn send_message(account_sid: &str, auth_token: &str, from_number: &str, to_number: &str, text: &str) -> Result<(), String> {
    let url = format!(
        "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
        account_sid
    );
    let auth = format!("{}:{}", account_sid, auth_token);

    // Format numbers with whatsapp: prefix
    let from_arg = format!("From=whatsapp:{}", from_number);
    let to_arg = format!("To=whatsapp:{}", to_number);
    let body_arg = format!("Body={}", text);

    let _ = run_curl([
        "-sS",
        "--max-time",
        "30",
        "-u",
        &auth,
        "-X",
        "POST",
        &url,
        "-d",
        &from_arg,
        "-d",
        &to_arg,
        "--data-urlencode",
        &body_arg,
    ])?;
    Ok(())
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

fn parse_messages(body: &str) -> Vec<WhatsAppMessage> {
    let mut out = Vec::new();
    let key = "\"sid\":";
    let mut i = 0usize;
    while let Some(rel) = body[i..].find(key) {
        let start = i + rel;
        let next = body[start + key.len()..]
            .find(key)
            .map(|r| start + key.len() + r)
            .unwrap_or(body.len());
        let chunk = &body[start..next];
        i = next;

        let sid = extract_json_string_after(chunk, key).unwrap_or_default();
        if sid.is_empty() {
            continue;
        }

        // Parse from number (remove whatsapp: prefix if present)
        let from = extract_json_string_after(chunk, "\"from\":")
            .unwrap_or_default()
            .trim_start_matches("whatsapp:")
            .to_string();
        if from.is_empty() {
            continue;
        }

        // Check if this is an inbound message (direction: "inbound")
        let direction = extract_json_string_after(chunk, "\"direction\":").unwrap_or_default();
        if direction != "inbound" {
            continue;
        }

        let body_text = extract_json_string_after(chunk, "\"body\":").unwrap_or_default();

        out.push(WhatsAppMessage {
            sid,
            from,
            body: body_text,
        });
    }

    // Reverse to get oldest messages first
    out.reverse();
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

fn is_help_command(text: &str) -> bool {
    let t = text.trim();
    t == "/help" || t.starts_with("/help ") || t == "!help" || t.starts_with("!help ")
}

fn whatsapp_help_text() -> &'static str {
    "Crabplane commands:\n\
     /help - show this help\n\
     !ping - reply with pong\n\
     !echo <text> - echo back text\n\
     !ask <prompt> - run prompt via CRABPLANE_AI_BACKEND\n\
     Any non-command message is routed to !ask."
}

fn urlencoded(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                c.to_string()
            } else {
                format!("%{:02X}", c as u8)
            }
        })
        .collect()
}
