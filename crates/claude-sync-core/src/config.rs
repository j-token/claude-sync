use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 시크릿 마스킹 동작
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SecretAction {
    Mask,
    Remove,
    Keep,
}

/// 시크릿 패턴 정의
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretPattern {
    pub name: String,
    /// JSONPath-like 패턴 (e.g., "mcpServers.*.env.*_API_KEY")
    pub json_path: String,
    pub action: SecretAction,
}

/// 플랫폼 종류
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Windows,
    Macos,
    Linux,
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::Macos
        } else {
            Platform::Linux
        }
    }
}

/// 플랫폼별 경로 처리 규칙
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformPathRule {
    pub field_path: String,
    pub platform: Platform,
    /// "skip" = 다른 플랫폼에서는 이 필드를 건드리지 않음
    pub action: String,
}

/// Git 인증 방식
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    SshAgent,
    SshKey,
    HttpsToken,
    GhCli,
}

/// Repo 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub url: String,
    #[serde(default = "default_branch")]
    pub branch: String,
}

fn default_branch() -> String {
    "main".to_string()
}

/// 인증 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub method: AuthMethod,
    /// ssh_key 방식일 때 사용할 키 경로
    pub ssh_key_path: Option<String>,
}

/// 디바이스 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub id: String,
    pub platform: Platform,
}

/// 싱크 옵션
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOptions {
    #[serde(default)]
    pub auto_sync: bool,
    #[serde(default = "default_interval")]
    pub auto_sync_interval_secs: u64,
    #[serde(default)]
    pub sync_memory: bool,
    #[serde(default = "default_true")]
    pub sync_teams: bool,
    #[serde(default = "default_true")]
    pub sync_skills: bool,
    #[serde(default = "default_true")]
    pub sync_plugins: bool,
}

fn default_interval() -> u64 {
    300
}

fn default_true() -> bool {
    true
}

/// 스냅샷 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotConfig {
    #[serde(default = "default_max_snapshots")]
    pub max_count: usize,
}

fn default_max_snapshots() -> usize {
    10
}

/// 메인 설정 구조체 (~/.claude-sync/config.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub repo: RepoConfig,
    pub auth: AuthConfig,
    pub device: DeviceConfig,
    pub sync: SyncOptions,
    #[serde(default)]
    pub secret_patterns: Vec<SecretPattern>,
    #[serde(default)]
    pub platform_path_rules: Vec<PlatformPathRule>,
    #[serde(default)]
    pub snapshots: SnapshotConfig,
}

fn default_schema_version() -> u32 {
    1
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self { max_count: 10 }
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        Self {
            schema_version: 1,
            repo: RepoConfig {
                url: String::new(),
                branch: "main".to_string(),
            },
            auth: AuthConfig {
                method: AuthMethod::SshAgent,
                ssh_key_path: None,
            },
            device: DeviceConfig {
                id: hostname,
                platform: Platform::current(),
            },
            sync: SyncOptions {
                auto_sync: false,
                auto_sync_interval_secs: 300,
                sync_memory: false,
                sync_teams: true,
                sync_skills: true,
                sync_plugins: true,
            },
            secret_patterns: Self::default_secret_patterns(),
            platform_path_rules: Vec::new(),
            snapshots: SnapshotConfig::default(),
        }
    }
}

impl SyncConfig {
    /// 설정 파일이 저장되는 디렉토리
    pub fn sync_dir() -> PathBuf {
        dirs::home_dir()
            .expect("home directory not found")
            .join(".claude-sync")
    }

    /// config.toml 경로
    pub fn config_path() -> PathBuf {
        Self::sync_dir().join("config.toml")
    }

    /// 싱크 레포 경로
    pub fn repo_path() -> PathBuf {
        Self::sync_dir().join("repo")
    }

    /// 스냅샷 디렉토리 경로
    pub fn snapshots_path() -> PathBuf {
        Self::sync_dir().join("snapshots")
    }

    /// Claude 설정 디렉토리 경로 (~/.claude)
    pub fn claude_dir() -> PathBuf {
        dirs::home_dir()
            .expect("home directory not found")
            .join(".claude")
    }

    /// 파일에서 설정 로드
    pub fn load() -> crate::error::Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Err(crate::error::SyncError::NotInitialized);
        }
        let content = std::fs::read_to_string(&path)?;
        let config: SyncConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// 설정을 파일에 저장
    pub fn save(&self) -> crate::error::Result<()> {
        let dir = Self::sync_dir();
        std::fs::create_dir_all(&dir)?;
        let content = toml::to_string_pretty(self)?;
        std::fs::write(Self::config_path(), content)?;
        Ok(())
    }

    /// 빌트인 시크릿 패턴
    pub fn default_secret_patterns() -> Vec<SecretPattern> {
        vec![
            SecretPattern {
                name: "MCP env API keys".to_string(),
                json_path: "mcpServers.*.env.*_API_KEY".to_string(),
                action: SecretAction::Mask,
            },
            SecretPattern {
                name: "MCP env tokens".to_string(),
                json_path: "mcpServers.*.env.*_TOKEN".to_string(),
                action: SecretAction::Mask,
            },
            SecretPattern {
                name: "MCP env secrets".to_string(),
                json_path: "mcpServers.*.env.*_SECRET".to_string(),
                action: SecretAction::Mask,
            },
            SecretPattern {
                name: "MCP header API keys".to_string(),
                json_path: "mcpServers.*.headers.*_API_KEY".to_string(),
                action: SecretAction::Mask,
            },
            SecretPattern {
                name: "MCP header tokens".to_string(),
                json_path: "mcpServers.*.headers.*_TOKEN".to_string(),
                action: SecretAction::Mask,
            },
        ]
    }
}
