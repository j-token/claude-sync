use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::config::SyncConfig;
use crate::error::{Result, SyncError};

/// 파일 카테고리
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FileCategory {
    ClaudeMd,
    Settings,
    SettingsLocal,
    McpJson,
    Rule,
    Command,
    Agent,
    Skill,
    Memory,
    Hook,
    Team,
    Plugin,
}

/// 싱크 가능한 파일 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncableFile {
    /// ~/.claude/ 기준 상대 경로
    pub relative_path: String,
    pub category: FileCategory,
    pub size_bytes: u64,
    pub last_modified: Option<SystemTime>,
}

/// 스킬 정보 (개별 선택용)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub files: Vec<String>,
}

/// 플러그인 정보 (메타데이터 기반 싱크)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// 플러그인 식별자 (e.g., "plannotator@plannotator")
    pub id: String,
    /// 마켓플레이스 이름
    pub marketplace: String,
    /// 플러그인 이름
    pub name: String,
    /// 버전
    pub version: String,
    /// GitHub 소스 정보
    pub source: Option<PluginSource>,
}

/// 플러그인 소스 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSource {
    pub source_type: String,
    pub repo: Option<String>,
    pub url: Option<String>,
}

/// 스킬 싱크 상태 (GUI에서 사용)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SkillSyncStatus {
    /// 로컬에만 존재
    LocalOnly,
    /// 원격에만 존재
    RemoteOnly,
    /// 양쪽 모두 존재
    Both,
    /// 양쪽 모두 존재하나 내용이 다름
    Modified,
}

/// 스킬 + 싱크 상태 (GUI 리스트용)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSyncInfo {
    pub skill: SkillInfo,
    pub status: SkillSyncStatus,
    /// 유저가 이 스킬을 싱크 대상으로 선택했는지
    pub selected: bool,
}

/// 건너뛴 파일 사유
#[derive(Debug, Clone)]
pub enum SkipReason {
    Credential,
    Cache,
    TooLarge(u64),
    Session,
    History,
    Excluded,
}

/// 건너뛴 파일 정보
#[derive(Debug, Clone)]
pub struct SkippedFile {
    pub path: PathBuf,
    pub reason: SkipReason,
}

/// 파일 탐색 결과
#[derive(Debug)]
pub struct DiscoveryResult {
    pub syncable: Vec<SyncableFile>,
    pub skills: Vec<SkillInfo>,
    pub plugins: Vec<PluginInfo>,
    pub skipped: Vec<SkippedFile>,
}

/// 절대 싱크하지 않는 파일/디렉토리 패턴
const NEVER_SYNC: &[&str] = &[
    ".credentials.json",
    "projects",
    "sessions",
    "history.jsonl",
    "cache",
    "image-cache",
    "paste-cache",
    "file-history",
    "debug",
    "telemetry",
    "shell-snapshots",
    "usage-data",
    "downloads",
    "ide",
    "statsig",
    "session-env",
    "backups",
    "stats-cache.json",
    "ai-token-monitor-cache.json",
    "ai-token-monitor-prefs.json",
    "mcp-needs-auth-cache.json",
    ".git",
    ".gitignore",
    "README.md",
    "aftman.toml",
    "default.project.json",
    "plans",
    "todos",
    "tasks",
    "channels",
];

