use anyhow::{Result, bail};
use console::style;
use dialoguer::{Confirm, theme::ColorfulTheme};

use claude_sync_core::app::App;
use claude_sync_core::backup::BackupKind;
use claude_sync_core::fsutil::format_bytes;

use super::{BackupAction, import, ui};

pub fn run(action: BackupAction) -> Result<()> {
    let app = App::load()?;
    match action {
        BackupAction::Create { items, template, label } => {
            let info = app.create_backup(&items, template.as_deref(), label)?;
            ui::ok(format!(
                "백업 {} 생성 (항목 {}개, {})",
                info.id,
                info.item_count,
                format_bytes(info.size_bytes)
            ));
        }
        BackupAction::List => {
            ui::title("백업 목록");
            let backups = app.backups.list()?;
            if backups.is_empty() {
                println!("  백업이 없습니다");
            }
            for b in backups {
                let kind = match b.kind {
                    BackupKind::Auto => style("자동").dim().to_string(),
                    BackupKind::Manual => style("수동").green().to_string(),
                };
                println!(
                    "  {}  {}  항목 {:>3}개  {:>9}  {}",
                    b.id,
                    kind,
                    b.item_count,
                    format_bytes(b.size_bytes),
                    style(b.label.unwrap_or_default()).dim()
                );
            }
        }
        BackupAction::Show { id } => {
            let path = app.backups.path_of(&id)?;
            import::inspect(&path)?;
        }
        BackupAction::Restore { id, items, yes } => {
            let path = app.backups.path_of(&id)?;
            let (backup, preview) = app.preview(&path)?;
            drop(backup);
            let selected: Vec<String> = if items.is_empty() {
                preview.iter().map(|p| p.id.clone()).collect()
            } else {
                items
            };
            ui::print_preview(&preview);
            if !yes {
                if !ui::is_interactive() {
                    bail!("복원하면 현재 값이 덮어써집니다. 확인했다면 --yes를 붙이세요.");
                }
                let accepted = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt(format!("{}개 항목을 백업 {id}의 값으로 덮어쓸까요?", selected.len()))
                    .default(false)
                    .interact()?;
                if !accepted {
                    bail!("취소했습니다");
                }
            }
            let report = app.restore(&id, &selected)?;
            import::print_report(&report);
        }
        BackupAction::Delete { id } => {
            app.backups.delete(&id)?;
            ui::ok(format!("백업 {id} 삭제"));
        }
    }
    Ok(())
}
