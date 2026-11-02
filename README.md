# Claude Sync

Claude Code 설정을 항목 단위로 골라서 **ZIP 번들** 또는 **LAN P2P**로 내보내고, 가져오고, 백업하는 도구입니다.
GitHub 계정이나 git 저장소가 필요 없습니다.

## 기능

- **항목 단위 선택**: 스킬, 에이전트, 커맨드, 룰, 출력 스타일, CLAUDE.md, 키바인딩, 메모리, 팀, settings.json의 최상위 설정 키 하나하나, 훅 이벤트, 훅 스크립트, MCP 서버, 플러그인
- **ZIP 내보내기/가져오기**: 가져오기 전에 미리보기에서 항목마다 새 항목/동일/변경됨을 보여 줍니다. 병합, 덮어쓰기, 기존 항목 건너뛰기 중에서 고를 수 있습니다.
- **P2P 공유**: 같은 네트워크의 기기끼리 페어링 코드로 직접 전송합니다. 주변 기기 자동 탐색도 됩니다.
- **백업/복원**: 수동 백업을 만들 수 있고, 가져오기·복원 전에는 바뀔 항목을 자동으로 백업합니다. 항목을 골라서 복원할 수 있습니다.
- **템플릿**: 항목 id glob 패턴으로 이름 있는 선택 집합을 만들어 내보내기, 백업, 공유, 가져오기 필터에 씁니다.
- **시크릿 마스킹**: `env`·`headers`의 API 키, 토큰, 인증정보가 든 프록시 URL을 비워서 내보냅니다. 가져올 때는 그 기기에 있던 값으로 다시 채웁니다.
- **경로 이식**: 훅·MCP 명령에 들어 있는 `~/.claude`와 홈 경로를 토큰으로 바꿔 두었다가, 가져오는 기기의 경로로 바꿉니다.
- **GUI + CLI**: Tauri v2 데스크톱 앱과 CLI를 함께 제공합니다.

## 항목 id

| 종류 | id 예시 | 저장 위치 |
|------|---------|-----------|
| CLAUDE.md | `claude_md` | `~/.claude/CLAUDE.md` |
| 설정 | `setting:permissions`, `setting:model` | `settings.json` 최상위 키 |
| 훅 | `hook:PreToolUse` | `settings.json` → `hooks.<이벤트>` |
| 훅 스크립트 | `hook_script:guard.mjs` | `~/.claude/hooks/` |
| MCP 서버 | `mcp:user:exa`, `mcp:settings:local` | `~/.claude.json` / `settings.json`의 `mcpServers` |
| 플러그인 | `plugin:name@marketplace` | `settings.json`의 `enabledPlugins`, `extraKnownMarketplaces` |
| 스킬 / 팀 | `skill:review`, `team:core` | `~/.claude/skills/<이름>/`, `teams/<이름>/` |
| 에이전트·커맨드·룰·출력 스타일·메모리 | `agent:planner.md`, `command:sub/x.md` | 각 디렉토리의 파일 |
| 키바인딩 | `keybindings` | `keybindings.json` |

`.credentials.json`, `projects/`, `sessions/`, `history.jsonl`, 캐시는 다루지 않습니다.

## 설치

```bash
# CLI
cargo install --path crates/claude-sync-cli

# GUI (Windows는 WebView2 필요)
pnpm --dir crates/claude-sync-gui/frontend install
cd crates/claude-sync-gui
cargo tauri build
```

## CLI 사용법

```bash
claude-sync list                        # 항목과 id 보기 (--kind skill 등으로 거르기)

# 내보내기: 항목 id나 템플릿으로 고르고, 둘 다 생략하면 대화형으로 고른다
claude-sync export skill:review setting:permissions -o my.zip
claude-sync export -t skills-agents -o skills.zip

# 가져오기
claude-sync inspect my.zip              # 미리보기
claude-sync import my.zip               # 대화형 선택
claude-sync import my.zip --all --mode merge
claude-sync import my.zip -t essentials --yes   # 실행 항목 확인 생략

# 백업
claude-sync backup create -l "업데이트 전"      # 기본 템플릿(full)
claude-sync backup list
claude-sync backup restore 20261102-101500-manual claude_md

# 템플릿
claude-sync template list
claude-sync template create work -i 'skill:*' -i 'agent:*' -e 'skill:old-*' -d "업무용"
claude-sync template show work

# P2P
claude-sync share -t work               # 페어링 코드와 주소 출력
claude-sync peers                       # 주변 공유 기기 찾기
claude-sync receive 192.168.0.10:47821 --code K7P3-QX9M --import
```

