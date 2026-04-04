use anyhow::Result;
use console::style;

use claude_sync_core::config::{SecretAction, SecretPattern, SyncConfig};
use claude_sync_core::secret::SecretEngine;

use super::SecretAction as CliSecretAction;

pub async fn run(action: CliSecretAction) -> Result<()> {
    match action {
        CliSecretAction::List => list().await,
        CliSecretAction::Add { name, json_path } => add(&name, &json_path).await,
        CliSecretAction::Remove { name } => remove(&name).await,
    }
}

async fn list() -> Result<()> {
    let config = SyncConfig::load()?;

    println!("{}", style("Secret Patterns").bold().cyan());
    println!("{}", style("─".repeat(30)).dim());

    for (i, pattern) in config.secret_patterns.iter().enumerate() {
        println!(
            "  {}. {} — {}",
            i + 1,
            style(&pattern.name).bold(),
            style(&pattern.json_path).dim()
        );
    }

    // 현재 설정 파일에서 탐지 결과 표시
    let claude_dir = SyncConfig::claude_dir();
    let settings_path = claude_dir.join("settings.json");

    if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        let engine = SecretEngine::new(&config.secret_patterns);
        let matches = engine.detect(&json);

        println!();
        println!(
            "{} (settings.json)",
            style(format!("Detected: {} secrets", matches.len())).bold()
        );
        for m in &matches {
            let preview = if m.original_value.len() > 8 {
                format!("{}...", &m.original_value[..8])
            } else {
                m.original_value.clone()
            };
            println!(
                "  {} {} = {}",
                style("•").yellow(),
                m.json_path,
                style(preview).dim()
            );
        }
    }

    Ok(())
}

async fn add(name: &str, json_path: &str) -> Result<()> {
    let mut config = SyncConfig::load()?;
    config.secret_patterns.push(SecretPattern {
        name: name.to_string(),
        json_path: json_path.to_string(),
        action: SecretAction::Mask,
    });
    config.save()?;

    println!(
        "  {} 패턴 추가: {} ({})",
        style("✓").green(),
        name,
        json_path
    );

    Ok(())
}

async fn remove(name: &str) -> Result<()> {
    let mut config = SyncConfig::load()?;
    let before_count = config.secret_patterns.len();
    config.secret_patterns.retain(|p| p.name != name);
    let removed = before_count - config.secret_patterns.len();

    if removed > 0 {
        config.save()?;
        println!("  {} 패턴 제거: {}", style("✓").green(), name);
    } else {
        println!("  {} 패턴을 찾을 수 없음: {}", style("!").yellow(), name);
    }

    Ok(())
}
