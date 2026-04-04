# Claude Sync

Claude Code 설정을 GitHub를 통해 여러 디바이스에서 싱크하는 도구.

## Features

- **GUI + CLI** — Tauri v2 데스크톱 앱과 CLI 동시 지원
- **Secret Masking** — API 키 구조는 싱크하되 값은 자동 마스킹
- **Selective Skill Sync** — 스킬별 개별 Push/Pull 선택 가능
- **Snapshot & Restore** — Pull 전 자동 백업, 언제든 복원
- **Cross-platform** — Windows, macOS, Linux 지원

## What Gets Synced

| Target | Default |
|--------|---------|
| `CLAUDE.md` | Always |
| `settings.json` (secrets masked) | Always |
| `.mcp.json` (secrets masked) | Always |
| `rules/`, `commands/`, `agents/` | Always |
| `hooks/` | Always |
| `skills/` (all installed) | ON |
| `teams/` | ON |
| `memory/` | OFF (user toggle) |

**Never synced:** `.credentials.json`, `projects/`, `plugins/`, `sessions/`, `history.jsonl`, caches

## Architecture

```
claude-sync/
├── crates/
│   ├── claude-sync-core/    # Pure Rust library (no Tauri dependency)
│   ├── claude-sync-cli/     # CLI binary (cargo install)
│   └── claude-sync-gui/     # Tauri v2 desktop app
```

- `claude-sync-core` — config, discovery, secret masking, git ops, merge, snapshot
- `claude-sync-cli` — 11 subcommands via clap
- `claude-sync-gui` — React + Tailwind + Tauri v2

## Installation

### CLI only

```bash
cargo install --path crates/claude-sync-cli
```

### GUI (requires WebView2 on Windows)

```bash
cargo tauri build -b crates/claude-sync-gui
```

## CLI Usage

```bash
# Initial setup
claude-sync init

# Push local config to GitHub
claude-sync push

# Pull remote config to local
claude-sync pull

# Check sync status
claude-sync status

# Manage skills individually
claude-sync skill list
claude-sync skill push my-skill
claude-sync skill pull my-skill

# View detected secrets
claude-sync secret list

# Diagnose issues
claude-sync doctor
```

### All Commands

| Command | Description |
|---------|-------------|
| `init` | Setup wizard (repo, auth, options) |
| `push [--dry-run]` | Push local config to remote |
| `pull [--force] [--dry-run]` | Pull remote config to local |
| `status` | Show sync status |
| `diff [--file <path>]` | Show local vs remote diff |
| `config show\|set\|edit` | Manage configuration |
| `skill list\|push\|pull` | Manage skills individually |
| `secret list\|add\|remove` | Manage secret patterns |
| `restore [--latest\|--list]` | Restore from snapshot |
| `doctor` | Diagnose issues |

## Secret Masking

API keys are detected via configurable patterns and replaced with empty strings before pushing:

```
mcpServers.*.env.*_API_KEY    → ""
mcpServers.*.env.*_TOKEN      → ""
mcpServers.*.env.*_SECRET     → ""
mcpServers.*.headers.*_API_KEY → ""
```

Additional heuristic detection: known prefixes (`sk-`, `hb_`, `ctx7sk-`, etc.) and high-entropy strings in `env`/`headers` context.

On pull, local secret values are automatically restored.

## Config

Stored at `~/.claude-sync/config.toml`:

```toml
[repo]
url = "git@github.com:user/claude-config.git"
branch = "main"

[auth]
method = "ssh_agent"  # ssh_agent | ssh_key | https_token | gh_cli

[sync]
sync_memory = false
sync_teams = true
sync_skills = true

[[secret_patterns]]
name = "MCP env API keys"
json_path = "mcpServers.*.env.*_API_KEY"
action = "Mask"
```

## License

MIT
