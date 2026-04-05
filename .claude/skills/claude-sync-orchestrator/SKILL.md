---
name: claude-sync-orchestrator
description: "claude-sync 에이전트 팀을 조율하는 오케스트레이터. 기능 추가, 버그 수정, 리팩토링 등 Rust+React 풀스택 작업 시 rust-engineer, frontend-engineer, qa-engineer 팀을 구성하고 조율한다. 'claude-sync에 기능 추가', '버그 수정', '구현해줘', 'build', '개발' 등의 요청에서 트리거된다."
---

# claude-sync 개발 오케스트레이터

claude-sync 프로젝트의 에이전트 팀을 조율하여 풀스택 기능 개발을 수행한다.

## 실행 모드: 에이전트 팀

## 에이전트 구성

| 팀원 | 타입 | 역할 | 스킬 | 출력 |
|------|------|------|------|------|
| rust-eng | rust-engineer | Rust 백엔드 (core, CLI, Tauri) | rust-dev | Rust 소스 변경 |
| frontend-eng | frontend-engineer | React GUI 프론트엔드 | frontend-dev | TypeScript 소스 변경 |
| qa-eng | qa-engineer | 빌드 검증 + 인터페이스 정합성 | qa-verify | 검증 리포트 |

## 워크플로우

### Phase 1: 준비

1. 사용자 요청 분석 — Rust만 / Frontend만 / 풀스택 판단
2. `_workspace/` 디렉토리 생성
3. 변경 범위 파악:
   - **Rust만** (core 로직, CLI): rust-eng 단독
   - **Frontend만** (UI 변경): frontend-eng 단독
   - **풀스택** (Tauri 커맨드 + UI): 팀 구성

### Phase 2: 팀 구성

**풀스택 작업 시:**

```
TeamCreate(
  team_name: "claude-sync-dev",
  members: [
    { name: "rust-eng", agent_type: "rust-engineer", model: "opus",
      prompt: "rust-dev 스킬을 읽고 다음 작업을 수행하라: {task-description}" },
    { name: "frontend-eng", agent_type: "frontend-engineer", model: "opus",
      prompt: "frontend-dev 스킬을 읽고 다음 작업을 수행하라: {task-description}" },
    { name: "qa-eng", agent_type: "qa-engineer", model: "opus",
      prompt: "qa-verify 스킬을 읽고 rust-eng, frontend-eng의 작업 완료 후 검증을 수행하라." }
  ]
)
```

```
TaskCreate(tasks: [
  { title: "Rust: {feature-description}", assignee: "rust-eng" },
  { title: "Frontend: {feature-description}", assignee: "frontend-eng",
    depends_on: ["Rust: {feature-description}"] },  # Tauri 커맨드가 먼저 필요한 경우
  { title: "QA: 빌드 및 인터페이스 검증", assignee: "qa-eng",
    depends_on: ["Rust: ...", "Frontend: ..."] }
])
```

**단일 영역 작업 시:**

서브 에이전트로 단독 실행:
```
Agent(name="rust-eng", subagent_type="rust-engineer", model="opus",
      prompt="rust-dev 스킬을 읽고: {task}")
```

### Phase 3: 개발

**팀 작업 흐름:**

1. **rust-eng**: Core/CLI/Tauri 커맨드 구현
   - 새 Tauri 커맨드 시그니처를 frontend-eng에게 SendMessage
   - 완료 시 qa-eng에게 알림
2. **frontend-eng**: React 컴포넌트 + 타입 구현
   - rust-eng의 시그니처 수신 → types.ts 업데이트 → 컴포넌트 구현
   - 완료 시 qa-eng에게 알림
3. **qa-eng**: 양쪽 완료 대기 → 검증 수행
   - FAIL 시 해당 에이전트에게 수정 요청

**산출물 저장:**
```
_workspace/
├── 01_rust-eng_changes.md    # 변경 파일 목록 + 요약
├── 02_frontend-eng_changes.md
└── 03_qa-eng_report.md       # PASS/FAIL 리포트
```

### Phase 4: 통합

1. qa-eng 리포트 확인
2. PASS: 최종 빌드 수행
3. FAIL: 수정 요청 → Phase 3 반복 (최대 2회)

### Phase 5: 정리

1. 팀원에게 종료 SendMessage
2. TeamDelete
3. `_workspace/` 보존
4. 사용자에게 결과 요약:
   - 변경된 파일 목록
   - 빌드 상태
   - 새 기능 사용 방법

## 데이터 흐름

```
[리더] → TeamCreate
          │
    ┌─────┴─────┐
    ▼           ▼
[rust-eng]  [frontend-eng]
    │           │
    ├──SendMessage──┤  (Tauri 커맨드 시그니처)
    │           │
    ▼           ▼
 Rust 코드   React 코드
    │           │
    └─────┬─────┘
          ▼
     [qa-eng]
          │
     검증 리포트
          │
     [리더: 결과 종합]
```

## 에러 핸들링

| 상황 | 전략 |
|------|------|
| rust-eng 빌드 실패 | 에러 분석 → 수정 재시도 (1회) |
| frontend-eng 빌드 실패 | 에러 분석 → 수정 재시도 (1회) |
| 인터페이스 불일치 | Rust 기준으로 TypeScript 수정 |
| qa-eng 전체 FAIL | 에러 분류 → 각 에이전트에 수정 요청 → 재검증 |
| 2회 재시도 후 실패 | 사용자에게 상황 보고 + 수동 개입 요청 |
| Tauri 빌드 실패 (MSVC) | 환경 문제 보고 (코드 문제 아닐 수 있음) |

## 테스트 시나리오

### 정상 흐름
1. 사용자: "새로운 config 필드 추가해줘"
2. Phase 1: Rust + Frontend 풀스택 판단
3. Phase 2: 3인 팀 구성
4. Phase 3: rust-eng (config.rs + lib.rs) → frontend-eng (types.ts + Dashboard) → qa-eng (검증)
5. Phase 4: PASS
6. Phase 5: 결과 보고

### 에러 흐름
1. 사용자: "새 CLI 명령어 추가해줘"
2. Phase 1: Rust 단독 판단 → 서브 에이전트
3. rust-eng 구현 완료
4. qa-eng 검증: cargo check 실패 (타입 에러)
5. rust-eng에게 수정 요청
6. 수정 후 재검증: PASS
7. 결과 보고
