use anyhow::Result;
use console::style;

use claude_sync_core::config::SyncConfig;
use claude_sync_core::discovery;

use super::SkillAction;

pub async fn run(action: SkillAction) -> Result<()> {
    let config = SyncConfig::load()?;

    match action {
        SkillAction::List => list_skills(&config).await,
        SkillAction::Push { names } => push_skills(&config, &names).await,
        SkillAction::Pull { names } => pull_skills(&config, &names).await,
    }
}

async fn list_skills(config: &SyncConfig) -> Result<()> {
    println!("{}", style("Installed Skills").bold().cyan());
    println!("{}", style("─".repeat(30)).dim());

    let result = discovery::discover(config)?;
    let repo_path = SyncConfig::repo_path();

    if result.skills.is_empty() {
        println!("  설치된 스킬 없음");
        return Ok(());
    }

    for skill in &result.skills {
        let remote_exists = repo_path.join(&skill.path).exists();
        let status_icon = if remote_exists {
            style("●").green() // 양쪽 존재
        } else {
            style("○").yellow() // 로컬만
        };

        let size_str = if skill.size_bytes > 1_000_000 {
            format!("{:.1}MB", skill.size_bytes as f64 / 1_000_000.0)
        } else if skill.size_bytes > 1_000 {
            format!("{:.1}KB", skill.size_bytes as f64 / 1_000.0)
        } else {
            format!("{}B", skill.size_bytes)
        };

        println!(
            "  {} {} ({}, {} files)",
            status_icon,
            style(&skill.name).bold(),
            style(size_str).dim(),
            skill.files.len()
        );
    }

    // 원격에만 있는 스킬 확인
    let remote_skills = repo_path.join("skills");
    if remote_skills.exists() {
        for entry in std::fs::read_dir(&remote_skills)? {
            let entry = entry?;
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let local_exists = result.skills.iter().any(|s| s.name == name);
                if !local_exists {
                    println!(
                        "  {} {} (remote only)",
                        style("◌").blue(),
                        style(&name).bold()
                    );
                }
            }
        }
    }

    println!();
    println!(
        "범례: {} 싱크됨  {} 로컬만  {} 원격만",
        style("●").green(),
        style("○").yellow(),
        style("◌").blue()
    );

    Ok(())
}

async fn push_skills(config: &SyncConfig, names: &[String]) -> Result<()> {
    let result = discovery::discover(config)?;
    let claude_dir = SyncConfig::claude_dir();
    let repo_path = SyncConfig::repo_path();

    let skills_to_push: Vec<_> = if names.is_empty() {
        result.skills.clone()
    } else {
        result
            .skills
            .iter()
            .filter(|s| names.contains(&s.name))
            .cloned()
            .collect()
    };

    if skills_to_push.is_empty() {
        println!("  Push 대상 스킬 없음");
        return Ok(());
    }

    for skill in &skills_to_push {
        let src = claude_dir.join(&skill.path);
        let dst = repo_path.join(&skill.path);
        copy_dir_recursive(&src, &dst)?;
        println!(
            "  {} {} pushed",
            style("✓").green(),
            skill.name
        );
    }

    Ok(())
}

async fn pull_skills(_config: &SyncConfig, names: &[String]) -> Result<()> {
    let repo_path = SyncConfig::repo_path();
    let claude_dir = SyncConfig::claude_dir();
    let remote_skills = repo_path.join("skills");

    if !remote_skills.exists() {
        println!("  원격에 스킬 없음");
        return Ok(());
    }

    for entry in std::fs::read_dir(&remote_skills)? {
        let entry = entry?;
        if !entry.path().is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if !names.is_empty() && !names.contains(&name) {
            continue;
        }

        let dst = claude_dir.join("skills").join(&name);
        copy_dir_recursive(&entry.path(), &dst)?;
        println!(
            "  {} {} pulled",
            style("✓").green(),
            name
        );
    }

    Ok(())
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        let name = entry.file_name().to_string_lossy().to_string();
        if name == "node_modules" || name == ".git" || name == "target" {
            continue;
        }

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
