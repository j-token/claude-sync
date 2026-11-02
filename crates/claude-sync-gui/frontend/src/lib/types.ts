export type ItemKind =
  | "claude_md"
  | "setting"
  | "hook"
  | "hook_script"
  | "mcp"
  | "plugin"
  | "skill"
  | "agent"
  | "command"
  | "rule"
  | "output_style"
  | "keybindings"
  | "memory"
  | "team";

export interface KindCount {
  kind: ItemKind;
  label: string;
  count: number;
}

export interface Overview {
  device_name: string;
  platform: string;
  claude_dir: string;
  sync_home: string;
  kinds: KindCount[];
  total_items: number;
  backup_count: number;
  last_backup: string | null;
  template_count: number;
  default_template: string;
}

export interface ItemSummary {
  id: string;
  kind: ItemKind;
  name: string;
  description: string | null;
  size_bytes: number;
  file_count: number;
  /** 훅·MCP·플러그인·statusLine처럼 명령을 실행할 수 있는 항목 */
  executable: boolean;
  secret_count: number;
}

export interface TemplateInfo {
  name: string;
  description: string;
  /** 항목 id에 대한 glob 패턴. 예: "skill:*", "setting:permissions", "*" */
  include: string[];
  exclude: string[];
  builtin: boolean;
}

export interface TemplateInput {
  name: string;
  description: string;
  include: string[];
  exclude: string[];
}

export interface ExportResult {
  path: string;
  item_count: number;
  file_count: number;
  size_bytes: number;
  masked_secret_count: number;
}

export type PreviewStatus = "new" | "same" | "changed";

export interface PreviewItem {
  id: string;
  kind: ItemKind;
  name: string;
  description: string | null;
  size_bytes: number;
  file_count: number;
  executable: boolean;
  status: PreviewStatus;
  masked_secret_count: number;
  /** 실행 항목이면 실제로 실행될 명령 줄 등 사용자가 확인해야 할 내용 */
  details: string[];
}

export interface BundlePreview {
  path: string;
  label: string | null;
  created_at: string;
  device_name: string;
  platform: string;
  template: string | null;
  size_bytes: number;
  items: PreviewItem[];
}

export type ImportMode = "merge" | "overwrite" | "skip_existing";

export interface SkippedItem {
  id: string;
  reason: string;
}

export interface ImportReport {
  imported: string[];
  skipped: SkippedItem[];
  warnings: string[];
  backup_id: string | null;
  backup_skipped_reason: string | null;
}

export type BackupKind = "auto" | "manual";

export interface BackupInfo {
  id: string;
  path: string;
  created_at: string;
  kind: BackupKind;
  label: string | null;
  item_count: number;
  size_bytes: number;
}

export interface ShareInfo {
  code: string;
  port: number;
  addresses: string[];
  device_name: string;
  item_count: number;
  size_bytes: number;
}

export type SharePhase = "idle" | "waiting" | "transferring" | "completed" | "failed" | "stopped";

export interface ShareStatus {
  phase: SharePhase;
  message: string;
  peer: string | null;
  failed_attempts: number;
}

export interface PeerInfo {
  device_name: string;
  /** "ip:port" */
  address: string;
  item_count: number;
}

export interface DetectedSecret {
  item_id: string;
  pointer: string;
  pattern_name: string;
  preview: string;
}

export type SecretAction = "mask" | "remove" | "keep";

export interface SecretPatternInfo {
  name: string;
  json_path: string;
  action: SecretAction;
}

export interface AppConfig {
  device_name: string;
  default_template: string;
  max_auto_backups: number;
  backup_before_import: boolean;
  p2p_port: number;
  p2p_discovery: boolean;
  secret_patterns: SecretPatternInfo[];
}
