use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SecretAction {
    Mask,
    Remove,
    Keep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretPattern {
    pub name: String,
    /// 점으로 구분한 키 경로 패턴 (e.g., "env.*_API_KEY"). 경로의 끝부분과 비교하며,
    /// 세그먼트마다 `*`, `*접미사`, `접두사*`를 지원한다.
    pub json_path: String,
    pub action: SecretAction,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceConfig {
    #[serde(alias = "id")]
    pub name: String,
    pub platform: Platform,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            name: default_device_name(),
            platform: Platform::current(),
        }
    }
}

pub fn default_device_name() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "my-device".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BackupConfig {
    /// 이 개수를 넘으면 오래된 자동 백업부터 지운다. 수동 백업은 지우지 않는다.
    pub max_auto_backups: usize,
    pub backup_before_import: bool,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            max_auto_backups: 20,
            backup_before_import: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct P2pConfig {
    pub port: u16,
    pub discovery: bool,
}

impl Default for P2pConfig {
    fn default() -> Self {
        Self {
            port: 47821,
            discovery: true,
        }
    }
}

/// `~/.claude-sync/config.toml`. 파일이 없어도 기본값으로 동작한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncConfig {
    pub schema_version: u32,
    pub device: DeviceConfig,
    pub default_template: String,
    pub backup: BackupConfig,
    pub p2p: P2pConfig,
    pub secret_patterns: Vec<SecretPattern>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            schema_version: 2,
            device: DeviceConfig::default(),
            default_template: "full".to_string(),
            backup: BackupConfig::default(),
            p2p: P2pConfig::default(),
            secret_patterns: Self::default_secret_patterns(),
        }
    }
}

impl SyncConfig {
    pub fn load() -> Result<Self> {
        let path = paths::config_file();
        if !path.exists() {
            return Ok(Self::default());
        }
        Self::parse(&std::fs::read_to_string(&path)?)
    }

    fn parse(content: &str) -> Result<Self> {
        let mut config: SyncConfig = toml::from_str(content)?;
        if config.schema_version < 2 {
            config.migrate_v1_secret_patterns();
        }
        if config.secret_patterns.is_empty() {
            config.secret_patterns = Self::default_secret_patterns();
        }
        config.schema_version = 2;
        Ok(config)
    }

    /// v1 패턴은 `mcpServers.*.env.*_API_KEY`처럼 파일 최상위부터 쓴 경로였다.
    /// v2는 경로 끝부분과 비교하므로 앞부분을 떼고, 새 기본 패턴 중 빠진 것을 더한다.
    fn migrate_v1_secret_patterns(&mut self) {
        for pattern in &mut self.secret_patterns {
            if let Some(rest) = pattern.json_path.strip_prefix("mcpServers.*.") {
                pattern.json_path = rest.to_string();
            }
        }
        for default in Self::default_secret_patterns() {
            if !self.secret_patterns.iter().any(|p| p.json_path == default.json_path) {
                self.secret_patterns.push(default);
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        crate::fsutil::atomic_write(&paths::config_file(), content.as_bytes())?;
        Ok(())
    }

    pub fn default_secret_patterns() -> Vec<SecretPattern> {
        let mask = |name: &str, json_path: &str| SecretPattern {
            name: name.to_string(),
            json_path: json_path.to_string(),
            action: SecretAction::Mask,
        };
        vec![
            mask("env API keys", "env.*_API_KEY"),
            mask("env API keys (short)", "env.*_KEY"),
            mask("env tokens", "env.*_TOKEN"),
            mask("env secrets", "env.*_SECRET"),
            mask("env passwords", "env.*PASSWORD"),
            mask("header API keys", "headers.*_API_KEY"),
            mask("header authorization", "headers.Authorization"),
            mask("header tokens", "headers.*_TOKEN"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_설정의_시크릿_패턴은_경로_끝부분_형식으로_바뀌고_새_기본값이_더해진다() {
        let v1 = r#"
schema_version = 1
[repo]
url = "git@github.com:me/cfg.git"
[device]
id = "old-pc"
platform = "windows"
[[secret_patterns]]
name = "MCP env API keys"
json_path = "mcpServers.*.env.*_API_KEY"
action = "mask"
"#;

        let config = SyncConfig::parse(v1).unwrap();
        let paths: Vec<&str> = config.secret_patterns.iter().map(|p| p.json_path.as_str()).collect();

        assert_eq!(config.device.name, "old-pc");
        assert_eq!(paths[0], "env.*_API_KEY");
        assert!(paths.contains(&"headers.Authorization"));
        assert_eq!(paths.iter().filter(|p| **p == "env.*_API_KEY").count(), 1);
    }
}
