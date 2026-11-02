use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use console::style;

use claude_sync_core::app::App;
use claude_sync_core::bundle::{ImportMode, ImportReport, PreviewItem, PreviewStatus};
use claude_sync_core::template;

use super::ui;

pub fn inspect(file: &Path) -> Result<()> {
    let app = App::load()?;
    let (bundle, items) = app.preview(file)?;
    print_header(&bundle.manifest, file);
    ui::print_preview(&items);
    Ok(())
}

fn print_header(manifest: &claude_sync_core::bundle::BundleManifest, file: &Path) {
    ui::title(&format!("번들 {}", file.display()));
    println!("  보낸 기기: {} ({})", manifest.device_name, manifest.platform);
    println!("  만든 시각: {}", manifest.created_at);
    if let Some(label) = &manifest.label {
        println!("  설명: {label}");
    }
    if let Some(template) = &manifest.template {
        println!("  템플릿: {template}");
    }
    println!("  항목 {}개\n", manifest.items.len());
}

pub struct ImportArgs {
    pub file: PathBuf,
    pub items: Vec<String>,
    pub all: bool,
    pub template: Option<String>,
    pub mode: String,
    pub yes: bool,
    pub dry_run: bool,
}

pub fn parse_mode(mode: &str) -> Result<ImportMode> {
    ImportMode::from_key(mode).ok_or_else(|| anyhow!("--mode는 merge, overwrite, skip 중 하나입니다"))
}

pub fn run(args: ImportArgs) -> Result<()> {
    let mode = parse_mode(&args.mode)?;
    let app = App::load()?;
    let (bundle, preview) = app.preview(&args.file)?;
    print_header(&bundle.manifest, &args.file);
    ui::print_preview(&preview);
    drop(bundle);

    let selected = select_items(&preview, &args.items, args.all, args.template.as_deref())?;
    if selected.is_empty() {
        ui::warn("가져올 항목이 없습니다");
        return Ok(());
    }
    if args.dry_run {
        println!("\n{}", style("Dry run — 다음 항목을 가져옵니다:").yellow());
        for id in &selected {
            println!("  {id}");
        }
        return Ok(());
    }
    ui::confirm_executables(&preview, &selected, args.yes)?;

    let report = app.import(&args.file, &selected, mode)?;
    print_report(&report);
    Ok(())
}

pub fn select_items(preview: &[PreviewItem], explicit: &[String], all: bool, template_name: Option<&str>) -> Result<Vec<String>> {
    for id in explicit {
        if !preview.iter().any(|p| &p.id == id) {
            return Err(anyhow!("번들에 없는 항목: {id}"));
        }
    }
    let template = template_name.map(template::find).transpose()?;
    let changed = |p: &&PreviewItem| p.status != PreviewStatus::Same;

    if !explicit.is_empty() {
        return Ok(explicit.to_vec());
    }
    if let Some(template) = template {
        return Ok(preview.iter().filter(changed).filter(|p| template.matches(&p.id)).map(|p| p.id.clone()).collect());
    }
    if all {
        return Ok(preview.iter().filter(changed).map(|p| p.id.clone()).collect());
    }
    if !ui::is_interactive() {
        return Err(anyhow!("가져올 항목 id, --all 또는 --template을 지정하세요"));
    }
    ui::pick_preview_items(preview)
}

pub fn print_report(report: &ImportReport) {
    println!();
    ui::title("가져오기 결과");
    if let Some(backup_id) = &report.backup_id {
        ui::ok(format!("가져오기 전 자동 백업: {backup_id} (claude-sync backup restore {backup_id})"));
    }
    if let Some(reason) = &report.backup_skipped_reason {
        ui::info(reason);
    }
    ui::ok(format!("가져온 항목 {}개", report.imported.len()));
    for id in &report.imported {
        println!("    {id}");
    }
    if !report.skipped.is_empty() {
        println!("  건너뜀 {}개", report.skipped.len());
        for skipped in &report.skipped {
            println!("    {} — {}", skipped.id, style(&skipped.reason).dim());
        }
    }
    for warning in &report.warnings {
        ui::warn(warning);
    }
}
