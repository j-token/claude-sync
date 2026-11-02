---
name: rust-dev
description: "claude-sync Rust 백엔드 개발 스킬. core 라이브러리 모듈(inventory, bundle, local_json, template, backup, p2p, secret, portable, config, paths, app) 수정, CLI 명령어 추가, Tauri IPC 커맨드 구현 시 사용. Rust 코드, Cargo, crate 구조에 관련된 모든 작업에서 트리거된다."
---

# claude-sync Rust 개발 스킬

## Workspace 구조

3-crate workspace. core는 순수 Rust (Tauri 의존성 없음), CLI와 GUI가 core를 의존한다.
GitHub/git은 쓰지 않는다. 전송 수단은 ZIP 번들 파일과 LAN P2P다.

```
Cargo.toml (workspace root)
├── crates/claude-sync-core/   # 비즈니스 로직
├── crates/claude-sync-cli/    # CLI 바이너리 (package name: claude-sync)
└── crates/claude-sync-gui/    # Tauri v2 앱
```

## Core 모듈 가이드

| 모듈 | 역할 |
|------|------|
| `paths.rs` | 경로. `CLAUDE_SYNC_CLAUDE_DIR`, `CLAUDE_SYNC_CLAUDE_JSON`, `CLAUDE_SYNC_HOME`으로 재정의 가능 (테스트는 반드시 재정의) |
| `config.rs` | `~/.claude-sync/config.toml`. 파일이 없어도 기본값으로 동작 |
| `inventory.rs` | `~/.claude` 스캔 → `Item { id, kind, target, files }`. id 형식: `skill:<name>`, `agent:<file>`, `setting:<key>`, `hook:<Event>`, `mcp:<user|settings>:<name>`, `plugin:<id>` 등. `target_from_id()`는 번들에서 온 id를 검증 |
| `local_json.rs` | settings.json / `~/.claude.json` 안의 JSON 조각 항목 읽기·쓰기. `.claude.json`은 쓰기 직전에 다시 읽고 `mcpServers`만 교체 |
| `bundle.rs` | ZIP 번들 export / open(검증) / preview / import |
| `secret.rs` | 시크릿 탐지·마스킹 (경로 끝부분 패턴 + 휴리스틱). 위치는 JSON Pointer |
| `portable.rs` | 절대 경로 ↔ `{{CLAUDE_DIR}}`, `{{HOME}}` 토큰 |
| `template.rs` | 항목 id glob 템플릿 (기본 템플릿 + `~/.claude-sync/templates/*.json`) |
| `backup.rs` | 백업 = `~/.claude-sync/backups/`의 번들. 자동 백업은 개수 제한 |
| `p2p.rs` | SPAKE2 + ChaCha20-Poly1305 LAN 전송, UDP 브로드캐스트 탐색 |
| `app.rs` | CLI/GUI 공용 상위 작업 (가져오기 전 자동 백업 등 정책 포함) |

- 새 항목 종류 추가: `ItemKind` variant → `scan()` → `target_from_id()` → 프론트 `types.ts`의 `ItemKind`와 `kinds.ts`
- 외부 번들은 신뢰하지 않는다. 경로는 `fsutil::safe_relative_path`, id는 `target_from_id`로 검증

## Tauri 커맨드 추가 패턴

인자는 항상 `input: XxxInput` 구조체 하나 (snake_case 필드). 프론트 계약은 `frontend/src/lib/commands.ts`.
블로킹 작업은 `blocking(|| ...)` 안에서 실행한다. 커맨드를 추가하면 `commands.ts`, `types.ts`, `mock.ts`도 갱신한다.

## 빌드 검증

```bash
cargo test --workspace
pnpm --dir crates/claude-sync-gui/frontend build   # GUI check 전에 dist가 있어야 함
cargo check --workspace
```
