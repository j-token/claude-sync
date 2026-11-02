---
name: frontend-dev
description: "claude-sync React/TypeScript 프론트엔드 개발 스킬. GUI 탭(개요, 항목, 가져오기, P2P, 백업, 템플릿, 시크릿, 설정)과 BundlePreviewPanel 수정, 새 탭/화면 추가, Tauri IPC 연동, Tailwind 스타일링 시 사용. React, TypeScript, Tailwind, Tauri 프론트엔드에 관련된 모든 작업에서 트리거된다."
---

# claude-sync Frontend 개발 스킬

## 프론트엔드 구조

```
crates/claude-sync-gui/frontend/src/
├── App.tsx                     # 헤더, 탭 전환, 공유 세션, 알림 배너
├── lib/
│   ├── types.ts                # 데이터 타입 (Rust Serialize 구조체와 1:1)
│   ├── commands.ts             # CommandSpec: 커맨드별 input/output 계약
│   ├── backend.ts              # invokeCommand(): Tauri면 invoke(cmd, { input }), 아니면 mock
│   ├── mock.ts                 # 브라우저 미리보기용 mock (모든 커맨드를 처리해야 tsc 통과)
│   ├── dialog.ts               # 파일 열기/저장 대화상자 (@tauri-apps/plugin-dialog)
│   ├── kinds.ts, glob.ts, format.ts, notify.ts, useShareSession.ts
└── components/
    ├── OverviewTab.tsx, ItemsTab.tsx, ImportTab.tsx, P2PTab.tsx
    ├── BackupsTab.tsx, TemplatesTab.tsx, SecretsTab.tsx, SettingsTab.tsx
    ├── BundlePreviewPanel.tsx  # 가져오기·P2P 수신·백업 복원이 함께 쓰는 미리보기/선택
    └── ui.tsx                  # 공용 버튼·배지 등
```

## 커맨드 추가 순서

1. Rust `crates/claude-sync-gui/src/lib.rs`에 커맨드 추가 (인자는 `input` 구조체 하나)
2. `types.ts`에 타입, `commands.ts`의 `CommandSpec`에 항목 추가
3. `mock.ts`에 mock 처리 추가 (`satisfies never` 때문에 빠지면 빌드 실패)
4. 컴포넌트에서 `invokeCommand("name", { ...input })` 호출

## 규칙

- 필드명은 Rust와 같은 snake_case. `Option<T>` → `T | null`, `Vec<T>` → `T[]`
- 실행 항목(`executable`)은 받은 번들에서 기본 선택하지 않는다
- Tauri 환경에서는 `window.prompt/confirm`을 쓰지 않는다 (인라인 폼이나 두 단계 버튼 사용)
- UI 문구는 한국어

## 빌드 검증

```bash
pnpm --dir crates/claude-sync-gui/frontend build  # tsc -b && vite build
```
