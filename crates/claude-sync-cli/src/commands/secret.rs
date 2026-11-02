use anyhow::Result;
use console::style;

use claude_sync_core::app::App;
use claude_sync_core::config::{SecretAction, SecretPattern};

use super::{SecretAction as CliSecretAction, ui};

pub fn run(action: CliSecretAction) -> Result<()> {
    let mut app = App::load()?;
    match action {
        CliSecretAction::List => {
            ui::title("마스킹 패턴");
            for pattern in &app.config.secret_patterns {
                println!("  {} — {}", style(&pattern.name).bold(), style(&pattern.json_path).dim());
            }
            let detected = app.detect_secrets()?;
            println!();
            ui::title(&format!("탐지된 시크릿 {}개 (내보낼 때 비워짐)", detected.len()));
            for d in detected {
                println!(
                    "  {}  {}  {}  {}",
                    d.item_id,
                    d.secret.pointer,
                    style(d.secret.preview()).dim(),
                    style(&d.secret.pattern_name).dim()
                );
            }
        }
        CliSecretAction::Add { name, json_path } => {
            app.config.secret_patterns.push(SecretPattern {
                name: name.clone(),
                json_path: json_path.clone(),
                action: SecretAction::Mask,
            });
            app.config.save()?;
            ui::ok(format!("패턴 추가: {name} ({json_path})"));
        }
        CliSecretAction::Remove { name } => {
            let before = app.config.secret_patterns.len();
            app.config.secret_patterns.retain(|p| p.name != name);
            if app.config.secret_patterns.len() == before {
                ui::warn(format!("패턴을 찾을 수 없음: {name}"));
            } else {
                app.config.save()?;
                ui::ok(format!("패턴 제거: {name}"));
            }
        }
    }
    Ok(())
}
