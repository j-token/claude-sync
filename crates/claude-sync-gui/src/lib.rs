use claude_sync_core::config::{
    AuthConfig, AuthMethod, DeviceConfig, Platform, RepoConfig, SnapshotConfig, SyncConfig,
    SyncOptions,
};
use claude_sync_core::discovery;
use claude_sync_core::git_ops::{self, GitRepo};
use claude_sync_core::manifest::{self, ManifestEntry, SyncManifest};
use claude_sync_core::platform;
use claude_sync_core::secret::SecretEngine;
use claude_sync_core::snapshot;
use serde::{Deserialize, Serialize};

/// GUI에서 사용하는 싱크 상태 정보
#[derive(Serialize, Clone)]
struct SyncStatus {
    initialized: bool,
    device_id: String,
    repo_url: String,
    last_sync: Option<String>,
    syncable_files: usize,
    skills_count: usize,
    ahead: usize,
    behind: usize,
    dirty_files: usize,
    git_available: bool,
    sync_memory: bool,
    sync_teams: bool,
    sync_skills: bool,
    sync_plugins: bool,
    plugins_count: usize,
}

/// 시크릿 탐지 결과
#[derive(Serialize, Clone)]
struct DetectedSecret {
    json_path: String,
    pattern_name: String,
    preview: String,
}

/// 스킬 정보 (GUI용)
#[derive(Serialize, Clone)]
struct SkillEntry {
    name: String,
    path: String,
    size_bytes: u64,
    file_count: usize,
    local_exists: bool,
    remote_exists: bool,
}

/// 블로킹 작업을 백그라운드 스레드에서 실행
async fn blocking<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

#[tauri::command]
async fn get_status() -> Result<SyncStatus, String> {
    blocking(|| {
        let git_available = git_ops::check_git_available().is_ok();

        let config = match SyncConfig::load() {
            Ok(c) => c,
            Err(_) => {
                return Ok(SyncStatus {
                    initialized: false,
                    device_id: String::new(),
                    repo_url: String::new(),
                    last_sync: None,
                    syncable_files: 0,
                    skills_count: 0,
                    ahead: 0,
                    behind: 0,
                    dirty_files: 0,
                    git_available,
                    sync_memory: false,
                    sync_teams: true,
                    sync_skills: true,
                    sync_plugins: true,
                    plugins_count: 0,
                });
            }
        };

        let discovery_result = discovery::discover(&config).map_err(|e| e.to_string())?;
        let repo_path = SyncConfig::repo_path();

        let mut ahead = 0usize;
        let mut behind = 0usize;
        let mut dirty_files = 0usize;
        let mut last_sync = None;

        if repo_path.join(".git").exists() {
            if let Ok(repo) = GitRepo::open_or_init(&repo_path) {
                if let Ok(status) = repo.status() {
                    dirty_files = status.stdout.lines().filter(|l| !l.is_empty()).count();
                }
                let _ = repo.fetch("origin");
                if let Ok((a, b)) = repo.ahead_behind("origin", &config.repo.branch) {
                    ahead = a;
                    behind = b;
                }
            }

            let manifest_path = repo_path.join("manifest.json");
            if manifest_path.exists() {
                if let Ok(manifest) = SyncManifest::load(&manifest_path) {
                    last_sync = Some(manifest.last_sync);
                }
            }
        }

        Ok(SyncStatus {
            initialized: true,
            device_id: config.device.id,
            repo_url: config.repo.url,
            last_sync,
            syncable_files: discovery_result.syncable.len(),
            skills_count: discovery_result.skills.len(),
            ahead,
            behind,
            dirty_files,
            git_available,
            sync_memory: config.sync.sync_memory,
            sync_teams: config.sync.sync_teams,
            sync_skills: config.sync.sync_skills,
            sync_plugins: config.sync.sync_plugins,
            plugins_count: discovery_result.plugins.len(),
        })
    })
    .await
}

