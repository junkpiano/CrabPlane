use std::env;

use crate::tasks::{Task, TaskContext, TaskOutput};
use crate::types::TaskInput;

#[derive(Default)]
pub struct OnboardingTask;

impl OnboardingTask {
    pub fn new() -> Self {
        Self
    }
}

impl Task for OnboardingTask {
    fn name(&self) -> &'static str {
        "onboard"
    }

    fn validate(&self, input: &TaskInput) -> Result<(), String> {
        match input {
            TaskInput::Empty => Ok(()),
            TaskInput::Text(_) => Ok(()),
        }
    }

    fn run(&self, _ctx: &TaskContext, input: TaskInput) -> Result<TaskOutput, String> {
        let scope = match input {
            TaskInput::Empty => String::new(),
            TaskInput::Text(t) => t.trim().to_ascii_lowercase(),
        };

        let include_chat = scope.is_empty()
            || scope == "all"
            || scope.contains("chat")
            || scope.contains("tool");
        let include_ai =
            scope.is_empty() || scope == "all" || scope.contains("ai") || scope.contains("provider");

        if !scope.is_empty() && !include_chat && !include_ai {
            return Err("usage: !onboard [chat|ai|all]".to_string());
        }

        let mut lines = vec![
            "Crabplane onboarding".to_string(),
            "Use `!onboard chat`, `!onboard ai`, or `!onboard all`.".to_string(),
        ];

        if include_chat {
            append_chat_section(&mut lines);
        }
        if include_ai {
            append_ai_section(&mut lines);
        }

        Ok(TaskOutput::Text(lines.join("\n")))
    }
}

fn append_chat_section(lines: &mut Vec<String>) {
    lines.push(String::new());
    lines.push("Chat tools".to_string());

    if env_set("DISCORD_TOKEN") {
        lines.push("- discord: configured (`DISCORD_TOKEN` set)".to_string());
    } else {
        lines.push(
            "- discord: missing `DISCORD_TOKEN` (run `export DISCORD_TOKEN=\"...\"` then `cargo run -- --mode=discord`)".to_string(),
        );
    }

    if env_set("TELEGRAM_BOT_TOKEN") {
        lines.push("- telegram: configured (`TELEGRAM_BOT_TOKEN` set)".to_string());
    } else {
        lines.push(
            "- telegram: missing `TELEGRAM_BOT_TOKEN` (run `export TELEGRAM_BOT_TOKEN=\"...\"` then `cargo run -- --mode=telegram`)".to_string(),
        );
    }

    let wa_ready =
        env_set("TWILIO_ACCOUNT_SID") && env_set("TWILIO_AUTH_TOKEN") && env_set("TWILIO_WHATSAPP_NUMBER");
    if wa_ready {
        lines.push(
            "- whatsapp: configured (`TWILIO_ACCOUNT_SID`, `TWILIO_AUTH_TOKEN`, `TWILIO_WHATSAPP_NUMBER` set)"
                .to_string(),
        );
    } else {
        let mut missing = Vec::new();
        if !env_set("TWILIO_ACCOUNT_SID") {
            missing.push("TWILIO_ACCOUNT_SID");
        }
        if !env_set("TWILIO_AUTH_TOKEN") {
            missing.push("TWILIO_AUTH_TOKEN");
        }
        if !env_set("TWILIO_WHATSAPP_NUMBER") {
            missing.push("TWILIO_WHATSAPP_NUMBER");
        }
        lines.push(format!(
            "- whatsapp: missing {} (run `cargo run -- --mode=whatsapp` after setting them)",
            missing.join(", ")
        ));
    }
}

fn append_ai_section(lines: &mut Vec<String>) {
    lines.push(String::new());
    lines.push("AI providers".to_string());

    let backend = env::var("CRABPLANE_AI_BACKEND").unwrap_or_else(|_| "codex".to_string());
    let selected = backend.trim().to_ascii_lowercase();
    lines.push(format!("- selected backend: `{}`", backend.trim()));
    lines.push("- supported backends: `openai`, `openai-codex-api`, `anthropic`, `codex`, `claude-code`".to_string());

    match selected.as_str() {
        "openai" => {
            lines.push(req_line("OPENAI_API_KEY", "required for OpenAI Responses API"));
            lines.push(opt_line(
                "OPENAI_MODEL",
                "optional model override (default: `gpt-5.3-codex`)",
            ));
        }
        "openai-codex-api" | "openai_codex_api" | "codex-api" | "codex_api" => {
            lines.push(req_line(
                "OPENAI_API_KEY",
                "required for OpenAI Codex API backend",
            ));
            lines.push(opt_line(
                "OPENAI_CODEX_MODEL",
                "optional model override (default: `gpt-5.3-codex`)",
            ));
        }
        "anthropic" | "claude-api" | "claude_api" => {
            lines.push(req_line(
                "ANTHROPIC_API_KEY",
                "required for Anthropic Messages API",
            ));
            lines.push(opt_line(
                "ANTHROPIC_MODEL",
                "optional model override (default: `claude-3-5-sonnet-latest`)",
            ));
        }
        "codex" => {
            lines.push(opt_line(
                "CRABPLANE_CODEX_CMD",
                "optional codex CLI command (default: `codex exec --skip-git-repo-check`)",
            ));
        }
        "claude-code" | "claude_code" => {
            lines.push(opt_line(
                "CRABPLANE_CLAUDE_CODE_CMD",
                "optional claude CLI command (default: `claude -p`)",
            ));
        }
        _ => {
            lines.push(
                "- warning: unknown `CRABPLANE_AI_BACKEND`; set one of the supported values above"
                    .to_string(),
            );
        }
    }
}

fn req_line(key: &str, detail: &str) -> String {
    if env_set(key) {
        format!("- `{key}`: configured ({detail})")
    } else {
        format!("- `{key}`: missing ({detail})")
    }
}

fn opt_line(key: &str, detail: &str) -> String {
    if env_set(key) {
        format!("- `{key}`: set ({detail})")
    } else {
        format!("- `{key}`: not set ({detail})")
    }
}

fn env_set(key: &str) -> bool {
    env::var(key)
        .ok()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}
