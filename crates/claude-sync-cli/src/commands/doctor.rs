use anyhow::Result;

use claude_sync_core::app::App;
use claude_sync_core::inventory::ItemKind;
use claude_sync_core::{fsutil, p2p, paths};

use super::ui;

pub fn run() -> Result<()> {
    ui::title("claude-sync 진단");
    let app = App::load()?;

    let claude_dir = &app.env.claude_dir;
    if claude_dir.is_dir() {
        ui::ok(format!("Claude 설정 디렉토리: {}", claude_dir.display()));
    } else {
        ui::warn(format!("Claude 설정 디렉토리가 없습니다: {}", claude_dir.display()));
    }

    for (label, path) in [("settings.json", app.env.settings_path()), (".claude.json", app.env.claude_json.clone())] {
        match fsutil::read_json(&path) {
            Ok(Some(_)) => ui::ok(format!("{label} 읽기 정상")),
            Ok(None) => ui::warn(format!("{label} 없음 ({})", path.display())),
            Err(e) => ui::warn(format!("{label} 오류: {e}")),
        }
    }

    match app.inventory() {
        Ok(inventory) => {
            let counts: Vec<String> = ItemKind::ALL
                .iter()
                .filter(|k| inventory.count(**k) > 0)
                .map(|k| format!("{} {}", k.label(), inventory.count(*k)))
                .collect();
            ui::ok(format!("항목 {}개: {}", inventory.items.len(), counts.join(", ")));
        }
        Err(e) => ui::warn(format!("항목 스캔 실패: {e}")),
    }

    ui::ok(format!("기기 이름: {}", app.config.device.name));
    ui::ok(format!("백업 {}개 ({})", app.backups.list()?.len(), paths::backups_dir().display()));
    ui::ok(format!("LAN 주소: {}", p2p::local_addresses().join(", ")));

    match std::net::TcpListener::bind(("0.0.0.0", app.config.p2p.port)) {
        Ok(_) => ui::ok(format!("P2P 포트 {} 사용 가능", app.config.p2p.port)),
        Err(e) => ui::warn(format!("P2P 포트 {}를 열 수 없습니다 ({e}). 공유 시 임의 포트를 씁니다.", app.config.p2p.port)),
    }
    Ok(())
}
