import type {
  AuthStatusInfo,
  DetectedSecret,
  PluginEntry,
  SkillEntry,
  SyncStatus,
} from "./types";

type CommandMap = {
  check_auth_status: AuthStatusInfo;
  check_git: string;
  get_default_device_id: string;
  get_status: SyncStatus;
  list_plugins: PluginEntry[];
  list_secrets: DetectedSecret[];
  list_skills: SkillEntry[];
  login_with_gh_cli: string;
  login_with_token: string;
  pull_selected_plugins: string;
  pull_selected_skills: string;
  run_setup: string;
  sync_pull: string;
  sync_push: string;
  push_selected_plugins: string;
  push_selected_skills: string;
  update_sync_options: string;
};

type CommandName = keyof CommandMap;

const mockState: {
  status: SyncStatus;
  auth: AuthStatusInfo;
  secrets: DetectedSecret[];
  plugins: PluginEntry[];
  skills: SkillEntry[];
} = {
  status: {
    initialized: true,
    device_id: "studio-laptop",
    repo_url: "git@github.com:team/claude-sync-config.git",
    last_sync: "2026-04-05 10:32",
    syncable_files: 42,
    skills_count: 14,
    ahead: 2,
    behind: 1,
    dirty_files: 3,
    git_available: true,
    sync_memory: true,
    sync_teams: true,
    sync_skills: true,
    sync_plugins: true,
    plugins_count: 6,
  },
  auth: {
    git_available: true,
    git_version: "git version 2.49.0.windows.1",
    gh_cli_available: true,
    gh_authenticated: true,
    gh_username: "winuser",
    ssh_key_found: true,
    ssh_keys: ["id_ed25519"],
  },
  secrets: [
    {
      json_path: "providers.openai.api_key",
      pattern_name: "API key",
      preview: "sk-...9A1b",
    },
    {
      json_path: "integrations.github.token",
      pattern_name: "GitHub token",
      preview: "ghp_...82kQ",
    },
  ],
  plugins: [
    {
      id: "github",
      name: "GitHub",
      marketplace: "OpenAI",
      version: "1.4.0",
      source_type: "marketplace",
      source_repo: null,
      enabled: true,
    },
    {
      id: "linear",
      name: "Linear",
      marketplace: "Community",
      version: "0.9.3",
      source_type: "git",
      source_repo: "github.com/acme/linear-plugin",
      enabled: true,
    },
    {
      id: "slack",
      name: "Slack",
      marketplace: "OpenAI",
      version: "1.1.2",
      source_type: "marketplace",
      source_repo: null,
      enabled: false,
    },
  ],
  skills: [
    {
      name: "frontend-skill",
      path: "~/.codex/skills/frontend-skill",
      size_bytes: 124000,
      file_count: 8,
      local_exists: true,
      remote_exists: true,
    },
    {
      name: "playwright-interactive",
      path: "~/.codex/skills/playwright-interactive",
      size_bytes: 98000,
      file_count: 6,
      local_exists: true,
      remote_exists: true,
    },
    {
      name: "release-notes",
      path: "~/.codex/skills/release-notes",
      size_bytes: 42000,
      file_count: 4,
      local_exists: true,
      remote_exists: false,
    },
  ],
};

function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function invokeTauri<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const mod = await import("@tauri-apps/api/core");
  return mod.invoke<T>(command, args);
}

function timestamp() {
  const now = new Date();
  return now.toISOString().slice(0, 16).replace("T", " ");
}

async function invokeMock<K extends CommandName>(
  command: K,
  args?: Record<string, unknown>,
): Promise<CommandMap[K]> {
  switch (command) {
    case "get_status":
      return structuredClone(mockState.status) as CommandMap[K];
    case "check_git":
      return mockState.auth.git_version as CommandMap[K];
    case "get_default_device_id":
      return mockState.status.device_id as CommandMap[K];
    case "check_auth_status":
      return structuredClone(mockState.auth) as CommandMap[K];
    case "run_setup": {
      const input = (args?.input ?? {}) as Record<string, unknown>;
      mockState.status.initialized = true;
      mockState.status.repo_url = String(input.repo_url ?? mockState.status.repo_url);
      mockState.status.device_id = String(input.device_id ?? mockState.status.device_id);
      mockState.status.sync_memory = Boolean(input.sync_memory);
      mockState.status.sync_teams = Boolean(input.sync_teams);
      mockState.status.sync_skills = Boolean(input.sync_skills);
      mockState.status.sync_plugins = Boolean(input.sync_plugins);
      mockState.status.last_sync = timestamp();
      return "Setup complete. Browser preview is now using the new demo configuration." as CommandMap[K];
    }
    case "login_with_gh_cli":
      mockState.auth.gh_authenticated = true;
      return "GitHub CLI session detected." as CommandMap[K];
    case "login_with_token":
      return "Token saved for browser preview." as CommandMap[K];
    case "list_secrets":
      return structuredClone(mockState.secrets) as CommandMap[K];
    case "list_plugins":
      return structuredClone(mockState.plugins) as CommandMap[K];
    case "list_skills":
      return structuredClone(mockState.skills) as CommandMap[K];
    case "sync_push":
      mockState.status.ahead = 0;
      mockState.status.dirty_files = 0;
      mockState.status.last_sync = timestamp();
      return "Pushed local changes to the demo repository." as CommandMap[K];
    case "sync_pull":
      mockState.status.behind = 0;
      mockState.status.last_sync = timestamp();
      return "Pulled remote changes into the demo workspace." as CommandMap[K];
    case "push_selected_plugins":
      mockState.status.last_sync = timestamp();
      return `Pushed ${(args?.ids as string[] | undefined)?.length ?? 0} plugin entries.` as CommandMap[K];
    case "pull_selected_plugins":
      mockState.status.last_sync = timestamp();
      return `Pulled ${(args?.ids as string[] | undefined)?.length ?? 0} plugin entries.` as CommandMap[K];
    case "push_selected_skills":
      mockState.status.last_sync = timestamp();
      return `Pushed ${(args?.names as string[] | undefined)?.length ?? 0} skills.` as CommandMap[K];
    case "pull_selected_skills":
      mockState.status.last_sync = timestamp();
      return `Pulled ${(args?.names as string[] | undefined)?.length ?? 0} skills.` as CommandMap[K];
    case "update_sync_options": {
      const opts = (args?.input ?? {}) as Record<string, unknown>;
      mockState.status.sync_memory = Boolean(opts.sync_memory);
      mockState.status.sync_teams = Boolean(opts.sync_teams);
      mockState.status.sync_skills = Boolean(opts.sync_skills);
      mockState.status.sync_plugins = Boolean(opts.sync_plugins);
      return "Sync options updated" as CommandMap[K];
    }
    default:
      throw new Error(`Unsupported mock command: ${command satisfies never}`);
  }
}

export async function invokeCommand<K extends CommandName>(
  command: K,
  args?: Record<string, unknown>,
): Promise<CommandMap[K]> {
  if (isTauriRuntime()) {
    return invokeTauri(command, args);
  }

  return invokeMock(command, args);
}