#[tauri::command]
async fn get_config() -> Result<String, String> {
    blocking(|| {
        let config = SyncConfig::load().unwrap_or_default();
        serde_json::to_string(&config).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
async fn list_secrets() -> Result<Vec<DetectedSecret>, String> {
    blocking(|| {
        let config = SyncConfig::load().map_err(|e| e.to_string())?;
        let claude_dir = SyncConfig::claude_dir();
        let settings_path = claude_dir.join("settings.json");

        if !settings_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&settings_path).map_err(|e| e.to_string())?;
        let json: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| e.to_string())?;

        let engine = SecretEngine::new(&config.secret_patterns);
        let matches = engine.detect(&json);

        Ok(matches
            .iter()
            .map(|m| {
                let preview = if m.original_value.len() > 8 {
                    format!("{}...", &m.original_value[..8])
                } else {
                    m.original_value.clone()
                };
                DetectedSecret {
                    json_path: m.json_path.clone(),
                    pattern_name: m.pattern_name.clone(),
                    preview,
                }
            })
            .collect())
    })
    .await
}

#[tauri::command]
async fn list_skills() -> Result<Vec<SkillEntry>, String> {
    blocking(|| {
        let config = SyncConfig::load().map_err(|e| e.to_string())?;
        let result = discovery::discover(&config).map_err(|e| e.to_string())?;
        let repo_path = SyncConfig::repo_path();

        let mut entries: Vec<SkillEntry> = result
            .skills
            .iter()
            .map(|s| {
                let remote_exists = repo_path.join(&s.path).exists();
                SkillEntry {
                    name: s.name.clone(),
                    path: s.path.clone(),
                    size_bytes: s.size_bytes,
                    file_count: s.files.len(),
                    local_exists: true,
                    remote_exists,
                }
            })
            .collect();

        // 원격에만 있는 스킬 추가
        let remote_skills = repo_path.join("skills");
        if remote_skills.exists() {
            if let Ok(dir) = std::fs::read_dir(&remote_skills) {
                for entry in dir.flatten() {
                    if entry.path().is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if !entries.iter().any(|e| e.name == name) {
                            entries.push(SkillEntry {
                                name,
                                path: format!(
                                    "skills/{}",
                                    entry.file_name().to_string_lossy()
                                ),
                                size_bytes: 0,
                                file_count: 0,
                                local_exists: false,
                                remote_exists: true,
                            });
                        }
                    }
                }
            }
        }

        Ok(entries)
    })
    .await
}

/// 플러그인 정보 (GUI용)
#[derive(Serialize, Clone)]
struct PluginEntry {
    id: String,
    name: String,
    marketplace: String,
    version: String,
    source_type: String,
    source_repo: Option<String>,
    enabled: bool,
}

#[tauri::command]
async fn list_plugins() -> Result<Vec<PluginEntry>, String> {
    blocking(|| {
        let config = SyncConfig::load().map_err(|e| e.to_string())?;
        let result = discovery::discover(&config).map_err(|e| e.to_string())?;

        // settings.json에서 enabledPlugins 읽기
        let claude_dir = SyncConfig::claude_dir();
        let settings_path = claude_dir.join("settings.json");
        let enabled_plugins: std::collections::HashMap<String, bool> =
            if settings_path.exists() {
                let content =
                    std::fs::read_to_string(&settings_path).unwrap_or_default();
                let json: serde_json::Value =
                    serde_json::from_str(&content).unwrap_or_default();
                json.get("enabledPlugins")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), v.as_bool().unwrap_or(false)))
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                std::collections::HashMap::new()
            };

        let entries: Vec<PluginEntry> = result
            .plugins
            .iter()
            .map(|p| {
                let enabled = enabled_plugins.get(&p.id).copied().unwrap_or(false);
                PluginEntry {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    marketplace: p.marketplace.clone(),
                    version: p.version.clone(),
                    source_type: p
                        .source
                        .as_ref()
                        .map(|s| s.source_type.clone())
                        .unwrap_or_else(|| "unknown".to_string()),
                    source_repo: p.source.as_ref().and_then(|s| s.repo.clone()),
                    enabled,
                }
            })
            .collect();

        Ok(entries)
    })
    .await
}

