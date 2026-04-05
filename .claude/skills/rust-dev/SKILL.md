---
name: rust-dev
description: "claude-sync Rust 백엔드 개발 스킬. core 라이브러리 모듈(config, discovery, secret, git_ops, merge, platform, snapshot, manifest) 수정, CLI 명령어 추가, Tauri IPC 커맨드 구현 시 사용. Rust 코드, Cargo, crate 구조에 관련된 모든 작업에서 트리거된다."
---

# claude-sync Rust 개발 스킬

## Workspace 구조

3-crate workspace. core는 순수 Rust (Tauri 의존성 없음), CLI와 GUI가 core를 의존한다.

```
Cargo.toml (workspace root)
├── crates/claude-sync-core/   # 비즈니스 로직
├── crates/claude-sync-cli/    # CLI 바이너리 (package name: claude-sync)
└── crates/claude-sync-gui/    # Tauri v2 앱
```

## Core 모듈 가이드

### config.rs
`SyncConfig`가 메인 구조체. `~/.claude-sync/config.toml`에서 로드/저장한다.
- 새 설정 필드 추가 시: `SyncOptions`에 필드 추가 → `Default` impl 업데이트 → CLI init에 프롬프트 추가 → GUI SetupInput에 추가
- `#[serde(default)]` 또는 `#[serde(default = "fn_name")]`으로 하위 호환성 유지

### discovery.rs
`discover()` 함수가 `~/.claude/` 스캔. `FileCategory` enum으로 분류한다.
- 새 싱크 대상 추가 시: FileCategory variant 추가 → discover() 함수에 탐색 로직 추가 → NEVER_SYNC에서 제거 (해당 시)

### secret.rs
`SecretEngine`이 JSONPath 패턴 + 휴리스틱으로 시크릿을 탐지한다.
- 새 패턴 추가: `SyncConfig::default_secret_patterns()`에 추가
- 휴리스틱 추가: `KNOWN_SECRET_PREFIXES` 상수 또는 `is_likely_secret()` 수정

### git_ops.rs
시스템 `git` CLI 래퍼. Windows에서 `CREATE_NO_WINDOW` 필수.
- ��� git 명령 추가: `GitRepo` impl에 메서드 추가, 내부에서 `self.run_git()` 호출
- 모든 외부 프로세스 실행: `git_command()` 또는 `silent_command()` 헬퍼 사용

### merge.rs
JSON 3-way 머지. base/local/remote 비교 후 필드 레벨 머��.
- 배열은 합집합(union), 스칼라는 변경측 우선, 양쪽 변경 시 로컬 우선 + 충돌 보고

## CLI 명령어 추가 패턴

```rust
// 1. commands/mod.rs에 variant 추가
enum Commands { NewCmd { #[arg(long)] flag: bool } }

// 2. commands/new_cmd.rs 파일 생성
pub async fn run(flag: bool) -> anyhow::Result<()> { ... }

// 3. execute() 함수에 매칭 추가
Commands::NewCmd { flag } => new_cmd::run(flag).await,
```

## Tauri 커맨드 추가 패턴

```rust
// 1. crates/claude-sync-gui/src/lib.rs에 async 함수 추가
#[tauri::command]
async fn new_command(param: String) -> Result<ReturnType, String> {
    blocking(move || {
        // core 함수 호출 — 블로킹 I/O는 반드시 blocking() 안에서
        Ok(result)
    }).await
}

// 2. invoke_handler에 등록
.invoke_handler(tauri::generate_handler![..., new_command])

// 3. 반환 타입은 Serialize + Clone derive
#[derive(Serialize, Clone)]
struct ReturnType { ... }
```

## 빌드 검증

작업 완료 후 반드시:
```bash
cargo check --workspace
```
