//! Tauri 커맨드. 프론트엔드 계약은 `frontend/src/lib/commands.ts`와 `types.ts`에 있다.
//! 인자가 있는 커맨드는 모두 `input` 구조체 하나만 받는다.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::State;

use claude_sync_core::app::App;
use claude_sync_core::backup::BackupInfo;
use claude_sync_core::bundle::{ImportMode, ImportReport, PreviewItem};
use claude_sync_core::config::{SecretAction, SecretPattern};
use claude_sync_core::inventory::ItemKind;
use claude_sync_core::p2p::{self, PeerInfo, ShareInfo, ShareOptions, ShareSession, ShareStatus};
use claude_sync_core::paths;
use claude_sync_core::template::{self, Template};

type CmdResult<T> = Result<T, String>;

async fn blocking<F, T>(f: F) -> CmdResult<T>
where
    F: FnOnce() -> claude_sync_core::error::Result<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| format!("작업 실패: {e}"))?
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct KindCount {
    kind: ItemKind,
    label: String,
    count: usize,
}

#[derive(Serialize)]
struct Overview {
    device_name: String,
    platform: String,
    claude_dir: String,
    sync_home: String,
    kinds: Vec<KindCount>,
    total_items: usize,
    backup_count: usize,
    last_backup: Option<String>,
    template_count: usize,
    default_template: String,
}

#[tauri::command]
async fn get_overview() -> CmdResult<Overview> {
    blocking(|| {
        let app = App::load()?;
        let inventory = app.inventory()?;
        let backups = app.backups.list()?;
        Ok(Overview {
            device_name: app.config.device.name.clone(),
            platform: format!("{:?}", app.config.device.platform).to_lowercase(),
            claude_dir: app.env.claude_dir.display().to_string(),
            sync_home: paths::sync_home().display().to_string(),
            kinds: ItemKind::ALL
                .iter()
                .map(|kind| KindCount {
                    kind: *kind,
                    label: kind.label().to_string(),
                    count: inventory.count(*kind),
                })
                .collect(),
            total_items: inventory.items.len(),
            backup_count: backups.len(),
            last_backup: backups.first().map(|b| b.created_at.clone()),
            template_count: template::list()?.len(),
            default_template: app.config.default_template.clone(),
        })
    })
    .await
}

#[derive(Serialize)]
struct ItemSummary {
    id: String,
    kind: ItemKind,
    name: String,
    description: Option<String>,
    size_bytes: u64,
    file_count: usize,
    executable: bool,
    secret_count: usize,
}

#[tauri::command]
async fn list_items() -> CmdResult<Vec<ItemSummary>> {
    blocking(|| {
        let app = App::load()?;
        let inventory = app.inventory()?;
        let secret_counts = app.secret_counts(&inventory)?;
        Ok(inventory
            .items
            .iter()
            .zip(secret_counts)
            .map(|(item, (_, secret_count))| ItemSummary {
                id: item.id.clone(),
                kind: item.kind,
                name: item.name.clone(),
                description: item.description.clone(),
                size_bytes: item.size_bytes,
                file_count: item.file_count(),
                executable: item.is_executable(),
                secret_count,
            })
            .collect())
    })
    .await
}

#[derive(Serialize)]
struct DetectedSecret {
    item_id: String,
    pointer: String,
    pattern_name: String,
    preview: String,
}

#[tauri::command]
async fn list_secrets() -> CmdResult<Vec<DetectedSecret>> {
    blocking(|| {
        let app = App::load()?;
        Ok(app
            .detect_secrets()?
            .into_iter()
            .map(|d| DetectedSecret {
                preview: d.secret.preview(),
                item_id: d.item_id,
                pointer: d.secret.pointer,
                pattern_name: d.secret.pattern_name,
            })
            .collect())
    })
    .await
}

#[derive(Serialize)]
struct TemplateInfo {
    name: String,
    description: String,
    include: Vec<String>,
    exclude: Vec<String>,
    builtin: bool,
}

impl From<Template> for TemplateInfo {
    fn from(t: Template) -> Self {
        Self {
            name: t.name,
            description: t.description,
            include: t.include,
            exclude: t.exclude,
            builtin: t.builtin,
        }
    }
}

#[derive(Deserialize)]
struct TemplateInput {
    name: String,
    description: String,
    include: Vec<String>,
    exclude: Vec<String>,
}

#[derive(Deserialize)]
struct NameInput {
    name: String,
}

#[tauri::command]
async fn list_templates() -> CmdResult<Vec<TemplateInfo>> {
    blocking(|| Ok(template::list()?.into_iter().map(TemplateInfo::from).collect())).await
}

#[tauri::command]
async fn save_template(input: TemplateInput) -> CmdResult<TemplateInfo> {
    blocking(move || {
        let saved = template::save(&Template {
            name: input.name,
            description: input.description,
            include: input.include,
            exclude: input.exclude,
            builtin: false,
        })?;
        Ok(saved.into())
    })
    .await
}