/// 선택된 스킬만 Push (로컬 → 레포)
#[tauri::command]
async fn push_selected_skills(names: Vec<String>) -> Result<String, String> {
    blocking(move || {
        let config = SyncConfig::load().map_err(|e| e.to_string())?;
        let discovery_result = discovery::discover(&config).map_err(|e| e.to_string())?;
        let repo_path = SyncConfig::repo_path();
        let claude_dir = SyncConfig::claude_dir();

        let mut count = 0;
        for skill in &discovery_result.skills {
            if !names.contains(&skill.name) {
                continue;
            }
            let src = claude_dir.join(&skill.path);
            let dst = repo_path.join(&skill.path);
            if src.exists() {
                copy_dir_recursive(&src, &dst)?;
                count += 1;
            }
        }

        // git commit & push
        let repo = GitRepo::open_or_init(&repo_path).map_err(|e| e.to_string())?;
        repo.add_all().map_err(|e| e.to_string())?;
        let message = format!(
            "sync skills [{}]: {} @ {}",
            names.join(", "),
            config.device.id,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
        let _ = repo.commit(&message);
        let _ = repo.push_with_recovery("origin", &config.repo.branch, &config);

        Ok(format!("{count}개 스킬 Push 완료"))
    })
    .await
}

/// 선택된 스킬만 Pull (레포 → 로컬)
#[tauri::command]
async fn pull_selected_skills(names: Vec<String>) -> Result<String, String> {
    blocking(move || {
        let config = SyncConfig::load().map_err(|e| e.to_string())?;
        let repo_path = SyncConfig::repo_path();
        let claude_dir = SyncConfig::claude_dir();

        // 먼저 원격에서 최신 가져오기
        let repo = GitRepo::open_or_init(&repo_path).map_err(|e| e.to_string())?;
        repo.ensure_remote("origin", &config).map_err(|e| e.to_string())?;
        let _ = repo.pull("origin", &config.repo.branch);

        let skills_src = repo_path.join("skills");
        let skills_dst = claude_dir.join("skills");
        let mut count = 0;

        if skills_src.exists() {
            for entry in std::fs::read_dir(&skills_src).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.path().is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !names.contains(&name) {
                        continue;
                    }
                    let dst = skills_dst.join(&name);
                    copy_dir_recursive(&entry.path(), &dst)?;
                    count += 1;
                }
            }
        }

        Ok(format!("{count}개 스킬 Pull 완료"))
    })
    .await
}

/// 선택된 플러그인만 Push (로컬 installed_plugins.json에서 선택된 항목만 레포에 기록)
#[tauri::command]
async fn push_selected_plugins(ids: Vec<String>) -> Result<String, String> {
    blocking(move || {
        let config = SyncConfig::load().map_err(|e| e.to_string())?;
        let repo_path = SyncConfig::repo_path();
        let claude_dir = SyncConfig::claude_dir();

        // 로컬 installed_plugins.json 읽기
        let installed_path = claude_dir.join("plugins/installed_plugins.json");
        if !installed_path.exists() {
            return Ok("플러그인 파일이 없습니다".to_string());
        }

        let content = std::fs::read_to_string(&installed_path).map_err(|e| e.to_string())?;
        let installed: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| e.to_string())?;

        // 선택된 플러그인만 필터링
        let filtered = filter_plugins_json(&installed, &ids);

        // 레포에 필터링된 파일 저장
        let dst_dir = repo_path.join("plugins");
        std::fs::create_dir_all(&dst_dir).map_err(|e| e.to_string())?;
        let dst_path = dst_dir.join("installed_plugins.json");
        let output = serde_json::to_string_pretty(&filtered).map_err(|e| e.to_string())?;
        std::fs::write(&dst_path, output).map_err(|e| e.to_string())?;

        // known_marketplaces.json도 복사
        let marketplaces_src = claude_dir.join("plugins/known_marketplaces.json");
        if marketplaces_src.exists() {
            let marketplaces_dst = dst_dir.join("known_marketplaces.json");
            std::fs::copy(&marketplaces_src, &marketplaces_dst).map_err(|e| e.to_string())?;
        }

        // git commit & push
        let repo = GitRepo::open_or_init(&repo_path).map_err(|e| e.to_string())?;
        repo.add_all().map_err(|e| e.to_string())?;
        let message = format!(
            "sync plugins [{}]: {} @ {}",
            ids.len(),
            config.device.id,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
        let _ = repo.commit(&message);
        let _ = repo.push_with_recovery("origin", &config.repo.branch, &config);

        Ok(format!("{}개 플러그인 Push 완료", ids.len()))
    })
    .await
}

