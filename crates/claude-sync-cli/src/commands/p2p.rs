use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use console::style;
use dialoguer::{Input, Select, theme::ColorfulTheme};
use indicatif::{ProgressBar, ProgressStyle};

use claude_sync_core::app::App;
use claude_sync_core::fsutil::format_bytes;
use claude_sync_core::p2p::{self, ShareOptions, SharePhase};
use claude_sync_core::paths;

use super::{import, ui};

pub fn share(items: &[String], template: Option<&str>, port: Option<u16>) -> Result<()> {
    let app = App::load()?;
    let ids = if items.is_empty() && template.is_none() {
        ui::pick_local_items(&app.inventory()?, "공유할 항목")?
    } else {
        items.to_vec()
    };
    let (bundle_path, item_count) = app.build_share_bundle(&ids, template)?;
    let result = run_share(&app, &bundle_path, item_count, port);
    let _ = std::fs::remove_file(&bundle_path);
    result
}

fn run_share(app: &App, bundle_path: &std::path::Path, item_count: usize, port: Option<u16>) -> Result<()> {
    let mut session = p2p::start_share(
        bundle_path,
        item_count,
        ShareOptions {
            port: port.unwrap_or(app.config.p2p.port),
            device_name: app.config.device.name.clone(),
            announce: app.config.p2p.discovery,
            code: None,
        },
    )?;

    ui::title("P2P 공유 대기 중");
    println!("  항목 {}개 · {}", item_count, format_bytes(session.info.size_bytes));
    println!("\n  페어링 코드:  {}\n", style(&session.info.code).bold().green());
    println!("  받는 기기에서 실행:");
    for address in &session.info.addresses {
        println!("    claude-sync receive {address} --code {}", session.info.code);
    }
    println!(
        "\n  {}",
        style("Windows 방화벽이 연결 허용을 물으면 '개인 네트워크'를 허용하세요. Ctrl+C로 중지합니다.").dim()
    );

    let mut last_message = String::new();
    loop {
        let status = session.status();
        if status.message != last_message {
            println!("  {} {}", style("·").dim(), status.message);
            last_message = status.message.clone();
        }
        if status.phase.is_finished() {
            session.wait();
            return match status.phase {
                SharePhase::Completed => {
                    ui::ok("전송 완료");
                    Ok(())
                }
                _ => Err(anyhow!(status.message)),
            };
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

pub fn peers(timeout_secs: u64) -> Result<()> {
    ui::title("주변 공유 기기 찾는 중");
    let peers = p2p::discover(Duration::from_secs(timeout_secs))?;
    if peers.is_empty() {
        println!("  찾지 못했습니다. 상대 화면의 주소(ip:port)를 직접 입력하세요.");
    }
    for peer in peers {
        println!("  {}  {}  항목 {}개", peer.address, peer.device_name, peer.item_count);
    }
    Ok(())
}

pub fn receive(address: Option<String>, code: Option<String>, then_import: bool, mode: &str) -> Result<()> {
    let mode = import::parse_mode(mode)?;
    let app = App::load()?;
    let address = match address {
        Some(address) => address,
        None => pick_peer()?,
    };
    let code = match code {
        Some(code) => code,
        None if ui::is_interactive() => Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt("페어링 코드")
            .interact_text()?,
        None => bail!("--code를 지정하세요"),
    };

    let bar = ProgressBar::new(0);
    bar.set_style(ProgressStyle::with_template("  {bar:30} {bytes}/{total_bytes}").unwrap_or_else(|_| ProgressStyle::default_bar()));
    let received = p2p::receive(&address, &code, &paths::inbox_dir(), &|done, total| {
        bar.set_length(total);
        bar.set_position(done);
    })?;
    bar.finish_and_clear();

    ui::ok(format!(
        "{}에게서 받음: {} ({})",
        received.device_name,
        received.path.display(),
        format_bytes(received.size_bytes)
    ));

    let (bundle, preview) = app.preview(&received.path)?;
    drop(bundle);
    ui::print_preview(&preview);
    if !then_import {
        println!(
            "\n  가져오려면: claude-sync import \"{}\"",
            received.path.display()
        );
        return Ok(());
    }

    let selected = import::select_items(&preview, &[], false, None)?;
    if selected.is_empty() {
        ui::warn("가져올 항목이 없습니다");
        return Ok(());
    }
    ui::confirm_executables(&preview, &selected, false)?;
    let report = app.import(&received.path, &selected, mode)?;
    import::print_report(&report);
    Ok(())
}

fn pick_peer() -> Result<String> {
    println!("  주변 공유 기기를 찾는 중...");
    let peers = p2p::discover(Duration::from_secs(3))?;
    if peers.is_empty() {
        bail!("주변에서 공유 중인 기기를 찾지 못했습니다. 주소를 직접 지정하세요: claude-sync receive <ip:port> --code <코드>");
    }
    if peers.len() == 1 || !ui::is_interactive() {
        return Ok(peers[0].address.clone());
    }
    let labels: Vec<String> = peers
        .iter()
        .map(|p| format!("{} ({}, 항목 {}개)", p.device_name, p.address, p.item_count))
        .collect();
    let index = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("받을 기기")
        .items(&labels)
        .default(0)
        .interact()?;
    Ok(peers[index].address.clone())
}