/// ~/.claude/ 디렉토리를 스캔하여 싱크 가능한 파일 목록 반환
pub fn discover(config: &SyncConfig) -> Result<DiscoveryResult> {
    let claude_dir = SyncConfig::claude_dir();
    if !claude_dir.exists() {
        return Err(SyncError::Discovery(format!(
            "Claude directory not found: {}",
            claude_dir.display()
        )));
    }

    let mut syncable = Vec::new();
    let mut skills = Vec::new();
    let mut skipped = Vec::new();

    // 1. 단일 파일들
    discover_single_file(&claude_dir, "CLAUDE.md", FileCategory::ClaudeMd, &mut syncable);
    discover_single_file(
        &claude_dir,
        "settings.json",
        FileCategory::Settings,
        &mut syncable,
    );
    discover_single_file(
        &claude_dir,
        "settings.local.json",
        FileCategory::SettingsLocal,
        &mut syncable,
    );
    discover_single_file(&claude_dir, ".mcp.json", FileCategory::McpJson, &mut syncable);

    // 2. 디렉토리 기반 파일들
    discover_directory(&claude_dir, "rules", FileCategory::Rule, &mut syncable)?;
    discover_directory(&claude_dir, "commands", FileCategory::Command, &mut syncable)?;
    discover_directory(&claude_dir, "agents", FileCategory::Agent, &mut syncable)?;
    discover_directory(&claude_dir, "hooks", FileCategory::Hook, &mut syncable)?;

    // 3. 선택적 싱크: teams
    if config.sync.sync_teams {
        discover_directory_recursive(&claude_dir, "teams", FileCategory::Team, &mut syncable)?;
    }

    // 4. 선택적 싱크: memory
    if config.sync.sync_memory {
        discover_directory(&claude_dir, "memory", FileCategory::Memory, &mut syncable)?;
    }

    // 5. 선택적 싱크: skills (개별 선택 지원)
    if config.sync.sync_skills {
        skills = discover_skills(&claude_dir)?;
    }

    // 6. 선택적 싱크: plugins (메타데이터만)
    let mut plugins = Vec::new();
    if config.sync.sync_plugins {
        plugins = discover_plugins(&claude_dir)?;
        // 플러그인 메타데이터 파일도 싱크 대상에 추가
        discover_single_file(
            &claude_dir,
            "plugins/installed_plugins.json",
            FileCategory::Plugin,
            &mut syncable,
        );
        discover_single_file(
            &claude_dir,
            "plugins/known_marketplaces.json",
            FileCategory::Plugin,
            &mut syncable,
        );
    }

    // 7. 건너뛴 파일 목록 구성
    if let Ok(entries) = std::fs::read_dir(&claude_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if NEVER_SYNC.contains(&name.as_str()) {
                let reason = match name.as_str() {
                    ".credentials.json" => SkipReason::Credential,
                    "sessions" | "session-env" => SkipReason::Session,
                    "history.jsonl" => SkipReason::History,
                    "cache" | "image-cache" | "paste-cache" | "file-history" => SkipReason::Cache,
                    _ => SkipReason::Excluded,
                };
                skipped.push(SkippedFile {
                    path: entry.path(),
                    reason,
                });
            }
        }
    }

    Ok(DiscoveryResult {
        syncable,
        skills,
        plugins,
        skipped,
    })
}

