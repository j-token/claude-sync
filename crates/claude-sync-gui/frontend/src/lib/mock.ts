import type { CommandInput, CommandName } from "./commands";
import { filterByTemplate } from "./glob";
import { KIND_LABELS, KIND_ORDER } from "./kinds";
import type {
  AppConfig,
  BackupInfo,
  BundlePreview,
  DetectedSecret,
  ImportReport,
  ItemKind,
  ItemSummary,
  PreviewItem,
  PreviewStatus,
  ShareInfo,
  SharePhase,
  TemplateInfo,
} from "./types";

function item(
  id: string,
  kind: ItemKind,
  name: string,
  description: string | null,
  size_bytes: number,
  extra: Partial<Pick<ItemSummary, "file_count" | "executable" | "secret_count">> = {},
): ItemSummary {
  return {
    id,
    kind,
    name,
    description,
    size_bytes,
    file_count: extra.file_count ?? 1,
    executable: extra.executable ?? false,
    secret_count: extra.secret_count ?? 0,
  };
}

const items: ItemSummary[] = [
  item("claude_md", "claude_md", "CLAUDE.md", "전역 지침 (한국어 응답, 커밋 규칙)", 4_812),
  item("setting:permissions", "setting", "permissions", "허용/거부 도구 목록", 1_204),
  item("setting:env", "setting", "env", "환경 변수", 388, { secret_count: 2 }),
  item("setting:model", "setting", "model", "기본 모델 설정", 42),
  item("setting:statusLine", "setting", "statusLine", "상태 표시줄 명령", 156, { executable: true }),
  item("hook:PreToolUse", "hook", "PreToolUse", "Bash 명령 체이닝 차단", 512, { executable: true }),
  item("hook:PostToolUse", "hook", "PostToolUse", "편집 후 prettier 실행", 344, { executable: true }),
  item("hook:Stop", "hook", "Stop", "작업 종료 알림", 210, { executable: true }),
  item("hook_script:block-chain.ps1", "hook_script", "block-chain.ps1", null, 1_920, { executable: true }),
  item("hook_script:notify.sh", "hook_script", "notify.sh", null, 640, { executable: true }),
  item("mcp:user:context7", "mcp", "context7", "npx @upstash/context7-mcp", 260, { executable: true }),
  item("mcp:user:github", "mcp", "github", "GitHub MCP 서버", 410, { executable: true, secret_count: 1 }),
  item("mcp:settings:playwright", "mcp", "playwright", "브라우저 자동화", 230, { executable: true }),
  item("plugin:exa@claude-plugins", "plugin", "exa@claude-plugins", "웹 검색 플러그인", 88_000, { executable: true, file_count: 14 }),
  item("plugin:aseprite@community", "plugin", "aseprite@community", "픽셀 아트 도구", 152_000, { executable: true, file_count: 31 }),
  item("skill:commit-rule", "skill", "commit-rule", "한국어 커밋 메시지 규칙", 3_400, { file_count: 2 }),
  item("skill:frontend-dev", "skill", "frontend-dev", "React/TypeScript 프론트엔드 개발", 12_800, { file_count: 5 }),
  item("skill:debug-flow", "skill", "debug-flow", "구조적 디버깅 절차", 6_100, { file_count: 3 }),
  item("skill:pr-rule", "skill", "pr-rule", "PR 본문 템플릿", 2_900),
  item("agent:verifier.md", "agent", "verifier.md", "적대적 버그 탐지 에이전트", 5_200),
  item("agent:librarian.md", "agent", "librarian.md", "외부 문서 조사 에이전트", 3_800),
  item("command:review.md", "command", "review.md", "/review 커맨드", 1_500),
  item("command:release.md", "command", "release.md", "/release 커맨드", 2_100),
  item("rule:readable-code.md", "rule", "readable-code.md", "읽기 좋은 코드 원칙", 4_300),
  item("output_style:concise.md", "output_style", "concise.md", "간결한 응답 스타일", 900),
  item("keybindings", "keybindings", "keybindings.json", "단축키 설정", 620),
  item("memory:project-notes.md", "memory", "project-notes.md", "프로젝트 메모", 7_400),
  item("team:claude-sync", "team", "claude-sync", "에이전트 팀 구성", 9_800, { file_count: 4 }),
];

