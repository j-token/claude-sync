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

export interface SkillEntry {
  name: string;
  path: string;
  size_bytes: number;
  file_count: number;
  local_exists: boolean;
  remote_exists: boolean;
}
