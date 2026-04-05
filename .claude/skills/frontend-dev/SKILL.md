---
name: frontend-dev
description: "claude-sync React/TypeScript 프론트엔드 개발 스킬. GUI 컴포넌트(Dashboard, SkillManager, PluginManager, SecretManager, SetupWizard) 수정, 새 탭/화면 추가, Tauri IPC invoke 연동, Tailwind 스타일링 시 사용. React, TypeScript, Tailwind, Tauri 프론트엔드에 관련된 모든 작업에서 트리거된다."
---

# claude-sync Frontend 개발 스킬

## 프론트엔드 구조

```
crates/claude-sync-gui/frontend/src/
├── main.tsx              # ReactDOM.createRoot
├── App.tsx               # 탭: Dashboard | Skills | Plugins | Secrets
├── lib/types.ts          # 모든 TypeScript 인터페이���
├── styles/globals.css    # @import "tailwindcss"
└── components/
    ├── Dashboard.tsx     # 싱크 상태 + Push/Pull
    ├── SkillManager.tsx  # 스킬 체크박스 리스트
    ├── PluginManager.tsx # 플러그인 메타데이터
    ├── SecretManager.tsx # 시크릿 목록
    └── SetupWizard.tsx   # 5단계 셋업 위저드
```

## 새 컴포넌트 추가 패턴

```tsx
// 1. components/NewComponent.tsx
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { NewType } from "../lib/types";

export default function NewComponent() {
  const [data, setData] = useState<NewType[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => { loadData(); }, []);

  async function loadData() {
    setLoading(true);
    try {
      const result = await invoke<NewType[]>("tauri_command_name");
      setData(result);
    } catch (e) {
      // 에러 처리
    } finally {
      setLoading(false);
    }
  }

  if (loading) return <div className="p-6 text-gray-400">Loading...</div>;

  return (
    <div className="space-y-4">
      <h2 className="text-lg font-semibold">Title</h2>
      {/* content */}
    </div>
  );
}

// 2. App.tsx에 탭 추가
type Tab = "dashboard" | "skills" | "plugins" | "secrets" | "new-tab";
// tabs 배열에 추가, 렌더링 분기 추가

// 3. lib/types.ts에 인터페이스 추가
export interface NewType { field: string; }
```

## 디자인 시스템 (다크 테마)

| 요소 | 클래스 |
|------|--------|
| 배경 | `bg-gray-950` (최외곽), `bg-gray-900` (카드) |
| 텍스트 | `text-gray-100` (기본), `text-gray-400` (보조), `text-gray-500` (힌트) |
| 테두리 | `border-gray-800` (기본), `border-gray-700` (호버/인풋) |
| 버튼 (Primary) | `bg-blue-600 hover:bg-blue-500` |
| 버튼 (Success) | `bg-green-600 hover:bg-green-500` |
| 버튼 (Action) | `bg-purple-600 hover:bg-purple-500` |
| 뱃지 (성공) | `bg-green-900 text-green-300` |
| 뱃지 (경고) | `bg-yellow-900 text-yellow-300` |
| 뱃지 (비활성) | `bg-gray-800 text-gray-500` |
| 인풋 | `border-gray-700 bg-gray-800 focus:border-blue-500` |

## Tauri IPC 연동 규칙

- invoke 반환 타입은 반드시 `lib/types.ts`의 인터페이스와 일치해��� 함
- Rust의 `snake_case` 필드 → TypeScript도 `snake_case` (serde 기본)
- Rust `Option<T>` → TypeScript `T | null`
- Rust `Vec<T>` → TypeScript `T[]`
- invoke 파라미터는 object로 전달: `invoke("cmd", { param_name: value })`

## SetupWizard 구조

5단계 스텝 머신: `welcome → repo → auth → options → progress → done`
- `onComplete`: 셋업 완료 시 Dashboard로 전환
- `onCancel`: 이미 초기화된 상태에서만 표시 (처음 셋업 시 없음)
- AuthStep은 별도 컴포넌트로 분리 (인증 상태 감지 + PAT 입력 + gh CLI)

## 빌드 검증

```bash
cd crates/claude-sync-gui/frontend && pnpm build  # tsc + vite
```
