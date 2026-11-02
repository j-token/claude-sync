use std::path::PathBuf;

use anyhow::{Result, bail};

use claude_sync_core::app::App;
use claude_sync_core::fsutil::format_bytes;

use super::ui;

pub fn run(items: &[String], template: Option<&str>, output: Option<PathBuf>, label: Option<String>) -> Result<()> {
    let app = App::load()?;
    let ids = if items.is_empty() && template.is_none() {
        let inventory = app.inventory()?;
        ui::pick_local_items(&inventory, "내보낼 항목")?
    } else {
        items.to_vec()
    };
    if ids.is_empty() && template.is_none() {
        bail!("선택한 항목이 없습니다");
    }

    let output = output.unwrap_or_else(|| default_output_name(&app.config.device.name));
    let summary = app.export(&ids, template, label, &output)?;

    ui::title("내보내기 완료");
    ui::ok(format!("{}", summary.path.display()));
    ui::ok(format!(
        "항목 {}개 · 파일 {}개 · {}",
        summary.item_count,
        summary.file_count,
        format_bytes(summary.size_bytes)
    ));
    if summary.masked_secret_count > 0 {
        ui::warn(format!(
            "시크릿 {}개는 비워서 담았습니다 (가져오는 기기의 기존 값으로 채워집니다)",
            summary.masked_secret_count
        ));
    }
    Ok(())
}

pub fn default_output_name(device_name: &str) -> PathBuf {
    let device: String = device_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .collect();
    PathBuf::from(format!(
        "claude-sync-{device}-{}.zip",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    ))
}