| 명령 | 설명 |
|------|------|
| `list [--kind]` | 로컬 항목 목록 |
| `export [ITEMS] [-t] [-o] [-l]` | ZIP 번들로 내보내기 |
| `inspect FILE` | 번들 미리보기 |
| `import FILE [ITEMS] [--all] [-t] [-m merge\|overwrite\|skip] [--yes] [--dry-run]` | 가져오기 |
| `backup create\|list\|show\|restore\|delete` | 백업 관리 |
| `template list\|show\|create\|delete` | 템플릿 관리 |
| `share [ITEMS] [-t] [--port]` | P2P 공유 |
| `receive [ADDR] [--code] [--import]` | P2P 받기 |
| `peers` | 주변 공유 기기 탐색 |
| `secret list\|add\|remove` | 시크릿 탐지 결과와 마스킹 패턴 |
| `config show\|set\|edit\|path` | 설정 |
| `doctor` | 환경 진단 |

### 기본 템플릿

| 이름 | 포함 |
|------|------|
| `full` | 모든 항목 |
| `essentials` | CLAUDE.md, 설정, 훅, 훅 스크립트, 키바인딩 |
| `skills-agents` | 스킬, 에이전트, 커맨드, 출력 스타일 |
| `integrations` | MCP 서버, 플러그인 |
| `safe-share` | 명령을 실행하지 않는 항목만 (남에게 공유할 때) |

사용자 템플릿은 `~/.claude-sync/templates/<이름>.json`에 저장됩니다.

## 안전장치

- **실행 항목 확인**: 훅, 훅 스크립트, MCP 서버, 플러그인, `statusLine` 같은 설정은 이 기기에서 명령을 실행합니다. 미리보기에 실행될 명령 줄을 보여 주고, GUI에서는 기본으로 선택하지 않습니다. CLI는 확인을 받거나 `--yes`가 있어야 진행합니다.
- **번들 검증**: 항목 id와 파일 경로를 검사합니다. `..`, 절대 경로, 항목 범위를 벗어난 파일은 거부합니다. 파일마다 SHA-256으로 무결성을 확인하고, 크기 상한을 둡니다.
- **P2P 보안**: 짧은 페어링 코드로 SPAKE2 키 합의를 하고 키 확인을 거친 뒤에만 ChaCha20-Poly1305로 암호화해 전송합니다. 코드는 한 번만 쓸 수 있고, 3번 틀리면 공유가 닫힙니다.
- **원자적 쓰기**: `settings.json`과 `~/.claude.json`은 임시 파일에 쓴 뒤 교체합니다. `~/.claude.json`은 쓰기 직전에 다시 읽어 `mcpServers`만 바꾸고, 키 순서를 그대로 둡니다.

## P2P 네트워크 참고

- 보내는 쪽은 TCP 47821 포트(사용 중이면 임의 포트)에서 기다리고, UDP 47820으로 자신을 알립니다.
- Windows 방화벽이 허용을 물으면 개인 네트워크를 허용하세요. 브로드캐스트가 막힌 네트워크에서는 탐색이 안 되므로, 화면에 나온 `ip:port`를 직접 입력하세요.
- 같은 LAN(또는 VPN)에서만 동작합니다. 인터넷을 건너는 NAT 통과는 지원하지 않습니다.

## 설정과 경로

`~/.claude-sync/config.toml`이 없어도 기본값으로 동작합니다.

```toml
default_template = "full"

[device]
name = "my-laptop"

[backup]
max_auto_backups = 20
backup_before_import = true

[p2p]
port = 47821
discovery = true
```

| 경로 | 용도 |
|------|------|
| `~/.claude-sync/backups/` | 백업 번들 |
| `~/.claude-sync/templates/` | 사용자 템플릿 |
| `~/.claude-sync/inbox/` | P2P로 받은 번들 |

환경 변수 `CLAUDE_CONFIG_DIR`를 따르고, 테스트용으로 `CLAUDE_SYNC_CLAUDE_DIR`, `CLAUDE_SYNC_CLAUDE_JSON`, `CLAUDE_SYNC_HOME`으로 경로를 바꿀 수 있습니다.

## 구조

```
crates/
├── claude-sync-core/   # 순수 Rust 라이브러리 (inventory, bundle, template, backup, p2p, secret ...)
├── claude-sync-cli/    # CLI (clap)
└── claude-sync-gui/    # Tauri v2 + React + Tailwind
```

## License

MIT
