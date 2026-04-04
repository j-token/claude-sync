use anyhow::Result;
use console::style;
use dialoguer::Select;

use claude_sync_core::snapshot;

pub async fn run(latest: bool, list: bool) -> Result<()> {
    println!("{}", style("Claude Sync Restore").bold().cyan());
    println!("{}", style("─".repeat(30)).dim());

    let snapshots = snapshot::list_snapshots()?;

    if snapshots.is_empty() {
        println!("  스냅샷 없음");
        return Ok(());
    }

    if list {
        println!("  Available snapshots:");
        for (i, snap) in snapshots.iter().enumerate() {
            println!(
                "  {}. {} ({} files)",
                i + 1,
                style(&snap.id).bold(),
                snap.file_count
            );
        }
        return Ok(());
    }

    let snapshot_id = if latest {
        snapshots[0].id.clone()
    } else {
        // 인터랙티브 선택
        let items: Vec<String> = snapshots
            .iter()
            .map(|s| format!("{} ({} files)", s.id, s.file_count))
            .collect();

        let choice = Select::new()
            .with_prompt("복원할 스냅샷 선택")
            .items(&items)
            .default(0)
            .interact()?;

        snapshots[choice].id.clone()
    };

    let restored = snapshot::restore_snapshot(&snapshot_id)?;
    println!(
        "  {} {} 에서 {} files 복원됨",
        style("✓").green(),
        snapshot_id,
        restored
    );

    Ok(())
}