#[tauri::command]
async fn delete_template(input: NameInput) -> CmdResult<String> {
    blocking(move || {
        template::delete(&input.name)?;
        Ok(format!("템플릿 '{}'을 삭제했습니다", input.name))
    })
    .await
}

#[tauri::command]
async fn resolve_template(input: NameInput) -> CmdResult<Vec<String>> {
    blocking(move || {
        let app = App::load()?;
        let inventory = app.inventory()?;
        app.resolve_selection(&inventory, &[], Some(&input.name))
    })
    .await
}

#[derive(Deserialize)]
struct ExportInput {
    ids: Vec<String>,
    path: String,
    label: Option<String>,
    template: Option<String>,
}

#[derive(Serialize)]
struct ExportResult {
    path: String,
    item_count: usize,
    file_count: usize,
    size_bytes: u64,
    masked_secret_count: usize,
}

#[tauri::command]
async fn export_bundle(input: ExportInput) -> CmdResult<ExportResult> {
    blocking(move || {
        let app = App::load()?;
        let label = input.label.filter(|l| !l.trim().is_empty());
        let summary = app.export_selected(&input.ids, input.template, label, &PathBuf::from(&input.path))?;
        Ok(ExportResult {
            path: summary.path.display().to_string(),
            item_count: summary.item_count,
            file_count: summary.file_count,
            size_bytes: summary.size_bytes,
            masked_secret_count: summary.masked_secret_count,
        })
    })
    .await
}

#[derive(Serialize)]
struct BundlePreview {
    path: String,
    label: Option<String>,
    created_at: String,
    device_name: String,
    platform: String,
    template: Option<String>,
    size_bytes: u64,
    items: Vec<PreviewItem>,
}

fn load_preview(app: &App, path: &std::path::Path) -> claude_sync_core::error::Result<BundlePreview> {
    let (bundle, items) = app.preview(path)?;
    Ok(BundlePreview {
        path: path.display().to_string(),
        label: bundle.manifest.label.clone(),
        created_at: bundle.manifest.created_at.clone(),
        device_name: bundle.manifest.device_name.clone(),
        platform: bundle.manifest.platform.clone(),
        template: bundle.manifest.template.clone(),
        size_bytes: std::fs::metadata(path)?.len(),
        items,
    })
}

#[derive(Deserialize)]
struct PathInput {
    path: String,
}

#[tauri::command]
async fn inspect_bundle(input: PathInput) -> CmdResult<BundlePreview> {
    blocking(move || load_preview(&App::load()?, &PathBuf::from(&input.path))).await
}

#[derive(Deserialize)]
struct ImportInput {
    path: String,
    ids: Vec<String>,
    mode: ImportMode,
}

#[tauri::command]
async fn import_bundle(input: ImportInput) -> CmdResult<ImportReport> {
    blocking(move || {
        if input.ids.is_empty() {
            return Err(claude_sync_core::error::SyncError::Bundle("가져올 항목을 선택하세요".into()));
        }
        App::load()?.import(&PathBuf::from(&input.path), &input.ids, input.mode)
    })
    .await
}

#[tauri::command]
async fn list_backups() -> CmdResult<Vec<BackupInfo>> {
    blocking(|| App::load()?.backups.list()).await
}

#[derive(Deserialize)]
struct CreateBackupInput {
    ids: Option<Vec<String>>,
    template: Option<String>,
    label: Option<String>,
}

#[tauri::command]
async fn create_backup(input: CreateBackupInput) -> CmdResult<BackupInfo> {
    blocking(move || {
        let label = input.label.filter(|l| !l.trim().is_empty());
        App::load()?.create_backup(&input.ids.unwrap_or_default(), input.template.as_deref(), label)
    })
    .await
}

#[derive(Deserialize)]
struct RestoreInput {
    id: String,
    ids: Vec<String>,
}

#[tauri::command]
async fn restore_backup(input: RestoreInput) -> CmdResult<ImportReport> {
    blocking(move || App::load()?.restore(&input.id, &input.ids)).await
}

#[derive(Deserialize)]
struct IdInput {
    id: String,
}

#[tauri::command]
async fn delete_backup(input: IdInput) -> CmdResult<String> {
    blocking(move || {
        App::load()?.backups.delete(&input.id)?;
        Ok(format!("백업 {}을 삭제했습니다", input.id))
    })
    .await
}

/// 진행 중인 P2P 공유. 공유가 끝나면 임시 번들 파일을 지운다.
#[derive(Default)]
struct ShareState(Mutex<Option<ActiveShare>>);

struct ActiveShare {
    session: ShareSession,
    bundle_path: PathBuf,
}

impl ActiveShare {
    fn finish(mut self) {
        self.session.stop();
        let _ = std::fs::remove_file(&self.bundle_path);
    }
}

#[derive(Deserialize)]
struct ShareInput {
    ids: Vec<String>,
}

