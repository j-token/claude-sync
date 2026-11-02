//! CLI와 GUI가 함께 쓰는 상위 작업. 각 작업은 설정·인벤토리·백업 정책을 한곳에서 적용한다.

use std::path::{Path, PathBuf};

use crate::backup::{BackupInfo, BackupKind, BackupStore};
use crate::bundle::{self, Bundle, ExportOptions, ExportSummary, ImportMode, ImportReport, PreviewItem};
use crate::config::SyncConfig;
use crate::error::{Result, SyncError};
use crate::inventory::{self, ClaudeEnv, Inventory, ItemTarget};
use crate::local_json::LocalJson;
use crate::secret::{SecretEngine, SecretMatch};
use crate::template;

pub struct App {
    pub config: SyncConfig,
    pub env: ClaudeEnv,
    pub backups: BackupStore,
}

pub struct DetectedSecret {
    pub item_id: String,
    pub secret: SecretMatch,
}

enum AutoBackup {
    Created(String),
    Disabled,
    NothingToOverwrite,
}

impl AutoBackup {
    fn record_in(self, report: &mut ImportReport) {
        match self {
            AutoBackup::Created(id) => report.backup_id = Some(id),
            AutoBackup::Disabled => report.backup_skipped_reason = Some("설정에서 자동 백업이 꺼져 있습니다.".into()),
            AutoBackup::NothingToOverwrite => {
                report.backup_skipped_reason = Some("덮어쓸 기존 항목이 없어 자동 백업을 만들지 않았습니다.".into())
            }
        }
    }
}

impl App {
    pub fn load() -> Result<Self> {
        Ok(Self {
            config: SyncConfig::load()?,
            env: ClaudeEnv::local(),
            backups: BackupStore::local(),
        })
    }

    pub fn inventory(&self) -> Result<Inventory> {
        if !self.env.claude_dir.is_dir() {
            return Err(SyncError::NotFound(format!(
                "Claude 설정 디렉토리 {}",
                self.env.claude_dir.display()
            )));
        }
        inventory::scan(&self.env)
    }

    pub fn secrets(&self) -> SecretEngine {
        SecretEngine::new(&self.config.secret_patterns)
    }

    /// 직접 고른 id와 템플릿이 고른 id의 합집합. 순서는 인벤토리 순서를 따른다.
    pub fn resolve_selection(&self, inventory: &Inventory, ids: &[String], template_name: Option<&str>) -> Result<Vec<String>> {
        let template = template_name.map(template::find).transpose()?;
        for id in ids {
            if inventory.get(id).is_none() {
                return Err(SyncError::NotFound(format!("항목 {id}")));
            }
        }
        Ok(inventory
            .items
            .iter()
            .filter(|item| ids.contains(&item.id) || template.as_ref().is_some_and(|t| t.matches(&item.id)))
            .map(|item| item.id.clone())
            .collect())
    }

    pub fn export(&self, ids: &[String], template_name: Option<&str>, label: Option<String>, out: &Path) -> Result<ExportSummary> {
        let inventory = self.inventory()?;
        let selected = self.resolve_selection(&inventory, ids, template_name)?;
        self.export_resolved(&inventory, &selected, template_name.map(str::to_string), label, out)
    }

    /// 이미 확정된 id 목록을 내보낸다. `template_label`은 매니페스트에 기록만 한다.
    pub fn export_selected(&self, ids: &[String], template_label: Option<String>, label: Option<String>, out: &Path) -> Result<ExportSummary> {
        let inventory = self.inventory()?;
        let selected = self.resolve_selection(&inventory, ids, None)?;
        self.export_resolved(&inventory, &selected, template_label, label, out)
    }

    fn export_resolved(
        &self,
        inventory: &Inventory,
        selected: &[String],
        template_label: Option<String>,
        label: Option<String>,
        out: &Path,
    ) -> Result<ExportSummary> {
        bundle::export(
            &self.env,
            inventory,
            selected,
            &self.secrets(),
            &ExportOptions {
                label,
                template: template_label,
                device_name: self.config.device.name.clone(),
            },
            out,
        )
    }

    /// `ids`와 `template_name`이 모두 비면 기본 템플릿으로 백업한다.
    pub fn create_backup(&self, ids: &[String], template_name: Option<&str>, label: Option<String>) -> Result<BackupInfo> {
        let inventory = self.inventory()?;
        let template_name = match (ids.is_empty(), template_name) {
            (true, None) => Some(self.config.default_template.as_str()),
            (_, name) => name,
        };
        let selected = self.resolve_selection(&inventory, ids, template_name)?;
        self.backups.create(
            &self.env,
            &inventory,
            &selected,
            &self.secrets(),
            BackupKind::Manual,
            label,
            &self.config.device.name,
        )
    }

    pub fn preview(&self, path: &Path) -> Result<(Bundle, Vec<PreviewItem>)> {
        let bundle = Bundle::open(path)?;
        let items = bundle::preview(&bundle, &self.env)?;
        Ok((bundle, items))
    }

    pub fn import(&self, path: &Path, ids: &[String], mode: ImportMode) -> Result<ImportReport> {
        let mut bundle = Bundle::open(path)?;
        let selected = if ids.is_empty() { bundle.ids() } else { ids.to_vec() };
        let auto_backup = self.backup_items_about_to_change(&bundle, &selected, "가져오기 전 자동 백업")?;
        let mut report = bundle::import(&mut bundle, &self.env, &selected, mode)?;
        auto_backup.record_in(&mut report);
        self.backups.prune_auto(self.config.backup.max_auto_backups, None)?;
        Ok(report)
    }