const templates: TemplateInfo[] = [
  { name: "full", description: "모든 항목", include: ["*"], exclude: [], builtin: true },
  {
    name: "essentials",
    description: "CLAUDE.md, 설정, 스킬, 에이전트, 커맨드, 룰",
    include: ["claude_md", "setting:*", "skill:*", "agent:*", "command:*", "rule:*"],
    exclude: ["setting:env"],
    builtin: true,
  },
  { name: "skills-agents", description: "스킬과 에이전트만", include: ["skill:*", "agent:*"], exclude: [], builtin: true },
  {
    name: "integrations",
    description: "MCP 서버, 플러그인, 훅",
    include: ["mcp:*", "plugin:*", "hook:*", "hook_script:*"],
    exclude: [],
    builtin: true,
  },
  {
    name: "safe-share",
    description: "명령을 실행하지 않는 항목만 (다른 사람과 공유용)",
    include: ["claude_md", "skill:*", "agent:*", "command:*", "rule:*", "output_style:*"],
    exclude: [],
    builtin: true,
  },
  {
    name: "회사-노트북",
    description: "회사 노트북에 필요한 최소 구성",
    include: ["claude_md", "skill:commit-rule", "skill:pr-rule", "rule:*", "mcp:user:context7"],
    exclude: [],
    builtin: false,
  },
];

const backups: BackupInfo[] = [
  {
    id: "20261101-2214-auto",
    path: "C:/Users/demo/.claude-sync/backups/20261101-2214-auto.zip",
    created_at: "2026-11-01T22:14:00+09:00",
    kind: "auto",
    label: "가져오기 전 자동 백업",
    item_count: 28,
    size_bytes: 312_000,
  },
  {
    id: "20261028-0930-manual",
    path: "C:/Users/demo/.claude-sync/backups/20261028-0930-manual.zip",
    created_at: "2026-10-28T09:30:00+09:00",
    kind: "manual",
    label: "플러그인 업데이트 전",
    item_count: 26,
    size_bytes: 298_500,
  },
];

const secrets: DetectedSecret[] = [
  { item_id: "setting:env", pointer: "/env/ANTHROPIC_API_KEY", pattern_name: "API 키", preview: "sk-ant-...9A1b" },
  { item_id: "setting:env", pointer: "/env/OPENAI_API_KEY", pattern_name: "API 키", preview: "sk-...k2Pq" },
  { item_id: "mcp:user:github", pointer: "/env/GITHUB_TOKEN", pattern_name: "토큰", preview: "ghp_...82kQ" },
];

let config: AppConfig = {
  device_name: "작업실-데스크톱",
  default_template: "essentials",
  max_auto_backups: 10,
  backup_before_import: true,
  p2p_port: 47321,
  p2p_discovery: true,
  secret_patterns: [
    { name: "API 키", json_path: "env.*_API_KEY", action: "mask" },
    { name: "토큰", json_path: "*.env.*TOKEN*", action: "mask" },
    { name: "비밀번호", json_path: "*password*", action: "remove" },
  ],
};

let share: { info: ShareInfo; startedAt: number; stopped: boolean } | null = null;

const EXECUTABLE_DETAILS: Record<string, string[]> = {
  "hook:PreToolUse": ["matcher: Bash", "pwsh -File ~/.claude/hooks/block-chain.ps1"],
  "hook:PostToolUse": ["matcher: Edit|Write", "npx prettier --write \"$CLAUDE_FILE_PATH\""],
  "mcp:user:context7": ["npx -y @upstash/context7-mcp@latest"],
  "plugin:exa@claude-plugins": ["marketplace: claude-plugins", "MCP: node ./server/index.js"],
  "setting:statusLine": ["bash ~/.claude/statusline.sh"],
};

function delay(ms = 250): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function nowIso(): string {
  return new Date().toISOString();
}

function backupId(kind: "auto" | "manual"): string {
  const stamp = new Date().toISOString().slice(0, 16).replace(/[-:]/g, "").replace("T", "-");
  return `${stamp}-${kind}`;
}

function resolveTemplate(name: string): string[] {
  const template = templates.find((t) => t.name === name);
  if (!template) throw `템플릿을 찾을 수 없습니다: ${name}`;
  return filterByTemplate(
    items.map((i) => i.id),
    template.include,
    template.exclude,
  );
}

function sumSize(ids: string[]): number {
  return items.filter((i) => ids.includes(i.id)).reduce((sum, i) => sum + i.size_bytes, 0);
}

function previewItem(source: ItemSummary, status: PreviewStatus): PreviewItem {
  return {
    id: source.id,
    kind: source.kind,
    name: source.name,
    description: source.description,
    size_bytes: source.size_bytes,
    file_count: source.file_count,
    executable: source.executable,
    status,
    masked_secret_count: source.secret_count,
    details: EXECUTABLE_DETAILS[source.id] ?? (source.executable ? ["(명령 정보 없음)"] : []),
  };
}

