//! 백업은 `~/.claude-sync/backups/`에 저장하는 일반 번들이다.
//! 파일 이름: `<YYYYMMDD-HHMMSS>-<auto|manual>.zip`

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::bundle::{self, Bundle, ExportOptions, ImportMode, ImportReport};
use crate::error::{Result, SyncError};
use crate::inventory::{ClaudeEnv, Inventory};
use crate::secret::SecretEngine;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupKind {
    Auto,
    Manual,
}

impl BackupKind {
    fn key(self) -> &'static str {
        match self {
            BackupKind::Auto => "auto",
            BackupKind::Manual => "manual",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupInfo {
    pub id: String,
    pub path: PathBuf,
    pub created_at: String,
    pub kind: BackupKind,
    pub label: Option<String>,
    pub item_count: usize,
    pub size_bytes: u64,
}

pub struct BackupStore {
    dir: PathBuf,
}

impl BackupStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn local() -> Self {
        Self::new(crate::paths::backups_dir())
    }

    pub fn create(
        &self,
        env: &ClaudeEnv,
        inventory: &Inventory,
        ids: &[String],
        secrets: &SecretEngine,
        kind: BackupKind,
        label: Option<String>,
        device_name: &str,
    ) -> Result<BackupInfo> {
        std::fs::create_dir_all(&self.dir)?;
        let id = self.unique_id(kind);
        let path = self.dir.join(format!("{id}.zip"));
        bundle::export(
            env,
            inventory,
            ids,
            secrets,
            &ExportOptions {
                label,
                template: None,
                device_name: device_name.to_string(),
            },
            &path,
        )?;
        read_info(&path)
    }

    fn unique_id(&self, kind: BackupKind) -> String {
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let base = format!("{stamp}-{}", kind.key());
        let mut id = base.clone();
        let mut suffix = 2;
        while self.dir.join(format!("{id}.zip")).exists() {
            id = format!("{base}-{suffix}");
            suffix += 1;
        }
        id
    }

    /// 최신순 목록. 읽을 수 없는 파일은 건너뛴다.
    pub fn list(&self) -> Result<Vec<BackupInfo>> {
        if !self.dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut backups = Vec::new();
        for entry in std::fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "zip") {
                match read_info(&path) {
                    Ok(info) => backups.push(info),
                    Err(e) => tracing::warn!("백업 {} 무시: {e}", path.display()),
                }
            }
        }
        backups.sort_by(|a, b| b.id.cmp(&a.id));
        Ok(backups)
    }

    pub fn path_of(&self, id: &str) -> Result<PathBuf> {
        let valid_id = !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        let path = self.dir.join(format!("{id}.zip"));
        if !valid_id || !path.exists() {
            return Err(SyncError::NotFound(format!("백업 '{id}'")));
        }
        Ok(path)
    }

    /// `ids`가 비어 있으면 백업의 모든 항목을 덮어쓰기로 복원한다.
    pub fn restore(&self, env: &ClaudeEnv, id: &str, ids: &[String]) -> Result<ImportReport> {
        let mut backup = Bundle::open(&self.path_of(id)?)?;
        let selected = if ids.is_empty() { backup.ids() } else { ids.to_vec() };
        bundle::import(&mut backup, env, &selected, ImportMode::Overwrite)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        std::fs::remove_file(self.path_of(id)?)?;
        Ok(())
    }

    /// 자동 백업이 `max_auto`개를 넘으면 오래된 것부터 지운다. `keep_id`는 개수에 세지 않고 남긴다.
    pub fn prune_auto(&self, max_auto: usize, keep_id: Option<&str>) -> Result<usize> {
        let autos: Vec<BackupInfo> = self
            .list()?
            .into_iter()
            .filter(|b| b.kind == BackupKind::Auto && Some(b.id.as_str()) != keep_id)
            .collect();
        let mut removed = 0;
        for old in autos.iter().skip(max_auto) {
            std::fs::remove_file(&old.path)?;
            removed += 1;
        }
        Ok(removed)
    }
}

fn read_info(path: &Path) -> Result<BackupInfo> {
    let bundle = Bundle::open(path)?;
    let id = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let kind = if id.contains("-auto") { BackupKind::Auto } else { BackupKind::Manual };
    Ok(BackupInfo {
        created_at: bundle.manifest.created_at.clone(),
        label: bundle.manifest.label.clone(),
        item_count: bundle.manifest.items.len(),
        size_bytes: std::fs::metadata(path)?.len(),
        path: path.to_path_buf(),
        kind,
        id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SyncConfig;
    use crate::inventory::{self, test_support::TempEnv};

    #[test]
    fn 백업_후_내용이_바뀌어도_복원하면_원래대로_돌아온다() {
        let t = TempEnv::new("backup");
        t.write("CLAUDE.md", "original");
        t.write("agents/a.md", "agent v1");
        let store = BackupStore::new(t.dir.join("backups"));
        let inventory = inventory::scan(&t.env).unwrap();
        let secrets = SecretEngine::new(&SyncConfig::default_secret_patterns());

        let info = store
            .create(&t.env, &inventory, &inventory.ids(), &secrets, BackupKind::Manual, Some("before".into()), "dev")
            .unwrap();
        t.write("CLAUDE.md", "changed");
        t.write("agents/a.md", "agent v2");
        let report = store.restore(&t.env, &info.id, &["claude_md".to_string()]).unwrap();

        assert_eq!(report.imported, vec!["claude_md"]);
        assert_eq!(t.read("CLAUDE.md"), "original");
        assert_eq!(t.read("agents/a.md"), "agent v2");
        assert_eq!(store.list().unwrap()[0].label.as_deref(), Some("before"));
    }

    #[test]
    fn 자동_백업은_최대_개수만_남긴다() {
        let t = TempEnv::new("prune");
        t.write("CLAUDE.md", "x");
        let store = BackupStore::new(t.dir.join("backups"));
        let inventory = inventory::scan(&t.env).unwrap();
        let secrets = SecretEngine::new(&SyncConfig::default_secret_patterns());
        for _ in 0..3 {
            store
                .create(&t.env, &inventory, &inventory.ids(), &secrets, BackupKind::Auto, None, "dev")
                .unwrap();
        }

        let removed = store.prune_auto(1, None).unwrap();

        assert_eq!(removed, 2);
        assert_eq!(store.list().unwrap().len(), 1);
    }

    #[test]
    fn 경로_조작이_들어간_백업_id는_찾지_않는다() {
        let store = BackupStore::new(std::env::temp_dir());

        assert!(store.path_of("../x").is_err());
    }
}
