use claude_sync_core::config::{
    AuthConfig, AuthMethod, DeviceConfig, Platform, RepoConfig, SnapshotConfig, SyncConfig,
    SyncOptions,
};
use claude_sync_core::discovery;
use claude_sync_core::git_ops::{self, GitRepo};
use claude_sync_core::manifest::SyncManifest;
use claude_sync_core::secret::SecretEngine;
use claude_sync_core::snapshot;
use serde::{Deserialize, Serialize};

/// GUI에서 사용하는 싱크 상태 정보
#[derive(Serialize)]
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
}

/// 시크릿 탐지 결과
#[derive(Serialize)]
struct DetectedSecret {
    json_path: String,
    pattern_name: String,
    preview: String,
}

/// 스킬 정보 (GUI용)
#[derive(Serialize)]
struct SkillEntry {
    name: String,
    path: String,
    size_bytes: u64,
    file_count: usize,
    local_exists: bool,
    remote_exists: bool,
}

#[tauri::command]
fn get_status() -> Result<SyncStatus, String> {
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
    })
}

#[tauri::command]
fn get_config() -> Result<String, String> {
    let config = SyncConfig::load().unwrap_or_default();
    serde_json::to_string(&config).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_secrets() -> Result<Vec<DetectedSecret>, String> {
    let config = SyncConfig::load().map_err(|e| e.to_string())?;
    let claude_dir = SyncConfig::claude_dir();
    let settings_path = claude_dir.join("settings.json");

    if !settings_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&settings_path).map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

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
}

#[tauri::command]
fn list_skills() -> Result<Vec<SkillEntry>, String> {
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
                            path: format!("skills/{}", entry.file_name().to_string_lossy()),
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
}

#[tauri::command]
fn sync_push() -> Result<String, String> {
    // Push 로직 (CLI와 동일한 core 호출)
    let config = SyncConfig::load().map_err(|e| e.to_string())?;
    let discovery_result = discovery::discover(&config).map_err(|e| e.to_string())?;
    let secret_engine = SecretEngine::new(&config.secret_patterns);
    let repo_path = SyncConfig::repo_path();
    let claude_dir = SyncConfig::claude_dir();

    // 스냅샷
    snapshot::create_snapshot().map_err(|e| e.to_string())?;

    // 파일 복사 + 마스킹
    for file in &discovery_result.syncable {
        let src = claude_dir.join(&file.relative_path);
        let dst = repo_path.join(&file.relative_path);
        if !src.exists() {
            continue;
        }
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        match file.category {
            discovery::FileCategory::Settings | discovery::FileCategory::McpJson => {
                let content = std::fs::read(&src).map_err(|e| e.to_string())?;
                let json: serde_json::Value =
                    serde_json::from_slice(&content).map_err(|e| e.to_string())?;
                let (masked, _) = secret_engine.mask(&json);
                let output = serde_json::to_string_pretty(&masked).map_err(|e| e.to_string())?;
                std::fs::write(&dst, output).map_err(|e| e.to_string())?;
            }
            _ => {
                std::fs::copy(&src, &dst).map_err(|e| e.to_string())?;
            }
        }
    }

    // Git commit & push
    let repo = GitRepo::open_or_init(&repo_path).map_err(|e| e.to_string())?;
    repo.add_all().map_err(|e| e.to_string())?;

    let message = format!(
        "sync: {} @ {}",
        config.device.id,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );
    let _ = repo.commit(&message);
    repo.push("origin", &config.repo.branch)
        .map_err(|e| e.to_string())?;

    Ok("Push 완료".to_string())
}

#[tauri::command]
fn sync_pull() -> Result<String, String> {
    let config = SyncConfig::load().map_err(|e| e.to_string())?;
    let repo_path = SyncConfig::repo_path();
    let claude_dir = SyncConfig::claude_dir();

    snapshot::create_snapshot().map_err(|e| e.to_string())?;

    let repo = GitRepo::open_or_init(&repo_path).map_err(|e| e.to_string())?;
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

    Ok(format!("Pull 완료: {} files", count))
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
}

#[tauri::command]
fn check_git() -> Result<String, String> {
    git_ops::check_git_available().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_default_device_id() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "my-device".to_string())
}

#[tauri::command]
fn run_setup(input: SetupInput) -> Result<String, String> {
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
        },
        secret_patterns: SyncConfig::default_secret_patterns(),
        platform_path_rules: Vec::new(),
        snapshots: SnapshotConfig::default(),
    };

    config.save().map_err(|e| e.to_string())?;

    // 싱크 레포 초기화
    let repo_path = SyncConfig::repo_path();
    if !repo_path.join(".git").exists() {
        match GitRepo::clone_repo(&input.repo_url, &repo_path) {
            Ok(_) => {}
            Err(_) => {
                let repo = GitRepo::open_or_init(&repo_path).map_err(|e| e.to_string())?;
                repo.set_remote("origin", &input.repo_url)
                    .map_err(|e| e.to_string())?;
                let _ = repo.set_branch("main");
            }
        }
    }

    Ok("Setup 완료".to_string())
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
            sync_push,
            sync_pull,
            check_git,
            get_default_device_id,
            run_setup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