function demoPreview(path: string, deviceName: string, label: string | null): BundlePreview {
  const pick: [string, PreviewStatus][] = [
    ["claude_md", "changed"],
    ["setting:permissions", "same"],
    ["setting:env", "changed"],
    ["hook:PreToolUse", "new"],
    ["hook:PostToolUse", "changed"],
    ["mcp:user:context7", "new"],
    ["plugin:exa@claude-plugins", "new"],
    ["setting:statusLine", "same"],
    ["skill:commit-rule", "same"],
    ["skill:frontend-dev", "changed"],
    ["agent:verifier.md", "new"],
    ["command:review.md", "new"],
    ["rule:readable-code.md", "same"],
  ];
  const previewItems = pick.map(([id, status]) => {
    const source = items.find((i) => i.id === id);
    if (!source) throw `데모 항목 누락: ${id}`;
    return previewItem(source, status);
  });
  previewItems.push({
    id: "skill:ux-writing",
    kind: "skill",
    name: "ux-writing",
    description: "UI 문구 작성 가이드",
    size_bytes: 5_600,
    file_count: 2,
    executable: false,
    status: "new",
    masked_secret_count: 0,
    details: [],
  });
  return {
    path,
    label,
    created_at: "2026-11-02T14:05:00+09:00",
    device_name: deviceName,
    platform: "macos",
    template: "essentials",
    size_bytes: previewItems.reduce((sum, i) => sum + i.size_bytes, 0),
    items: previewItems,
  };
}

function backupPreview(backup: BackupInfo): BundlePreview {
  const previewItems = items.slice(0, backup.item_count).map((source, index) =>
    previewItem(source, index % 4 === 0 ? "changed" : "same"),
  );
  return {
    path: backup.path,
    label: backup.label,
    created_at: backup.created_at,
    device_name: config.device_name,
    platform: "windows",
    template: null,
    size_bytes: backup.size_bytes,
    items: previewItems,
  };
}

function importReport(ids: string[], skipExisting: boolean): ImportReport {
  const skipped = skipExisting
    ? ids.filter((id) => items.some((i) => i.id === id)).map((id) => ({ id, reason: "이미 존재하는 항목" }))
    : [];
  const skippedIds = new Set(skipped.map((s) => s.id));
  let autoBackupId: string | null = null;
  if (config.backup_before_import) {
    const created = makeBackup("auto", "가져오기 전 자동 백업", items.length);
    autoBackupId = created.id;
  }
  const warnings = ids.includes("setting:env")
    ? ["setting:env: 마스킹된 시크릿 2개는 이 기기의 기존 값으로 복원했습니다."]
    : [];
  return {
    imported: ids.filter((id) => !skippedIds.has(id)),
    skipped,
    warnings,
    backup_id: autoBackupId,
    backup_skipped_reason: autoBackupId ? null : "설정에서 자동 백업이 꺼져 있습니다.",
  };
}

function makeBackup(kind: "auto" | "manual", label: string | null, itemCount: number): BackupInfo {
  const id = backupId(kind);
  const backup: BackupInfo = {
    id,
    path: `C:/Users/demo/.claude-sync/backups/${id}.zip`,
    created_at: nowIso(),
    kind,
    label,
    item_count: itemCount,
    size_bytes: 9_000 * itemCount,
  };
  backups.unshift(backup);
  return backup;
}

function sharePhase(): SharePhase {
  if (!share) return "idle";
  if (share.stopped) return "stopped";
  const elapsedMs = Date.now() - share.startedAt;
  if (elapsedMs < 4_000) return "waiting";
  if (elapsedMs < 7_000) return "transferring";
  return "completed";
}

const SHARE_MESSAGES: Record<SharePhase, string> = {
  idle: "공유 중이 아닙니다.",
  waiting: "상대 기기의 연결을 기다리는 중...",
  transferring: "번들을 전송하는 중...",
  completed: "전송을 완료했습니다.",
  failed: "전송에 실패했습니다.",
  stopped: "공유를 중지했습니다.",
};