/// plugins 메타데이터에서 플러그인 목록 파싱
fn discover_plugins(claude_dir: &Path) -> Result<Vec<PluginInfo>> {
    let installed_path = claude_dir.join("plugins/installed_plugins.json");
    let marketplaces_path = claude_dir.join("plugins/known_marketplaces.json");

    if !installed_path.exists() {
        return Ok(Vec::new());
    }

    // installed_plugins.json 파싱
    let installed_content = std::fs::read_to_string(&installed_path)
        .map_err(|e| SyncError::Discovery(format!("Failed to read installed_plugins.json: {e}")))?;
    let installed: serde_json::Value = serde_json::from_str(&installed_content)
        .map_err(|e| SyncError::Discovery(format!("Failed to parse installed_plugins.json: {e}")))?;

    // known_marketplaces.json 파싱 (소스 정보용)
    let marketplaces: serde_json::Value = if marketplaces_path.exists() {
        let content = std::fs::read_to_string(&marketplaces_path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or(serde_json::Value::Object(Default::default()))
    } else {
        serde_json::Value::Object(Default::default())
    };

    let mut plugins = Vec::new();

    if let Some(plugin_map) = installed.get("plugins").and_then(|p| p.as_object()) {
        for (id, entries) in plugin_map {
            // id 형식: "name@marketplace"
            let parts: Vec<&str> = id.splitn(2, '@').collect();
            let (name, marketplace) = if parts.len() == 2 {
                (parts[0].to_string(), parts[1].to_string())
            } else {
                (id.clone(), "unknown".to_string())
            };

            // 최신 버전 찾기
            let version = entries
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|e| e.get("version"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            // 마켓플레이스에서 소스 정보 찾기
            let source = marketplaces
                .get(&marketplace)
                .and_then(|m| m.get("source"))
                .map(|s| PluginSource {
                    source_type: s
                        .get("source")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    repo: s.get("repo").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    url: s.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()),
                });

            plugins.push(PluginInfo {
                id: id.clone(),
                marketplace,
                name,
                version,
                source,
            });
        }
    }

    Ok(plugins)
}

/// skills 디렉토리의 각 스킬을 개별 식별
fn discover_skills(claude_dir: &Path) -> Result<Vec<SkillInfo>> {
    let skills_dir = claude_dir.join("skills");
    if !skills_dir.exists() {
        return Ok(Vec::new());
    }

    let mut skills = Vec::new();

    for entry in std::fs::read_dir(&skills_dir)
        .map_err(|e| SyncError::Discovery(format!("Failed to read skills directory: {e}")))?
    {
        let entry = entry.map_err(|e| SyncError::Discovery(e.to_string()))?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let mut total_size = 0u64;
        let mut files = Vec::new();

        collect_files_recursive(&path, &path, &mut files, &mut total_size)?;

        skills.push(SkillInfo {
            name,
            path: format!("skills/{}", entry.file_name().to_string_lossy()),
            size_bytes: total_size,
            files,
        });
    }

    Ok(skills)
}

/// 재귀적으로 파일 목록과 총 크기 수집
fn collect_files_recursive(
    base: &Path,
    dir: &Path,
    files: &mut Vec<String>,
    total_size: &mut u64,
) -> Result<()> {
    for entry in
        std::fs::read_dir(dir).map_err(|e| SyncError::Discovery(e.to_string()))?
    {
        let entry = entry.map_err(|e| SyncError::Discovery(e.to_string()))?;
        let path = entry.path();

        if path.is_dir() {
            // node_modules 등 불필요한 디렉토리 건너뛰기
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "node_modules" || name == ".git" || name == "target" {
                continue;
            }
            collect_files_recursive(base, &path, files, total_size)?;
        } else {
            let relative = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            *total_size += size;
            files.push(relative);
        }
    }
    Ok(())
}

fn discover_single_file(
    claude_dir: &Path,
    filename: &str,
    category: FileCategory,
    result: &mut Vec<SyncableFile>,
) {
    let path = claude_dir.join(filename);
    if path.exists() && path.is_file() {
        let metadata = std::fs::metadata(&path).ok();
        result.push(SyncableFile {
            relative_path: filename.to_string(),
            category,
            size_bytes: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
            last_modified: metadata.and_then(|m| m.modified().ok()),
        });
    }
}

fn discover_directory(
    claude_dir: &Path,
    dirname: &str,
    category: FileCategory,
    result: &mut Vec<SyncableFile>,
) -> Result<()> {
    let dir = claude_dir.join(dirname);
    if !dir.exists() || !dir.is_dir() {
        return Ok(());
    }

    for entry in
        std::fs::read_dir(&dir).map_err(|e| SyncError::Discovery(e.to_string()))?
    {
        let entry = entry.map_err(|e| SyncError::Discovery(e.to_string()))?;
        let path = entry.path();

        if path.is_file() {
            let relative = format!("{}/{}", dirname, entry.file_name().to_string_lossy());
            let metadata = std::fs::metadata(&path).ok();
            result.push(SyncableFile {
                relative_path: relative,
                category: category.clone(),
                size_bytes: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                last_modified: metadata.and_then(|m| m.modified().ok()),
            });
        }
    }
    Ok(())
}

fn discover_directory_recursive(
    claude_dir: &Path,
    dirname: &str,
    category: FileCategory,
    result: &mut Vec<SyncableFile>,
) -> Result<()> {
    let dir = claude_dir.join(dirname);
    if !dir.exists() || !dir.is_dir() {
        return Ok(());
    }
    discover_dir_inner(&dir, dirname, &category, result)
}

fn discover_dir_inner(
    dir: &Path,
    prefix: &str,
    category: &FileCategory,
    result: &mut Vec<SyncableFile>,
) -> Result<()> {
    for entry in
        std::fs::read_dir(dir).map_err(|e| SyncError::Discovery(e.to_string()))?
    {
        let entry = entry.map_err(|e| SyncError::Discovery(e.to_string()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            let sub_prefix = format!("{}/{}", prefix, name);
            discover_dir_inner(&path, &sub_prefix, category, result)?;
        } else {
            let relative = format!("{}/{}", prefix, name);
            let metadata = std::fs::metadata(&path).ok();
            result.push(SyncableFile {
                relative_path: relative,
                category: category.clone(),
                size_bytes: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                last_modified: metadata.and_then(|m| m.modified().ok()),
            });
        }
    }
    Ok(())
}