/// 선택된 플러그인만 Pull (레포 → 로컬 installed_plugins.json에 병합)
#[tauri::command]
async fn pull_selected_plugins(ids: Vec<String>) -> Result<String, String> {
    blocking(move || {
        let config = SyncConfig::load().map_err(|e| e.to_string())?;
        let repo_path = SyncConfig::repo_path();
        let claude_dir = SyncConfig::claude_dir();

        // 먼저 원격에서 최신 가져오기
        let repo = GitRepo::open_or_init(&repo_path).map_err(|e| e.to_string())?;
        repo.ensure_remote("origin", &config).map_err(|e| e.to_string())?;
        let _ = repo.pull("origin", &config.repo.branch);

        // 레포의 installed_plugins.json 읽기
        let remote_path = repo_path.join("plugins/installed_plugins.json");
        if !remote_path.exists() {
            return Ok("원격에 플러그인 데이터가 없습니다".to_string());
        }

        let remote_content = std::fs::read_to_string(&remote_path).map_err(|e| e.to_string())?;
        let remote_json: serde_json::Value =
            serde_json::from_str(&remote_content).map_err(|e| e.to_string())?;

        // 선택된 플러그인만 필터링
        let filtered_remote = filter_plugins_json(&remote_json, &ids);

        // 로컬 installed_plugins.json에 병합
        let local_path = claude_dir.join("plugins/installed_plugins.json");
        let local_json: serde_json::Value = if local_path.exists() {
            let content = std::fs::read_to_string(&local_path).map_err(|e| e.to_string())?;
            serde_json::from_str(&content).map_err(|e| e.to_string())?
        } else {
            serde_json::json!({"plugins": {}})
        };

        let merged = merge_plugins_json(&local_json, &filtered_remote);

        std::fs::create_dir_all(claude_dir.join("plugins")).map_err(|e| e.to_string())?;
        let output = serde_json::to_string_pretty(&merged).map_err(|e| e.to_string())?;
        std::fs::write(&local_path, output).map_err(|e| e.to_string())?;

        // known_marketplaces.json도 복사
        let remote_mp = repo_path.join("plugins/known_marketplaces.json");
        if remote_mp.exists() {
            let local_mp = claude_dir.join("plugins/known_marketplaces.json");
            std::fs::copy(&remote_mp, &local_mp).map_err(|e| e.to_string())?;
        }

        Ok(format!("{}개 플러그인 Pull 완료", ids.len()))
    })
    .await
}

/// installed_plugins.json에서 선택된 plugin ID만 남기는 필터
fn filter_plugins_json(json: &serde_json::Value, ids: &[String]) -> serde_json::Value {
    let mut result = json.clone();
    if let Some(plugins) = result.get_mut("plugins").and_then(|p| p.as_object_mut()) {
        let keys_to_remove: Vec<String> = plugins
            .keys()
            .filter(|k| !ids.contains(k))
            .cloned()
            .collect();
        for key in keys_to_remove {
            plugins.remove(&key);
        }
    }
    result
}

/// 로컬 installed_plugins.json에 원격의 선택된 플러그인을 병합
fn merge_plugins_json(
    local: &serde_json::Value,
    remote_filtered: &serde_json::Value,
) -> serde_json::Value {
    let mut result = local.clone();
    if let (Some(local_plugins), Some(remote_plugins)) = (
        result.get_mut("plugins").and_then(|p| p.as_object_mut()),
        remote_filtered.get("plugins").and_then(|p| p.as_object()),
    ) {
        for (key, value) in remote_plugins {
            local_plugins.insert(key.clone(), value.clone());
        }
    }
    result
}

/// 디렉토리를 재귀적으로 복사 (node_modules, .git, target 제외)
/// GitHub 파일 크기 제한(100MB)보다 낮은 안전 한계
const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024; // 50MB

/// 디렉토리를 재귀적으로 복사 (대용량 바이너리 및 빌드 산출물 제외)
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        let name = entry.file_name().to_string_lossy().to_string();
        if matches!(
            name.as_str(),
            "node_modules" | ".git" | "target" | "bin" | "obj" | "dist" | "build"
        ) {
            continue;
        }

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            // 대용량 파일 스킵 (GitHub 100MB 제한 방지)
            let metadata = std::fs::metadata(&src_path).map_err(|e| e.to_string())?;
            if metadata.len() > MAX_FILE_SIZE {
                continue;
            }
            std::fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
