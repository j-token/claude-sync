use std::io::IsTerminal;

use anyhow::{Result, bail};
use console::style;
use dialoguer::{Confirm, MultiSelect, theme::ColorfulTheme};

use claude_sync_core::bundle::{PreviewItem, PreviewStatus};
use claude_sync_core::fsutil::format_bytes;
use claude_sync_core::inventory::{Inventory, ItemKind};

pub fn is_interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

pub fn title(text: &str) {
    println!("{}", style(text).bold().cyan());
    println!("{}", style("─".repeat(40)).dim());
}

pub fn ok(text: impl AsRef<str>) {
    println!("  {} {}", style("✓").green(), text.as_ref());
}

pub fn info(text: impl AsRef<str>) {
    println!("  {} {}", style("·").dim(), style(text.as_ref()).dim());
}

pub fn warn(text: impl AsRef<str>) {
    println!("  {} {}", style("!").yellow(), text.as_ref());
}

pub fn executable_badge(executable: bool) -> String {
    if executable { style(" [실행]").red().to_string() } else { String::new() }
}

pub fn status_label(status: PreviewStatus) -> String {
    match status {
        PreviewStatus::New => style("새 항목").green().to_string(),
        PreviewStatus::Same => style("동일").dim().to_string(),
        PreviewStatus::Changed => style("변경됨").yellow().to_string(),
    }
}

pub fn kind_from_arg(kind: &str) -> Result<ItemKind> {
    ItemKind::from_key(kind).ok_or_else(|| {
        let keys: Vec<&str> = ItemKind::ALL.iter().map(|k| k.key()).collect();
        anyhow::anyhow!("알 수 없는 종류 '{kind}'. 가능한 값: {}", keys.join(", "))
    })
}

/// 인벤토리에서 대화형으로 항목을 고른다.
pub fn pick_local_items(inventory: &Inventory, prompt: &str) -> Result<Vec<String>> {
    if !is_interactive() {
        bail!("항목 id 또는 --template을 지정하세요 (목록: claude-sync list)");
    }
    let labels: Vec<String> = inventory
        .items
        .iter()
        .map(|item| {
            format!(
                "[{}] {}{}  {}",
                item.kind.label(),
                item.name,
                executable_badge(item.is_executable()),
                style(item.description.clone().unwrap_or_default()).dim()
            )
        })
        .collect();
    let chosen = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("{prompt} (스페이스로 선택, 엔터로 확정)"))
        .items(&labels)
        .max_length(20)
        .interact()?;
    Ok(chosen.into_iter().map(|i| inventory.items[i].id.clone()).collect())
}

pub fn print_preview(items: &[PreviewItem]) {
    for item in items {
        let secrets = if item.masked_secret_count > 0 {
            style(format!(" 시크릿 {}개 비워짐", item.masked_secret_count)).magenta().to_string()
        } else {
            String::new()
        };
        println!(
            "  {:<8} {}{}{}  {}",
            status_label(item.status),
            item.id,
            executable_badge(item.executable),
            secrets,
            style(format_bytes(item.size_bytes)).dim()
        );
        for detail in &item.details {
            println!("           {}", style(detail).dim());
        }
    }
}

/// 미리보기에서 가져올 항목을 대화형으로 고른다. 기본 선택은 실행 항목이 아니면서 동일하지 않은 항목이다.
pub fn pick_preview_items(items: &[PreviewItem]) -> Result<Vec<String>> {
    let candidates: Vec<&PreviewItem> = items.iter().filter(|i| i.status != PreviewStatus::Same).collect();
    if candidates.is_empty() {
        return Ok(Vec::new());
    }
    let labels: Vec<String> = candidates
        .iter()
        .map(|i| format!("{} {}{}", status_label(i.status), i.id, executable_badge(i.executable)))
        .collect();
    let defaults: Vec<bool> = candidates.iter().map(|i| !i.executable).collect();
    let chosen = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("가져올 항목 (실행 항목은 직접 확인 후 선택하세요)")
        .items(&labels)
        .defaults(&defaults)
        .max_length(20)
        .interact()?;
    Ok(chosen.into_iter().map(|i| candidates[i].id.clone()).collect())
}

/// 실행 항목이 포함되면 확인을 받는다. 대화형이 아니면 `--yes` 없이 진행하지 않는다.
pub fn confirm_executables(items: &[PreviewItem], selected: &[String], assume_yes: bool) -> Result<()> {
    let risky: Vec<&PreviewItem> = items
        .iter()
        .filter(|i| i.executable && selected.contains(&i.id))
        .collect();
    if risky.is_empty() || assume_yes {
        return Ok(());
    }
    println!();
    warn("다음 항목은 이 기기에서 명령을 실행할 수 있습니다:");
    for item in &risky {
        println!("    {}", style(&item.id).red());
        for detail in &item.details {
            println!("      {detail}");
        }
    }
    if !is_interactive() {
        bail!("실행 항목이 포함되어 있습니다. 내용을 확인했다면 --yes를 붙여 다시 실행하세요.");
    }
    let accepted = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("이 항목들을 가져올까요?")
        .default(false)
        .interact()?;
    if !accepted {
        bail!("취소했습니다");
    }
    Ok(())
}
