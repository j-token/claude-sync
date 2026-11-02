use anyhow::Result;
use console::style;

use claude_sync_core::app::App;
use claude_sync_core::fsutil::format_bytes;
use claude_sync_core::inventory::ItemKind;

use super::ui;

pub fn run(kind: Option<&str>) -> Result<()> {
    let app = App::load()?;
    let inventory = app.inventory()?;
    let filter = kind.map(ui::kind_from_arg).transpose()?;

    ui::title(&format!("Claude Code 항목 — {}", app.env.claude_dir.display()));
    for kind in ItemKind::ALL {
        if filter.is_some_and(|f| f != kind) {
            continue;
        }
        let items: Vec<_> = inventory.items.iter().filter(|i| i.kind == kind).collect();
        if items.is_empty() {
            continue;
        }
        println!("\n{} ({})", style(kind.label()).bold(), items.len());
        for item in items {
            let size = if item.file_count() > 0 {
                format!("{} · 파일 {}개", format_bytes(item.size_bytes), item.file_count())
            } else {
                format_bytes(item.size_bytes)
            };
            println!(
                "  {}{}  {}",
                item.id,
                ui::executable_badge(item.is_executable()),
                style(size).dim()
            );
            if let Some(description) = &item.description {
                println!("      {}", style(description).dim());
            }
        }
    }
    println!("\n총 {}개", inventory.items.len());
    Ok(())
}