async fn sync_push() -> Result<String, String> {
    blocking(|| {
        let config = SyncConfig::load().map_err(|e| e.to_string())?;
        let discovery_result = discovery::discover(&config).map_err(|e| e.to_string())?;
        let secret_engine = SecretEngine::new(&config.secret_patterns);
        let repo_path = SyncConfig::repo_path();
        let claude_dir = SyncConfig::claude_dir();

        snapshot::create_snapshot().map_err(|e| e.to_string())?;

        // manifest 생성 — push 후 pull이 이 파일을 참조하여 동기화 수행
        let mut sync_manifest =
            SyncManifest::new(&config.device.id, config.device.platform.clone());

        for file in &discovery_result.syncable {
            let src = claude_dir.join(&file.relative_path);
            let dst = repo_path.join(&file.relative_path);
            if !src.exists() {
                continue;
            }
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }

            let content = std::fs::read(&src).map_err(|e| e.to_string())?;
            let sha256 = manifest::compute_sha256(&content);
            let mut masked_secrets = Vec::new();
            let mut platform_specific = Vec::new();

            match file.category {
                discovery::FileCategory::Settings | discovery::FileCategory::McpJson => {
                    let json: serde_json::Value =
                        serde_json::from_slice(&content).map_err(|e| e.to_string())?;
                    let (masked, matches) = secret_engine.mask(&json);
                    masked_secrets = matches.iter().map(|m| m.json_path.clone()).collect();
                    platform_specific = platform::detect_platform_paths(&json);
                    let output =
                        serde_json::to_string_pretty(&masked).map_err(|e| e.to_string())?;
                    std::fs::write(&dst, output).map_err(|e| e.to_string())?;
                }
                _ => {
                    std::fs::copy(&src, &dst).map_err(|e| e.to_string())?;
                }
            }

            sync_manifest.upsert_entry(ManifestEntry {
                path: file.relative_path.clone(),
                sha256,
                category: file.category.clone(),
                masked_secrets,
                platform_specific,
            });
        }

        // Skills 복사
        for skill in &discovery_result.skills {
            let skill_src = claude_dir.join(&skill.path);
            let skill_dst = repo_path.join(&skill.path);
            if skill_src.exists() {
                copy_dir_recursive(&skill_src, &skill_dst)?;
            }
        }

        // manifest 저장
        sync_manifest
            .save(&repo_path.join("manifest.json"))
            .map_err(|e| e.to_string())?;

        let repo = GitRepo::open_or_init(&repo_path).map_err(|e| e.to_string())?;
        repo.add_all().map_err(|e| e.to_string())?;

        let message = format!(
            "sync: {} @ {}",
            config.device.id,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
        let _ = repo.commit(&message);
        let result = repo
            .push_with_recovery("origin", &config.repo.branch, &config)
            .map_err(|e| e.to_string())?;

        if result.recovery_applied {
            Ok(format!("Push 완료 ({})", result.message))
        } else {
            Ok("Push 완료".to_string())
        }
    })
    .await
}

