import type {
  AppConfig,
  BackupInfo,
  BundlePreview,
  DetectedSecret,
  ExportResult,
  ImportMode,
  ImportReport,
  ItemSummary,
  Overview,
  PeerInfo,
  ShareInfo,
  ShareStatus,
  TemplateInfo,
  TemplateInput,
} from "./types";

/**
 * Tauri 커맨드 계약. Rust(`crates/claude-sync-gui/src/lib.rs`)가 정본이다.
 * 인자가 있는 커맨드는 모두 `{ input: {...} }` 한 개만 받는다 (필드명은 snake_case).
 */
export type CommandSpec = {
  get_overview: { input: void; output: Overview };
  list_items: { input: void; output: ItemSummary[] };
  list_secrets: { input: void; output: DetectedSecret[] };

  list_templates: { input: void; output: TemplateInfo[] };
  save_template: { input: TemplateInput; output: TemplateInfo };
  delete_template: { input: { name: string }; output: string };
  resolve_template: { input: { name: string }; output: string[] };

  export_bundle: {
    input: { ids: string[]; path: string; label: string | null; template: string | null };
    output: ExportResult;
  };
  inspect_bundle: { input: { path: string }; output: BundlePreview };
  import_bundle: {
    input: { path: string; ids: string[]; mode: ImportMode };
    output: ImportReport;
  };

  list_backups: { input: void; output: BackupInfo[] };
  create_backup: {
    input: { ids: string[] | null; template: string | null; label: string | null };
    output: BackupInfo;
  };
  restore_backup: { input: { id: string; ids: string[] }; output: ImportReport };
  delete_backup: { input: { id: string }; output: string };

  p2p_start_share: { input: { ids: string[] }; output: ShareInfo };
  p2p_share_status: { input: void; output: ShareStatus };
  p2p_stop_share: { input: void; output: string };
  p2p_discover: { input: { timeout_ms: number }; output: PeerInfo[] };
  p2p_receive: { input: { address: string; code: string }; output: BundlePreview };

  get_config: { input: void; output: AppConfig };
  update_config: { input: AppConfig; output: AppConfig };
};

export type CommandName = keyof CommandSpec;
export type CommandInput<K extends CommandName> = CommandSpec[K]["input"];
export type CommandOutput<K extends CommandName> = CommandSpec[K]["output"];