    pub fn restore(&self, backup_id: &str, ids: &[String]) -> Result<ImportReport> {
        let path = self.backups.path_of(backup_id)?;
        let bundle = Bundle::open(&path)?;
        let selected = if ids.is_empty() { bundle.ids() } else { ids.to_vec() };
        let safety_backup = self.backup_items_about_to_change(&bundle, &selected, &format!("{backup_id} 복원 전 자동 백업"))?;
        drop(bundle);
        let mut report = self.backups.restore(&self.env, backup_id, &selected)?;
        safety_backup.record_in(&mut report);
        // 복원 원본이 오래된 자동 백업이어도 사용자가 방금 쓴 백업이므로 지우지 않는다.
        self.backups.prune_auto(self.config.backup.max_auto_backups, Some(backup_id))?;
        Ok(report)
    }

    /// 덮어쓰일 수 있는 로컬 항목(번들에도 있고 로컬에도 있는 항목)만 자동 백업한다.
    fn backup_items_about_to_change(&self, bundle: &Bundle, selected: &[String], label: &str) -> Result<AutoBackup> {
        if !self.config.backup.backup_before_import {
            return Ok(AutoBackup::Disabled);
        }
        let inventory = self.inventory()?;
        let existing: Vec<String> = selected
            .iter()
            .filter(|id| bundle.item(id).is_some() && inventory.get(id).is_some())
            .cloned()
            .collect();
        if existing.is_empty() {
            return Ok(AutoBackup::NothingToOverwrite);
        }
        let info = self.backups.create(
            &self.env,
            &inventory,
            &existing,
            &self.secrets(),
            BackupKind::Auto,
            Some(label.to_string()),
            &self.config.device.name,
        )?;
        Ok(AutoBackup::Created(info.id))
    }

    /// P2P로 보낼 임시 번들을 만든다. 전송이 끝나면 호출한 쪽이 지운다.
    pub fn build_share_bundle(&self, ids: &[String], template_name: Option<&str>) -> Result<(PathBuf, usize)> {
        let outgoing = crate::paths::sync_home().join("outgoing");
        let path = outgoing.join(format!("share-{}.zip", chrono::Local::now().format("%Y%m%d-%H%M%S")));
        let summary = self.export(ids, template_name, Some("P2P 공유".into()), &path)?;
        Ok((path, summary.item_count))
    }

    pub fn detect_secrets(&self) -> Result<Vec<DetectedSecret>> {
        let inventory = self.inventory()?;
        let local = LocalJson::load(&self.env)?;
        let engine = self.secrets();
        let mut found = Vec::new();
        for item in &inventory.items {
            if matches!(item.target, ItemTarget::Files { .. }) {
                continue;
            }
            let Some(payload) = local.payload(&item.target) else {
                continue;
            };
            for secret in engine.detect(&payload) {
                found.push(DetectedSecret {
                    item_id: item.id.clone(),
                    secret,
                });
            }
        }
        Ok(found)
    }

    pub fn secret_counts(&self, inventory: &Inventory) -> Result<Vec<(String, usize)>> {
        let local = LocalJson::load(&self.env)?;
        let engine = self.secrets();
        Ok(inventory
            .items
            .iter()
            .map(|item| {
                let count = local.payload(&item.target).map(|p| engine.detect(&p).len()).unwrap_or(0);
                (item.id.clone(), count)
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::test_support::TempEnv;

    #[test]
    fn 자동_백업_한도가_1이어도_그_자동_백업으로_복원하면_백업이_남는다() {
        let t = TempEnv::new("app-restore");
        t.write("CLAUDE.md", "v1");
        let mut config = SyncConfig::default();
        config.backup.max_auto_backups = 1;
        let app = App {
            config,
            env: t.env.clone(),
            backups: BackupStore::new(t.dir.join("backups")),
        };
        let inventory = app.inventory().unwrap();
        let auto = app
            .backups
            .create(&app.env, &inventory, &inventory.ids(), &app.secrets(), BackupKind::Auto, None, "dev")
            .unwrap();
        t.write("CLAUDE.md", "v2");

        let report = app.restore(&auto.id, &[]).unwrap();

        assert_eq!(report.imported, vec!["claude_md"]);
        assert_eq!(t.read("CLAUDE.md"), "v1");
        assert!(app.backups.path_of(&auto.id).is_ok());
        assert!(report.backup_id.is_some());
    }

    #[test]
    fn 새_항목만_가져오면_자동_백업_없이_그_이유를_알린다() {
        let source = TempEnv::new("app-source");
        source.write("CLAUDE.md", "from source");
        let source_app = App {
            config: SyncConfig::default(),
            env: source.env.clone(),
            backups: BackupStore::new(source.dir.join("backups")),
        };
        let bundle_path = source.dir.join("b.zip");
        source_app.export(&["claude_md".to_string()], None, None, &bundle_path).unwrap();
        let target = TempEnv::new("app-target");
        let app = App {
            config: SyncConfig::default(),
            env: target.env.clone(),
            backups: BackupStore::new(target.dir.join("backups")),
        };

        let report = app.import(&bundle_path, &[], ImportMode::Merge).unwrap();

        assert_eq!(report.imported, vec!["claude_md"]);
        assert_eq!(report.backup_id, None);
        assert_eq!(report.backup_skipped_reason.as_deref(), Some("덮어쓸 기존 항목이 없어 자동 백업을 만들지 않았습니다."));
    }
}