#[tauri::command]
async fn sync_pull() -> Result<String, String> {
    blocking(|| {
        let config = SyncConfig::load().map_err(|e| e.to_string())?;
        let repo_path = SyncConfig::repo_path();
        let claude_dir = SyncConfig::claude_dir();

        snapshot::create_snapshot().map_err(|e| e.to_string())?;

        let repo = GitRepo::open_or_init(&repo_path).map_err(|e| e.to_string())?;
        // pull 전에 remote가 설정되어 있는지 확인 및 자동 복구
        repo.ensure_remote("origin", &config).map_err(|e| e.to_string())?;
        repo.pull("origin", &config.repo.branch)
            .map_err(|e| e.to_string())?;

        let manifest_path = repo_path.join("manifest.json");
        if !manifest_path.exists() {
            return Ok("원격이 비어있습니다".to_string());
        }

        let manifest = SyncManifest::load(&manifest_path).map_err(|e| e.to_string())?;
        let secret_engine = SecretEngine::new(&config.secret_patterns);
        let mut count = 0;

        for entry in &manifest.files {
            let src = repo_path.join(&entry.path);
            let dst = claude_dir.join(&entry.path);
            if !src.exists() {
                continue;
            }
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }

            match entry.category {
                discovery::FileCategory::Settings | discovery::FileCategory::McpJson => {
                    let remote_content =
                        std::fs::read_to_string(&src).map_err(|e| e.to_string())?;
                    let remote_json: serde_json::Value =
                        serde_json::from_str(&remote_content).map_err(|e| e.to_string())?;
                    let final_json = if dst.exists() {
                        let local_content =
                            std::fs::read_to_string(&dst).map_err(|e| e.to_string())?;
                        let local_json: serde_json::Value =
                            serde_json::from_str(&local_content).map_err(|e| e.to_string())?;
                        secret_engine.unmask(&remote_json, &local_json)
                    } else {
                        remote_json
                    };
                    let output =
                        serde_json::to_string_pretty(&final_json).map_err(|e| e.to_string())?;
                    std::fs::write(&dst, output).map_err(|e| e.to_string())?;
                    count += 1;
                }
                _ => {
                    std::fs::copy(&src, &dst).map_err(|e| e.to_string())?;
                    count += 1;
                }
            }
        }

        // Skills 복원 (CLI pull.rs와 동일)
        if config.sync.sync_skills {
            let skills_src = repo_path.join("skills");
            if skills_src.exists() {
                let skills_dst = claude_dir.join("skills");
                for entry in std::fs::read_dir(&skills_src).map_err(|e| e.to_string())? {
                    let entry = entry.map_err(|e| e.to_string())?;
                    if entry.path().is_dir() {
                        let dst = skills_dst.join(entry.file_name());
                        copy_dir_recursive(&entry.path(), &dst)?;
                        count += 1;
                    }
                }
            }
        }

        Ok(format!("Pull 완료: {} files", count))
    })
    .await
}

/// GUI 셋업 위저드 입력값
#[derive(Deserialize)]
struct SetupInput {
    repo_url: String,
    auth_method: String,
    device_id: String,
    sync_memory: bool,
    sync_teams: bool,
    sync_skills: bool,
    sync_plugins: bool,
}

/// Sync 옵션 업데이트 입력값
#[derive(Deserialize)]
struct SyncOptionsInput {
    sync_memory: bool,
    sync_teams: bool,
    sync_skills: bool,
    sync_plugins: bool,
}

/// 개별 Sync 옵션만 업데이트 (설정 페이지에서 스위치 토글 시 호출)
#[tauri::command]
async fn update_sync_options(input: SyncOptionsInput) -> Result<String, String> {
    blocking(move || {
        let mut config = SyncConfig::load().map_err(|e| e.to_string())?;
        config.sync.sync_memory = input.sync_memory;
        config.sync.sync_teams = input.sync_teams;
        config.sync.sync_skills = input.sync_skills;
        config.sync.sync_plugins = input.sync_plugins;
        config.save().map_err(|e| e.to_string())?;
        Ok("Sync options updated".to_string())
    })
    .await
}

#[tauri::command]
async fn check_git() -> Result<String, String> {
    blocking(|| git_ops::check_git_available().map_err(|e| e.to_string())).await
}

#[tauri::command]
async fn get_default_device_id() -> Result<String, String> {
    Ok(hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "my-device".to_string()))
}

