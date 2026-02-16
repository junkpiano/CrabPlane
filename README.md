# ClawPlane (v0)

ClawPlane is a personal infrastructure control plane framework.

v0 includes:
- Transport-agnostic core engine
- In-memory job queue + worker pool
- Task registry + simple prefix router
- CLI adapter
- Discord adapter (discordgo)

## Commands (v0)

- `!ping` -> `pong`
- `!echo <text>` -> echoes text

## Run (CLI)

```bash
mise install
export CRABPLANE_CONCURRENCY=4
mise exec -- go run ./cmd/crabplane -mode=cli
```

## Run (Daemon)

```bash
mise install
export CRABPLANE_CONCURRENCY=4
mise exec -- go run ./cmd/crabplane -mode=daemon
```

## Run (Discord)

```bash
mise install
export DISCORD_TOKEN="YOUR_TOKEN"
export CRABPLANE_CONCURRENCY=4
mise exec -- go run ./cmd/crabplane -mode=discord
```
=======
- Discord adapter
=======
>>>>>>> ac0b840 (feat: project rename)
- Telegram adapter
- Discord adapter stub (token check only; runtime not implemented in this Rust port)
- Pluggable AI backend for default chat execution (`openai`, `anthropic`, `codex`, `claude-code`)

## Commands (v0)

- `/help` (Telegram only) -> show bot help
- `!ping` -> `pong`
- `!echo <text>` -> echoes text
- `!ask <prompt>` -> sends prompt to backend selected by `CRABPLANE_AI_BACKEND`
- Any other non-empty message -> sent to backend selected by `CRABPLANE_AI_BACKEND`

## Rust Setup

### Prerequisites

- Rust toolchain (stable) via `rustup`
- `curl` installed (required by Telegram adapter, OpenAI task, and Anthropic task)
- Optional local backend CLI tools:
  - `codex` for `CRABPLANE_AI_BACKEND=codex`
  - `claude` for `CRABPLANE_AI_BACKEND=claude-code`

### Build / Check

```bash
cargo check
cargo build
```

### Run (auto mode)

```bash
export CRABPLANE_CONCURRENCY=4
cargo run
```

`auto` selects mode in this order:
1. `discord` when `DISCORD_TOKEN` is set
2. `telegram` when `TELEGRAM_BOT_TOKEN` is set
3. `cli` when stdin is a terminal
4. `daemon` otherwise

### Run (CLI)

```bash
export CRABPLANE_CONCURRENCY=4
export CRABPLANE_AI_BACKEND="codex" # openai|anthropic|codex|claude-code
export OPENAI_API_KEY="YOUR_OPENAI_API_KEY" # required when backend=openai
export ANTHROPIC_API_KEY="YOUR_ANTHROPIC_API_KEY" # required when backend=anthropic
cargo run -- --mode=cli
```

### Run (Daemon)

```bash
export CRABPLANE_CONCURRENCY=4
cargo run -- --mode=daemon
```

### Run (Discord)

```bash
export DISCORD_TOKEN="YOUR_TOKEN"
export CRABPLANE_CONCURRENCY=4
cargo run -- --mode=discord
```

Current status: the Rust Discord adapter is a stub and returns a not-implemented error.

### Run (Telegram)

```bash
export TELEGRAM_BOT_TOKEN="YOUR_BOT_TOKEN"
export CRABPLANE_AI_BACKEND="codex" # openai|anthropic|codex|claude-code
export OPENAI_API_KEY="YOUR_OPENAI_API_KEY" # required when backend=openai
export ANTHROPIC_API_KEY="YOUR_ANTHROPIC_API_KEY" # required when backend=anthropic
export CRABPLANE_CONCURRENCY=4
cargo run -- --mode=telegram
```

## AI Backend Configuration

- `CRABPLANE_AI_BACKEND` (optional, default: `codex`)
- `CRABPLANE_CODEX_CMD` (optional, default: `codex exec --skip-git-repo-check`)
- `CRABPLANE_CLAUDE_CODE_CMD` (optional, default: `claude -p`)

## OpenAI API Configuration

- `OPENAI_API_KEY` (required when backend is `openai`)
- `OPENAI_MODEL` (optional, default: `gpt-5.3-codex`)

## Anthropic API Configuration

- `ANTHROPIC_API_KEY` (required when backend is `anthropic`)
- `ANTHROPIC_MODEL` (optional, default: `claude-3-5-sonnet-latest`)

## Runtime Flags

- `--mode=auto|cli|discord|telegram|daemon` (default: `auto`)
- `--queue-size=128` (default: `128`)
- `--shutdown-timeout=10s` (examples: `500ms`, `10s`, `1m`)
