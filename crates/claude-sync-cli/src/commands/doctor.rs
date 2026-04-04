use anyhow::Result;
use console::style;

use claude_sync_core::config::SyncConfig;
use claude_sync_core::git_ops;

pub async fn run() -> Result<()> {
    println!("{}", style("Claude Sync Doctor").bold().cyan());
    println!("{}", style("─".repeat(30)).dim());

    let mut issues = 0;

    // 1. Git 확인
    match git_ops::check_git_available() {
        Ok(version) => println!("  {} Git: {}", style("✓").green(), version),
        Err(e) => {
            println!("  {} Git: {}", style("✗").red(), e);
            issues += 1;
        }
    }

    // 2. Claude 디렉토리 확인
    let claude_dir = SyncConfig::claude_dir();
    if claude_dir.exists() {
        println!(
            "  {} Claude dir: {}",
            style("✓").green(),
            claude_dir.display()
        );
    } else {
        println!(
            "  {} Claude dir not found: {}",
            style("✗").red(),
            claude_dir.display()
        );
        issues += 1;
    }

    // 3. 설정 파일 확인
    let config_path = SyncConfig::config_path();
    if config_path.exists() {
        match SyncConfig::load() {
            Ok(config) => {
                println!("  {} Config: valid", style("✓").green());
                println!("    - Repo: {}", config.repo.url);
                println!("    - Device: {}", config.device.id);
                println!("    - Secret patterns: {}", config.secret_patterns.len());
            }
            Err(e) => {
                println!("  {} Config parse error: {}", style("✗").red(), e);
                issues += 1;
            }
        }
    } else {
        println!(
            "  {} Config not found — run 'claude-sync init'",
            style("!").yellow()
        );
        issues += 1;
    }

    // 4. 싱크 레포 확인
    let repo_path = SyncConfig::repo_path();
    if repo_path.join(".git").exists() {
        println!("  {} Sync repo: {}", style("✓").green(), repo_path.display());

        // 원격 연결 확인
        let repo = git_ops::GitRepo::open_or_init(&repo_path)?;
        match repo.fetch("origin") {
            Ok(_) => println!("  {} Remote: reachable", style("✓").green()),
            Err(e) => {
                println!("  {} Remote: unreachable — {}", style("✗").red(), e);
                issues += 1;
            }
        }
    } else {
        println!(
            "  {} Sync repo not initialized",
            style("!").yellow()
        );
        issues += 1;
    }

    // 5. gh CLI 확인
    match git_ops::get_gh_token() {
        Ok(_) => println!("  {} gh CLI: authenticated", style("✓").green()),
        Err(_) => println!(
            "  {} gh CLI: not available (optional)",
            style("~").dim()
        ),
    }

    // 결과
    println!();
    if issues == 0 {
        println!("{}", style("모든 검사 통과!").bold().green());
    } else {
        println!(
            "{}",
            style(format!("{} issue(s) found", issues)).bold().yellow()
        );
    }

    Ok(())
}