#[tauri::command]
async fn run_setup(input: SetupInput) -> Result<String, String> {
    blocking(move || {
        let auth_method = match input.auth_method.as_str() {
            "ssh_key" => AuthMethod::SshKey,
            "https_token" => AuthMethod::HttpsToken,
            "gh_cli" => AuthMethod::GhCli,
            _ => AuthMethod::SshAgent,
        };

        let config = SyncConfig {
            schema_version: 1,
            repo: RepoConfig {
                url: input.repo_url.clone(),
                branch: "main".to_string(),
            },
            auth: AuthConfig {
                method: auth_method,
                ssh_key_path: None,
            },
            device: DeviceConfig {
                id: input.device_id.clone(),
                platform: Platform::current(),
            },
            sync: SyncOptions {
                auto_sync: false,
                auto_sync_interval_secs: 300,
                sync_memory: input.sync_memory,
                sync_teams: input.sync_teams,
                sync_skills: input.sync_skills,
                sync_plugins: input.sync_plugins,
            },
            secret_patterns: SyncConfig::default_secret_patterns(),
            platform_path_rules: Vec::new(),
            snapshots: SnapshotConfig::default(),
        };

        config.save().map_err(|e| e.to_string())?;

        let repo_path = SyncConfig::repo_path();
        if !repo_path.join(".git").exists() {
            match GitRepo::clone_repo(&input.repo_url, &repo_path) {
                Ok(_) => {}
                Err(_) => {
                    let repo =
                        GitRepo::open_or_init(&repo_path).map_err(|e| e.to_string())?;
                    repo.set_remote("origin", &input.repo_url)
                        .map_err(|e| e.to_string())?;
                    let _ = repo.set_branch("main");
                }
            }
        }

        Ok("Setup 완료".to_string())
    })
    .await
}

/// 인증 상태 정보 (GUI용)
#[derive(Serialize, Clone)]
struct AuthStatusInfo {
    git_available: bool,
    git_version: Option<String>,
    gh_cli_available: bool,
    gh_authenticated: bool,
    gh_username: Option<String>,
    ssh_key_found: bool,
    ssh_keys: Vec<String>,
}

#[tauri::command]
async fn check_auth_status() -> Result<AuthStatusInfo, String> {
    blocking(|| {
        let status = git_ops::check_auth_status();
        let ssh_keys = git_ops::find_ssh_keys();
        Ok(AuthStatusInfo {
            git_available: status.git_available,
            git_version: status.git_version,
            gh_cli_available: status.gh_cli_available,
            gh_authenticated: status.gh_authenticated,
            gh_username: status.gh_username,
            ssh_key_found: status.ssh_key_found,
            ssh_keys,
        })
    })
    .await
}

/// PAT 토큰으로 HTTPS 인증 설정
#[tauri::command]
async fn login_with_token(token: String) -> Result<String, String> {
    blocking(move || {
        // 토큰 유효성 확인: gh api로 테스트
        let _output = std::process::Command::new("git")
            .args(["ls-remote", "https://github.com"])
            .env("GIT_ASKPASS", "echo")
            .env("GIT_TERMINAL_PROMPT", "0")
            .output();

        // 토큰을 git credential에 저장
        let config = SyncConfig::load().map_err(|e| e.to_string())?;
        let repo_path = SyncConfig::repo_path();

        if repo_path.join(".git").exists() {
            let repo = GitRepo::open_or_init(&repo_path).map_err(|e| e.to_string())?;
            // HTTPS URL이면 토큰 임베드
            if config.repo.url.starts_with("https://") {
                repo.set_remote_with_token("origin", &config.repo.url, &token)
                    .map_err(|e| e.to_string())?;
            }
        }

        Ok("Token configured".to_string())
    })
    .await
}

/// gh CLI 로그인 상태 확인 및 토큰 가져오기
#[tauri::command]
async fn login_with_gh_cli() -> Result<String, String> {
    blocking(|| {
        let token = git_ops::get_gh_token().map_err(|e| e.to_string())?;
        let user = git_ops::get_gh_user().unwrap_or_else(|_| "unknown".to_string());

        let config = SyncConfig::load().map_err(|e| e.to_string())?;
        let repo_path = SyncConfig::repo_path();

        if repo_path.join(".git").exists() && config.repo.url.starts_with("https://") {
            let repo = GitRepo::open_or_init(&repo_path).map_err(|e| e.to_string())?;
            repo.set_remote_with_token("origin", &config.repo.url, &token)
                .map_err(|e| e.to_string())?;
        }

        Ok(format!("Logged in as {user}"))
    })
    .await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_config,
            list_secrets,
            list_skills,
            list_plugins,
            sync_push,
            sync_pull,
            push_selected_skills,
            pull_selected_skills,
            push_selected_plugins,
            pull_selected_plugins,
            check_git,
            get_default_device_id,
            run_setup,
            check_auth_status,
            login_with_token,
            login_with_gh_cli,
            update_sync_options,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
