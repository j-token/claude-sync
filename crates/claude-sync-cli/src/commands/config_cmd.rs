use anyhow::Result;
use console::style;

use claude_sync_core::config::SyncConfig;

use super::ConfigAction;

pub async fn run(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => show().await,
        ConfigAction::Set { key, value } => set(&key, &value).await,
        ConfigAction::Edit => edit().await,
    }
}

async fn show() -> Result<()> {
    let config = SyncConfig::load()?;
    let toml_str = toml::to_string_pretty(&config)?;

    println!("{}", style("Current Configuration").bold().cyan());
    println!("{}", style("─".repeat(30)).dim());
    println!("{}", toml_str);

    Ok(())
}

async fn set(key: &str, value: &str) -> Result<()> {
    let mut config = SyncConfig::load()?;

    match key {
        "sync.sync_memory" => config.sync.sync_memory = value.parse()?,
        "sync.sync_teams" => config.sync.sync_teams = value.parse()?,
        "sync.sync_skills" => config.sync.sync_skills = value.parse()?,
        "sync.auto_sync" => config.sync.auto_sync = value.parse()?,
        "repo.url" => config.repo.url = value.to_string(),
        "repo.branch" => config.repo.branch = value.to_string(),
        "device.id" => config.device.id = value.to_string(),
        _ => anyhow::bail!("Unknown config key: {key}"),
    }

    config.save()?;
    println!("  {} {key} = {value}", style("✓").green());
    Ok(())
}

async fn edit() -> Result<()> {
    let config_path = SyncConfig::config_path();
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
        if cfg!(windows) {
            "notepad".to_string()
        } else {
            "vi".to_string()
        }
    });

    std::process::Command::new(&editor)
        .arg(config_path.to_string_lossy().to_string())
        .status()?;

    Ok(())
}
