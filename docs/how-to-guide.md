# ClawPlane How-To Guide

This guide explains how to run ClawPlane (Rust port), especially with the Telegram adapter.

## Prerequisites

- Rust toolchain with `cargo` installed
- `curl` installed (required by the Telegram adapter, OpenAI task, and Claude task implementation)
- Optional local backend CLI tools (`codex` or `claude`) when using `CRABPLANE_AI_BACKEND=codex|claude-code`
- A Telegram bot token from BotFather if using Telegram mode

## 1. Build and check

```bash
cargo check
```

## 2. Run in CLI mode

Use this for local testing without external services.

```bash
export CRABPLANE_CONCURRENCY=4
export CRABPLANE_AI_BACKEND="codex" # openai|anthropic|codex|claude-code
cargo run -- -mode=cli
```

Try commands:

- `!ping`
- `!echo hello`
- `!ask explain rust ownership in one sentence`
- `what is clawplane?` (non-command messages are routed to backend from `CRABPLANE_AI_BACKEND`)

## 3. Run in Telegram mode

### 3.1 Create a bot token

1. Open Telegram and message `@BotFather`.
2. Run `/newbot`.
3. Follow prompts and copy the token.

### 3.2 Start ClawPlane

```bash
export TELEGRAM_BOT_TOKEN="YOUR_BOT_TOKEN"
export CRABPLANE_AI_BACKEND="codex" # openai|anthropic|codex|claude-code
export OPENAI_API_KEY="YOUR_OPENAI_API_KEY" # required when backend=openai
export ANTHROPIC_API_KEY="YOUR_ANTHROPIC_API_KEY" # required when backend=anthropic
export CRABPLANE_CONCURRENCY=4
cargo run -- -mode=telegram
```

### 3.3 Test in chat

Message your bot with:

- `/help` -> shows available commands
- `!ping` -> `working...`, then `pong`
- `!echo hello` -> `working...`, then `hello`
- `!ask what is clawplane?` -> typing status (when supported), then `<model output>`
- `hello bot` -> typing status (when supported), then `<model output>`

## 4. Run in daemon mode

No chat transport; results are emitted to logs.

```bash
export CRABPLANE_CONCURRENCY=4
cargo run -- -mode=daemon
```

## 5. Mode selection behavior

If you run with `-mode=auto`:

1. Uses `discord` if `DISCORD_TOKEN` is set.
2. Else uses `telegram` if `TELEGRAM_BOT_TOKEN` is set.
3. Else uses `cli` if stdin is interactive.
4. Else uses `daemon`.

## 6. Troubleshooting

- `TELEGRAM_BOT_TOKEN is empty`: set the env var before running.
- `failed to execute curl`: install `curl` and ensure it is on your `PATH`.
- `OPENAI_API_KEY is empty`: set the env var before running `!ask`.
- `ANTHROPIC_API_KEY is empty`: set the env var when `CRABPLANE_AI_BACKEND=anthropic`.
- `codex command failed`: ensure `codex` is installed, or set `CRABPLANE_CODEX_CMD`.
- `claude code command failed`: ensure `claude` is installed, or set `CRABPLANE_CLAUDE_CODE_CMD`.
- No bot replies:
  - Verify token is valid.
  - Ensure your bot has received at least one message from you.
  - Check process logs for curl/network errors.
