use anyhow::Result;
use console::style;

use claude_sync_core::config::SyncConfig;
use claude_sync_core::git_ops::GitRepo;

pub async fn run(file: Option<String>) -> Result<()> {
    let config = SyncConfig::load()?;
    let repo_path = SyncConfig::repo_path();

    println!("{}", style("Claude Sync Diff").bold().cyan());
    println!("{}", style("─".repeat(30)).dim());

    let repo = GitRepo::open_or_init(&repo_path)?;

    // fetch to get latest
    let _ = repo.fetch("origin");

    let diff_files = repo.diff_names("origin", &config.repo.branch)?;

    if diff_files.is_empty() {
        println!("  {}", style("변경 사항 없음").green());
        return Ok(());
    }

    match file {
        Some(ref target) => {
            if diff_files.contains(target) {
                println!("  Changed: {}", style(target).yellow());
            } else {
                println!("  {} 해당 파일에 변경 없음: {}", style("✓").green(), target);
            }
        }
        None => {
            println!("  Changed files ({}):", diff_files.len());
            for f in &diff_files {
                println!("    {} {}", style("M").yellow(), f);
            }
        }
    }

    Ok(())
}
