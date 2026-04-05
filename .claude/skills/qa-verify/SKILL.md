---
name: qa-verify
description: "claude-sync 빌드 및 인터페이스 검증 스킬. cargo check, cargo build, cargo tauri build, pnpm build 실행, CLI 명령어 테스트, Rust Tauri 커맨드 ↔ TypeScript 타입 정합성 교차 검증을 수행한다. 코드 변경 후 검증, 빌드 테스트, QA가 필요할 때 트리거된다."
---

# claude-sync QA 검증 스킬

## 검증 단계 (순서대로 실행)

### Step 1: Rust 빌드

```bash
cargo check --workspace
```

실패 시: 에러 메시지에서 파일:라인 추출 → 해당 에이전트에게 수정 요청.

### Step 2: 인터페이스 정합성 검증

**Rust 측** `crates/claude-sync-gui/src/lib.rs`에서:
- 모든 `#[derive(Serialize)]` 구조체의 필드 목록 추출
- 모든 `#[tauri::command]` 함수의 반환 타입 매핑

**TypeScript 측** `frontend/src/lib/types.ts`에서:
- 모든 `interface` 정의의 필드 목록 추출

**비교 규칙:**
| Rust | TypeScript | OK? |
|------|-----------|-----|
| `field: String` | `field: string` | O |
| `field: bool` | `field: boolean` | O |
| `field: usize` | `field: number` | O |
| `field: Option<String>` | `field: string \| null` | O |
| `field: Vec<T>` | `field: T[]` | O |
| 필드 누락 | - | **X** (에러) |

불일치 발견 시: Rust가 정본(source of truth). TypeScript 수정을 frontend-engineer에게 요청.

### Step 3: 프론트엔드 빌드

```bash
cd crates/claude-sync-gui/frontend && pnpm build
```

실패 시: tsc 에러 또는 vite 에러 분석 → frontend-engineer에게 수정 요청.

### Step 4: CLI 기능 테스트

테스트 config 생성 후 각 명령어 실행:

```bash
CLAUDE_SYNC="target/debug/claude-sync.exe"  # 또는 release

# 기본 동작
$CLAUDE_SYNC --help
$CLAUDE_SYNC doctor
$CLAUDE_SYNC status
$CLAUDE_SYNC push --dry-run
$CLAUDE_SYNC pull --dry-run
$CLAUDE_SYNC skill list
$CLAUDE_SYNC secret list
$CLAUDE_SYNC config show
$CLAUDE_SYNC restore --list
```

각 명령어의 종료 코드 확인. 비정상 종료(panic, exit code != 0) 시 에러 보고.

### Step 5: Tauri 앱 빌드 (최종 검증)

```bash
taskkill //f //im claude-sync-gui.exe 2>/dev/null  # 기존 프로세스 종료
cd crates/claude-sync-gui && cargo tauri build
```

성공 기준: MSI + NSIS 번들 생성 확인.

## 검증 결과 보고 형식

```markdown
## QA 검증 결과

| 항목 | 결과 | 비고 |
|------|------|------|
| Rust build | PASS/FAIL | 에러 메시지 |
| Interface check | PASS/FAIL | 불일치 필드 |
| Frontend build | PASS/FAIL | 에러 메시지 |
| CLI test | PASS/FAIL | 실패 명령어 |
| Tauri build | PASS/FAIL | 에러 메시지 |

### 수정 필요 사항
- [ ] 항목 1
- [ ] 항목 2
```
