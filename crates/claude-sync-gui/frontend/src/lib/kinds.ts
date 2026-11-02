import type { ItemKind } from "./types";

export const KIND_ORDER: ItemKind[] = [
  "claude_md",
  "setting",
  "hook",
  "hook_script",
  "mcp",
  "plugin",
  "skill",
  "agent",
  "command",
  "rule",
  "output_style",
  "keybindings",
  "memory",
  "team",
];

export const KIND_LABELS: Record<ItemKind, string> = {
  claude_md: "CLAUDE.md",
  setting: "설정",
  hook: "훅",
  hook_script: "훅 스크립트",
  mcp: "MCP 서버",
  plugin: "플러그인",
  skill: "스킬",
  agent: "에이전트",
  command: "커맨드",
  rule: "룰",
  output_style: "출력 스타일",
  keybindings: "키바인딩",
  memory: "메모리",
  team: "팀",
};

export function kindLabel(kind: ItemKind): string {
  return KIND_LABELS[kind];
}
