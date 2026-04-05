export interface SyncStatus {
  initialized: boolean;
  device_id: string;
  repo_url: string;
  last_sync: string | null;
  syncable_files: number;
  skills_count: number;
  ahead: number;
  behind: number;
  dirty_files: number;
  git_available: boolean;
  sync_memory: boolean;
  sync_teams: boolean;
  sync_skills: boolean;
}

export interface DetectedSecret {
  json_path: string;
  pattern_name: string;
  preview: string;
}

export interface AuthStatusInfo {
  git_available: boolean;
  git_version: string | null;
  gh_cli_available: boolean;
  gh_authenticated: boolean;
  gh_username: string | null;
  ssh_key_found: boolean;
  ssh_keys: string[];
}

export interface SkillEntry {
  name: string;
  path: string;
  size_bytes: number;
  file_count: number;
  local_exists: boolean;
  remote_exists: boolean;
}