export async function mockInvoke(command: CommandName, input: unknown): Promise<unknown> {
  await delay();
  switch (command) {
    case "get_overview": {
      const kinds = KIND_ORDER.map((kind) => ({
        kind,
        label: KIND_LABELS[kind],
        count: items.filter((i) => i.kind === kind).length,
      })).filter((k) => k.count > 0);
      return {
        device_name: config.device_name,
        platform: "windows",
        claude_dir: "C:/Users/demo/.claude",
        sync_home: "C:/Users/demo/.claude-sync",
        kinds,
        total_items: items.length,
        backup_count: backups.length,
        last_backup: backups[0]?.created_at ?? null,
        template_count: templates.length,
        default_template: config.default_template,
      };
    }
    case "list_items":
      return structuredClone(items);
    case "list_secrets":
      return structuredClone(secrets);

    case "list_templates":
      return structuredClone(templates);
    case "save_template": {
      const template = input as CommandInput<"save_template">;
      const existing = templates.findIndex((t) => t.name === template.name);
      if (existing >= 0 && templates[existing].builtin) throw `기본 템플릿은 수정할 수 없습니다: ${template.name}`;
      const saved: TemplateInfo = { ...template, builtin: false };
      if (existing >= 0) templates[existing] = saved;
      else templates.push(saved);
      return structuredClone(saved);
    }
    case "delete_template": {
      const { name } = input as CommandInput<"delete_template">;
      const index = templates.findIndex((t) => t.name === name);
      if (index < 0) throw `템플릿을 찾을 수 없습니다: ${name}`;
      if (templates[index].builtin) throw `기본 템플릿은 삭제할 수 없습니다: ${name}`;
      templates.splice(index, 1);
      return `템플릿 '${name}'을(를) 삭제했습니다.`;
    }
    case "resolve_template":
      return resolveTemplate((input as CommandInput<"resolve_template">).name);

    case "export_bundle": {
      const { ids, path } = input as CommandInput<"export_bundle">;
      const selected = items.filter((i) => ids.includes(i.id));
      return {
        path,
        item_count: selected.length,
        file_count: selected.reduce((sum, i) => sum + i.file_count, 0),
        size_bytes: sumSize(ids),
        masked_secret_count: selected.reduce((sum, i) => sum + i.secret_count, 0),
      };
    }
    case "inspect_bundle": {
      const { path } = input as CommandInput<"inspect_bundle">;
      const backup = backups.find((b) => b.path === path);
      if (backup) return backupPreview(backup);
      return demoPreview(path, "맥북-프로", "새 맥북 셋업용");
    }
    case "import_bundle": {
      const { ids, mode } = input as CommandInput<"import_bundle">;
      return importReport(ids, mode === "skip_existing");
    }

    case "list_backups":
      return structuredClone(backups);
    case "create_backup": {
      const { ids, template, label } = input as CommandInput<"create_backup">;
      const resolvedIds = ids ?? (template ? resolveTemplate(template) : items.map((i) => i.id));
      return makeBackup("manual", label, resolvedIds.length);
    }
    case "restore_backup": {
      const { id, ids } = input as CommandInput<"restore_backup">;
      if (!backups.some((b) => b.id === id)) throw `백업을 찾을 수 없습니다: ${id}`;
      return importReport(ids, false);
    }
    case "delete_backup": {
      const { id } = input as CommandInput<"delete_backup">;
      const index = backups.findIndex((b) => b.id === id);
      if (index < 0) throw `백업을 찾을 수 없습니다: ${id}`;
      backups.splice(index, 1);
      return `백업 ${id}을(를) 삭제했습니다.`;
    }

    case "p2p_start_share": {
      const { ids } = input as CommandInput<"p2p_start_share">;
      if (ids.length === 0) throw "공유할 항목이 없습니다.";
      const info: ShareInfo = {
        code: "K7P3-QX9M",
        port: config.p2p_port,
        addresses: [`192.168.0.24:${config.p2p_port}`, `10.0.0.7:${config.p2p_port}`],
        device_name: config.device_name,
        item_count: ids.length,
        size_bytes: sumSize(ids),
      };
      share = { info, startedAt: Date.now(), stopped: false };
      return structuredClone(info);
    }
    case "p2p_share_status": {
      const phase = sharePhase();
      return {
        phase,
        message: SHARE_MESSAGES[phase],
        peer: phase === "transferring" || phase === "completed" ? "192.168.0.31 (맥북-프로)" : null,
        failed_attempts: 0,
      };
    }
    case "p2p_stop_share":
      if (share) share.stopped = true;
      return "공유를 중지했습니다.";
    case "p2p_discover":
      await delay(1_200);
      return [
        { device_name: "맥북-프로", address: `192.168.0.31:${config.p2p_port}`, item_count: 14 },
        { device_name: "회사-노트북", address: `192.168.0.45:${config.p2p_port}`, item_count: 9 },
      ];
    case "p2p_receive": {
      const { address, code } = input as CommandInput<"p2p_receive">;
      if (code.replace("-", "").length !== 8) throw "페어링 코드는 8자리입니다. (예: K7P3-QX9M)";
      await delay(800);
      return demoPreview("C:/Users/demo/AppData/Local/Temp/claude-sync-received.zip", `${address}의 기기`, null);
    }

    case "get_config":
      return structuredClone(config);
    case "update_config":
      config = structuredClone(input as CommandInput<"update_config">);
      return structuredClone(config);

    default:
      throw new Error(`지원하지 않는 커맨드: ${command satisfies never}`);
  }
}