#[tauri::command]
async fn p2p_start_share(input: ShareInput, state: State<'_, ShareState>) -> CmdResult<ShareInfo> {
    let previous = state.0.lock().map_err(|e| e.to_string())?.take();
    if let Some(previous) = previous {
        tokio::task::spawn_blocking(move || previous.finish()).await.map_err(|e| e.to_string())?;
    }

    let active = blocking(move || {
        let app = App::load()?;
        let (bundle_path, item_count) = app.build_share_bundle(&input.ids, None)?;
        let session = p2p::start_share(
            &bundle_path,
            item_count,
            ShareOptions {
                port: app.config.p2p.port,
                device_name: app.config.device.name.clone(),
                announce: app.config.p2p.discovery,
                code: None,
            },
        );
        match session {
            Ok(session) => Ok(ActiveShare { session, bundle_path }),
            Err(e) => {
                let _ = std::fs::remove_file(&bundle_path);
                Err(e)
            }
        }
    })
    .await?;

    let info = active.session.info.clone();
    *state.0.lock().map_err(|e| e.to_string())? = Some(active);
    Ok(info)
}

#[tauri::command]
async fn p2p_share_status(state: State<'_, ShareState>) -> CmdResult<ShareStatus> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let Some(active) = guard.as_ref() else {
        return Ok(ShareStatus::idle());
    };
    let status = active.session.status();
    if status.phase.is_finished() {
        if let Some(done) = guard.take() {
            done.finish();
        }
    }
    Ok(status)
}

#[tauri::command]
async fn p2p_stop_share(state: State<'_, ShareState>) -> CmdResult<String> {
    let active = state.0.lock().map_err(|e| e.to_string())?.take();
    match active {
        Some(active) => {
            tokio::task::spawn_blocking(move || active.finish()).await.map_err(|e| e.to_string())?;
            Ok("공유를 중지했습니다".into())
        }
        None => Ok("진행 중인 공유가 없습니다".into()),
    }
}

#[derive(Deserialize)]
struct DiscoverInput {
    timeout_ms: u64,
}

#[tauri::command]
async fn p2p_discover(input: DiscoverInput) -> CmdResult<Vec<PeerInfo>> {
    blocking(move || p2p::discover(Duration::from_millis(input.timeout_ms.clamp(500, 15_000)))).await
}

#[derive(Deserialize)]
struct ReceiveInput {
    address: String,
    code: String,
}

#[tauri::command]
async fn p2p_receive(input: ReceiveInput) -> CmdResult<BundlePreview> {
    blocking(move || {
        let received = p2p::receive(input.address.trim(), &input.code, &paths::inbox_dir(), &|_, _| {})?;
        load_preview(&App::load()?, &received.path)
    })
    .await
}

#[derive(Serialize, Deserialize)]
struct SecretPatternInfo {
    name: String,
    json_path: String,
    action: SecretAction,
}

#[derive(Serialize, Deserialize)]
struct AppConfig {
    device_name: String,
    default_template: String,
    max_auto_backups: usize,
    backup_before_import: bool,
    p2p_port: u16,
    p2p_discovery: bool,
    secret_patterns: Vec<SecretPatternInfo>,
}

fn to_app_config(config: &claude_sync_core::config::SyncConfig) -> AppConfig {
    AppConfig {
        device_name: config.device.name.clone(),
        default_template: config.default_template.clone(),
        max_auto_backups: config.backup.max_auto_backups,
        backup_before_import: config.backup.backup_before_import,
        p2p_port: config.p2p.port,
        p2p_discovery: config.p2p.discovery,
        secret_patterns: config
            .secret_patterns
            .iter()
            .map(|p| SecretPatternInfo {
                name: p.name.clone(),
                json_path: p.json_path.clone(),
                action: p.action.clone(),
            })
            .collect(),
    }
}

#[tauri::command]
async fn get_config() -> CmdResult<AppConfig> {
    blocking(|| Ok(to_app_config(&App::load()?.config))).await
}

#[tauri::command]
async fn update_config(input: AppConfig) -> CmdResult<AppConfig> {
    blocking(move || {
        let mut config = claude_sync_core::config::SyncConfig::load()?;
        template::find(&input.default_template)?;
        config.device.name = input.device_name.trim().to_string();
        config.default_template = input.default_template;
        config.backup.max_auto_backups = input.max_auto_backups;
        config.backup.backup_before_import = input.backup_before_import;
        config.p2p.port = input.p2p_port;
        config.p2p.discovery = input.p2p_discovery;
        config.secret_patterns = input
            .secret_patterns
            .into_iter()
            .filter(|p| !p.json_path.trim().is_empty())
            .map(|p| SecretPattern {
                name: p.name,
                json_path: p.json_path.trim().to_string(),
                action: p.action,
            })
            .collect();
        config.save()?;
        Ok(to_app_config(&config))
    })
    .await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(ShareState::default())
        .invoke_handler(tauri::generate_handler![
            get_overview,
            list_items,
            list_secrets,
            list_templates,
            save_template,
            delete_template,
            resolve_template,
            export_bundle,
            inspect_bundle,
            import_bundle,
            list_backups,
            create_backup,
            restore_backup,
            delete_backup,
            p2p_start_share,
            p2p_share_status,
            p2p_stop_share,
            p2p_discover,
            p2p_receive,
            get_config,
            update_config,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri 앱 실행 실패");
}
