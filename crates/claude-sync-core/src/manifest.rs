use serde::{Deserialize, Serialize};

use crate::config::Platform;
use crate::discovery::FileCategory;
use crate::error::Result;

/// 싱크 매니페스트 (sync repo에 저장)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncManifest {
    pub schema_version: u32,
    pub last_sync: String,
    pub device_id: String,
    pub platform: Platform,
    pub files: Vec<ManifestEntry>,
}

/// 개별 파일 엔트리
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub path: String,
    pub sha256: String,
    pub category: FileCategory,
    /// 마스킹된 시크릿 필드 경로들
    #[serde(default)]
    pub masked_secrets: Vec<String>,
    /// 플랫폼별 필드 경로들
    #[serde(default)]
    pub platform_specific: Vec<String>,
}

impl SyncManifest {
    pub fn new(device_id: &str, platform: Platform) -> Self {
        Self {
            schema_version: 1,
            last_sync: chrono::Utc::now().to_rfc3339(),
            device_id: device_id.to_string(),
            platform,
            files: Vec::new(),
        }
    }

    /// JSON 파일에서 로드
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let manifest: SyncManifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    /// JSON 파일에 저장
    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 파일 엔트리 추가/업데이트
    pub fn upsert_entry(&mut self, entry: ManifestEntry) {
        if let Some(existing) = self.files.iter_mut().find(|e| e.path == entry.path) {
            *existing = entry;
        } else {
            self.files.push(entry);
        }
    }

    /// 특정 파일의 마스킹된 시크릿 경로 조회
    pub fn masked_secrets_for(&self, path: &str) -> Vec<String> {
        self.files
            .iter()
            .find(|e| e.path == path)
            .map(|e| e.masked_secrets.clone())
            .unwrap_or_default()
    }
}

/// 파일 SHA256 해시 계산
pub fn compute_sha256(content: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}
