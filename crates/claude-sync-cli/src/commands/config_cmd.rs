use anyhow::{Result, bail};

use claude_sync_core::config::SyncConfig;
use claude_sync_core::{paths, template};

use super::{ConfigAction, ui};

pub fn run(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            ui::title("현재 설정");
            println!("{}", toml::to_string_pretty(&SyncConfig::load()?)?);
        }
        ConfigAction::Set { key, value } => set(&key, &value)?,
        ConfigAction::Edit => {
            let path = paths::config_file();
            if !path.exists() {
                SyncConfig::load()?.save()?;
            }
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| if cfg!(windows) { "notepad".into() } else { "vi".into() });
            std::process::Command::new(editor).arg(&path).status()?;
        }
        ConfigAction::Path => {
            println!("Claude 설정 디렉토리: {}", paths::claude_dir().display());
            println!("사용자 MCP 파일:      {}", paths::claude_json().display());
            println!("claude-sync 설정:     {}", paths::config_file().display());
            println!("백업:                 {}", paths::backups_dir().display());
            println!("템플릿:               {}", paths::templates_dir().display());
            println!("P2P 수신함:           {}", paths::inbox_dir().display());
        }
    }
    Ok(())
}

fn set(key: &str, value: &str) -> Result<()> {
    let mut config = SyncConfig::load()?;
    match key {
        "device.name" => config.device.name = value.to_string(),
        "default_template" => {
            template::find(value)?;
            config.default_template = value.to_string();
        }
        "backup.max_auto_backups" => config.backup.max_auto_backups = value.parse()?,
        "backup.backup_before_import" => config.backup.backup_before_import = value.parse()?,
        "p2p.port" => config.p2p.port = value.parse()?,
        "p2p.discovery" => config.p2p.discovery = value.parse()?,
        _ => bail!("알 수 없는 설정 키: {key}"),
    }
    config.save()?;
    ui::ok(format!("{key} = {value}"));
    Ok(())
}
