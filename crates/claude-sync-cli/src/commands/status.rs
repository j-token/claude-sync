use anyhow::Result;
use console::style;

use claude_sync_core::config::SyncConfig;
use claude_sync_core::discovery;
use claude_sync_core::git_ops::GitRepo;
use claude_sync_core::manifest::SyncManifest;

pub async fn run() -> Result<()> {
    let config = SyncConfig::load()?;

    println!("{}", style("Claude Sync Status").bold().cyan());
    println!("{}", style("─".repeat(30)).dim());

    // 1. 설정 정보
    println!("  Device:   {}", style(&config.device.id).yellow());
    println!("  Platform: {:?}", config.device.platform);
    println!("  Repo:     {}", config.repo.url);
    println!("  Branch:   {}", config.repo.branch);
    println!();

    // 2. 싱크 옵션
    println!("{}", style("Sync Options").bold());
    println!(
        "  Memory:  {}",
        if config.sync.sync_memory {
            style("ON").green()
        } else {
            style("OFF").red()
        }
    );
    println!(
        "  Teams:   {}",
        if config.sync.sync_teams {
            style("ON").green()
        } else {
            style("OFF").red()
        }
    );
    println!(
        "  Skills:  {}",
        if config.sync.sync_skills {
            style("ON").green()
        } else {
            style("OFF").red()
        }
    );
    println!(
        "  Plugins: {}",
        if config.sync.sync_plugins {
            style("ON").green()
        } else {
            style("OFF").red()
        }
    );
    println!();

    // 3. 파일 현황
    let discovery_result = discovery::discover(&config)?;
    println!("{}", style("Discovered Files").bold());
    println!("  Syncable files: {}", discovery_result.syncable.len());
    println!("  Skills:         {}", discovery_result.skills.len());
    println!("  Plugins:        {}", discovery_result.plugins.len());
    println!("  Skipped:        {}", discovery_result.skipped.len());
    println!();

    // 4. Git 상태
    let repo_path = SyncConfig::repo_path();
    if repo_path.join(".git").exists() {
        let repo = GitRepo::open_or_init(&repo_path)?;

        println!("{}", style("Git Status").bold());

        // 로컬 변경사항
        let status = repo.status()?;
        let dirty_count = status.stdout.lines().filter(|l| !l.is_empty()).count();
        if dirty_count > 0 {
            println!(
                "  Local changes:  {} (not committed)",
                style(format!("{} files", dirty_count)).yellow()
            );
        } else {
            println!("  Local changes:  {}", style("clean").green());
        }

        // 매니페스트 정보
        let manifest_path = repo_path.join("manifest.json");
        if manifest_path.exists() {
            if let Ok(manifest) = SyncManifest::load(&manifest_path) {
                println!("  Last sync:      {}", manifest.last_sync);
                println!("  Last device:    {}", manifest.device_id);
                println!("  Manifest files: {}", manifest.files.len());
            }
        }

        // Ahead/behind
        let _ = repo.fetch("origin"); // fetch to get latest
        match repo.ahead_behind("origin", &config.repo.branch) {
            Ok((ahead, behind)) => {
                if ahead == 0 && behind == 0 {
                    println!("  Sync status:    {}", style("up to date").green());
                } else {
                    if ahead > 0 {
                        println!(
                            "  Ahead:          {}",
                            style(format!("{} commits", ahead)).yellow()
                        );
                    }
                    if behind > 0 {
                        println!(
                            "  Behind:         {}",
                            style(format!("{} commits", behind)).yellow()
                        );
                    }
                }
            }
            Err(_) => {
                println!(
                    "  Sync status:    {}",
                    style("원격 브랜치 없음 (첫 push 필요)").dim()
                );
            }
        }
    } else {
        println!(
            "  {}",
            style("싱크 레포가 초기화되지 않았습니다. 'claude-sync init'을 실행하세요.").red()
        );
    }

    Ok(())
}
