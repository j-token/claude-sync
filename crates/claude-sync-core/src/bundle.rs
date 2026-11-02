//! 선택한 항목을 ZIP 번들로 내보내고, 번들을 검사·미리보기·가져온다.
//!
//! 번들 구조:
//! - `claude-sync-bundle.json`: 매니페스트 (항목 목록, JSON 조각 payload)
//! - `files/<claude_dir 기준 상대 경로>`: 파일 항목의 내용

use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::{Result, SyncError};
use crate::fsutil;
use crate::inventory::{self, ClaudeEnv, Inventory, ItemKind, ItemTarget};
use crate::local_json::{ApplyMode, LocalJson};
use crate::secret::{self, SecretEngine};

pub const MANIFEST_NAME: &str = "claude-sync-bundle.json";
pub const FORMAT_VERSION: u32 = 1;
const FILES_PREFIX: &str = "files/";
const MAX_BUNDLE_UNCOMPRESSED_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    pub format: u32,
    pub tool_version: String,
    pub created_at: String,
    pub device_name: String,
    pub platform: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub template: Option<String>,
    pub items: Vec<BundleItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleItem {
    pub id: String,
    pub kind: ItemKind,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub files: Vec<BundleFile>,
    /// JSON 조각 항목의 `{이름: 값}`. 시크릿은 비워 두고 경로는 토큰으로 바꾼 상태다.
    #[serde(default)]
    pub payload: Option<Value>,
    /// payload 안에서 비워 둔 시크릿 위치 (JSON Pointer)
    #[serde(default)]
    pub masked_secrets: Vec<String>,
}

impl BundleItem {
    pub fn size_bytes(&self) -> u64 {
        let files: u64 = self.files.iter().map(|f| f.size).sum();
        let payload = self
            .payload
            .as_ref()
            .and_then(|p| serde_json::to_vec(p).ok())
            .map(|v| v.len() as u64)
            .unwrap_or(0);
        files + payload
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

pub struct ExportOptions {
    pub label: Option<String>,
    pub template: Option<String>,
    pub device_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportSummary {
    pub path: PathBuf,
    pub item_count: usize,
    pub file_count: usize,
    pub size_bytes: u64,
    pub masked_secret_count: usize,
}

pub fn export(
    env: &ClaudeEnv,
    inventory: &Inventory,
    ids: &[String],
    secrets: &SecretEngine,
    options: &ExportOptions,
    out_path: &Path,
) -> Result<ExportSummary> {
    if ids.is_empty() {
        return Err(SyncError::Bundle("내보낼 항목을 하나 이상 선택하세요".into()));
    }
    let local = LocalJson::load(env)?;
    let paths = env.path_context();

    let temp_path = out_path.with_extension("zip.partial");
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut zip = ZipWriter::new(File::create(&temp_path)?);
    let mut options_deflate = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    if let Some(now) = zip_timestamp_now() {
        options_deflate = options_deflate.last_modified_time(now);
    }

    let mut manifest_items = Vec::new();
    let mut file_count = 0;
    let mut masked_secret_count = 0;
    let mut seen = HashSet::new();

    for id in ids {
        if !seen.insert(id.as_str()) {
            continue;
        }
        let item = inventory
            .get(id)
            .ok_or_else(|| SyncError::NotFound(format!("항목 {id}")))?;
        let mut bundle_item = BundleItem {
            id: item.id.clone(),
            kind: item.kind,
            name: item.name.clone(),
            description: item.description.clone(),
            files: Vec::new(),
            payload: None,
            masked_secrets: Vec::new(),
        };

        if let ItemTarget::Files { .. } = item.target {
            for relative in &item.files {
                let bytes = std::fs::read(env.claude_dir.join(relative))?;
                zip.start_file(format!("{FILES_PREFIX}{relative}"), options_deflate)?;
                zip.write_all(&bytes)?;
                bundle_item.files.push(BundleFile {
                    path: relative.clone(),
                    size: bytes.len() as u64,
                    sha256: sha256_hex(&bytes),
                });
                file_count += 1;
            }
        } else {
            let raw = local
                .payload(&item.target)
                .ok_or_else(|| SyncError::NotFound(format!("항목 {id}의 로컬 값")))?;
            let (masked, matches) = secrets.mask(&raw);
            masked_secret_count += matches.len();
            bundle_item.masked_secrets = matches.into_iter().map(|m| m.pointer).collect();
            bundle_item.payload = Some(paths.tokenize(&masked));
        }
        manifest_items.push(bundle_item);
    }

    let manifest = BundleManifest {
        format: FORMAT_VERSION,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        created_at: chrono::Local::now().to_rfc3339(),
        device_name: options.device_name.clone(),
        platform: format!("{:?}", crate::config::Platform::current()).to_lowercase(),
        label: options.label.clone(),
        template: options.template.clone(),
        items: manifest_items,
    };
    zip.start_file(MANIFEST_NAME, options_deflate)?;
    zip.write_all(&serde_json::to_vec_pretty(&manifest)?)?;
    zip.finish()?;

    std::fs::rename(&temp_path, out_path)?;
    Ok(ExportSummary {
        path: out_path.to_path_buf(),
        item_count: manifest.items.len(),
        file_count,
        size_bytes: std::fs::metadata(out_path)?.len(),
        masked_secret_count,
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// 열린 번들. 열 때 구조와 경로를 모두 검증한다.
pub struct Bundle {
    pub path: PathBuf,
    pub manifest: BundleManifest,
    archive: ZipArchive<File>,
}

impl Bundle {
    pub fn open(path: &Path) -> Result<Self> {
        let mut archive = ZipArchive::new(File::open(path)?)
            .map_err(|e| SyncError::Bundle(format!("ZIP 파일을 열 수 없습니다: {e}")))?;

        let mut total_bytes = 0u64;
        for index in 0..archive.len() {
            let entry = archive.by_index(index)?;
            if entry.enclosed_name().is_none() {
                let name = entry.name().map(|n| n.to_string()).unwrap_or_default();
                return Err(SyncError::Bundle(format!("허용되지 않는 경로가 포함됨: {name}")));
            }
            total_bytes += entry.size();
        }
        if total_bytes > MAX_BUNDLE_UNCOMPRESSED_BYTES {
            return Err(SyncError::Bundle("압축 해제 크기가 너무 큽니다 (1GB 초과)".into()));
        }

        let manifest: BundleManifest = {
            let entry = archive
                .by_name(MANIFEST_NAME)
                .map_err(|_| SyncError::Bundle("claude-sync 번들이 아닙니다 (매니페스트 없음)".into()))?;
            let mut bytes = Vec::new();
            entry.take(MAX_MANIFEST_BYTES).read_to_end(&mut bytes)?;
            serde_json::from_slice(&bytes)
                .map_err(|e| SyncError::Bundle(format!("매니페스트를 읽을 수 없습니다: {e}")))?
        };
        if manifest.format > FORMAT_VERSION {
            return Err(SyncError::Bundle(format!(
                "더 새로운 번들 형식(v{})입니다. claude-sync를 업데이트하세요.",
                manifest.format
            )));
        }

        let mut bundle = Self {
            path: path.to_path_buf(),
            manifest,
            archive,
        };
        bundle.validate_items()?;
        Ok(bundle)
    }

    fn validate_items(&mut self) -> Result<()> {
        let mut ids = HashSet::new();
        for item in &self.manifest.items {
            if !ids.insert(item.id.clone()) {
                return Err(SyncError::Bundle(format!("중복 항목: {}", item.id)));
            }
            let target = inventory::target_from_id(item.kind, &item.id)
                .ok_or_else(|| SyncError::Bundle(format!("잘못된 항목 id: {}", item.id)))?;
            match &target {
                ItemTarget::Files { root } => {
                    if item.files.is_empty() {
                        return Err(SyncError::Bundle(format!("{}: 파일이 없습니다", item.id)));
                    }
                    for file in &item.files {
                        fsutil::safe_relative_path(&file.path)?;
                        let inside_root = file.path == *root || file.path.starts_with(&format!("{root}/"));
                        if !inside_root {
                            return Err(SyncError::Bundle(format!("{}: 범위를 벗어난 파일 {}", item.id, file.path)));
                        }
                        let entry = self
                            .archive
                            .by_name(&format!("{FILES_PREFIX}{}", file.path))
                            .map_err(|_| SyncError::Bundle(format!("{}: 파일 누락 {}", item.id, file.path)))?;
                        if entry.size() != file.size || entry.size() > fsutil::MAX_FILE_BYTES {
                            return Err(SyncError::Bundle(format!("{}: 파일 크기 불일치 {}", item.id, file.path)));
                        }
                    }
                }
                _ => {
                    if !item.payload.as_ref().is_some_and(Value::is_object) {
                        return Err(SyncError::Bundle(format!("{}: payload가 없습니다", item.id)));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn item(&self, id: &str) -> Option<&BundleItem> {
        self.manifest.items.iter().find(|i| i.id == id)
    }

    pub fn ids(&self) -> Vec<String> {
        self.manifest.items.iter().map(|i| i.id.clone()).collect()
    }

    fn read_file(&mut self, file: &BundleFile) -> Result<Vec<u8>> {
        let entry = self.archive.by_name(&format!("{FILES_PREFIX}{}", file.path))?;
        let mut bytes = Vec::with_capacity(file.size as usize);
        entry.take(fsutil::MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
        if sha256_hex(&bytes) != file.sha256 {
            return Err(SyncError::Bundle(format!("파일 무결성 검사 실패: {}", file.path)));
        }
        Ok(bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewStatus {
    New,
    Same,
    Changed,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreviewItem {
    pub id: String,
    pub kind: ItemKind,
    pub name: String,
    pub description: Option<String>,
    pub size_bytes: u64,
    pub file_count: usize,
    pub executable: bool,
    pub status: PreviewStatus,
    pub masked_secret_count: usize,
    pub details: Vec<String>,
}

pub fn preview(bundle: &Bundle, env: &ClaudeEnv) -> Result<Vec<PreviewItem>> {
    let local = LocalJson::load(env)?;
    let paths = env.path_context();
    let mut previews = Vec::new();
    for item in &bundle.manifest.items {
        let target = target_of(item)?;
        let status = match &target {
            ItemTarget::Files { root } => files_status(env, root, &item.files),
            _ => {
                let (incoming, _) = resolve_payload(item, &target, &local, &paths);
                match local.payload(&target) {
                    None => PreviewStatus::New,
                    Some(existing) if existing == incoming => PreviewStatus::Same,
                    Some(_) => PreviewStatus::Changed,
                }
            }
        };
        previews.push(PreviewItem {
            id: item.id.clone(),
            kind: item.kind,
            name: item.name.clone(),
            description: item.description.clone(),
            size_bytes: item.size_bytes(),
            file_count: item.files.len(),
            executable: inventory::is_executable(item.kind, &target),
            status,
            masked_secret_count: item.masked_secrets.len(),
            details: details_of(item, &target),
        });
    }
    Ok(previews)
}

/// zip은 시각을 지정하지 않으면 1980-01-01로 기록한다. ZIP 형식은 1980~2107년만 표현할 수 있어 범위 밖이면 None이다.
fn zip_timestamp_now() -> Option<zip::DateTime> {
    use chrono::{Datelike, Timelike};
    let now = chrono::Local::now();
    let year = u16::try_from(now.year()).ok()?;
    zip::DateTime::from_date_and_time(year, now.month() as u8, now.day() as u8, now.hour() as u8, now.minute() as u8, now.second() as u8).ok()
}

fn target_of(item: &BundleItem) -> Result<ItemTarget> {
    inventory::target_from_id(item.kind, &item.id)
        .ok_or_else(|| SyncError::Bundle(format!("잘못된 항목 id: {}", item.id)))
}

fn files_status(env: &ClaudeEnv, root: &str, files: &[BundleFile]) -> PreviewStatus {
    if !env.claude_dir.join(root).exists() {
        return PreviewStatus::New;
    }
    let all_equal = files.iter().all(|file| {
        std::fs::read(env.claude_dir.join(&file.path))
            .map(|bytes| sha256_hex(&bytes) == file.sha256)
            .unwrap_or(false)
    });
    if all_equal { PreviewStatus::Same } else { PreviewStatus::Changed }
}

/// payload의 경로 토큰을 이 기기 경로로 바꾸고, 비워 둔 시크릿은 로컬 값으로 채운다.
fn resolve_payload(
    item: &BundleItem,
    target: &ItemTarget,
    local: &LocalJson,
    paths: &crate::portable::PathContext,
) -> (Value, Vec<String>) {
    let payload = item.payload.clone().unwrap_or(Value::Null);
    let mut resolved = paths.detokenize(&payload);
    let existing = local.payload(target);
    let unresolved = secret::restore_masked(&mut resolved, &item.masked_secrets, existing.as_ref());
    (resolved, unresolved)
}

fn details_of(item: &BundleItem, target: &ItemTarget) -> Vec<String> {
    let payload = item.payload.as_ref();
    match target {
        ItemTarget::HookEvent(event) => payload
            .and_then(|p| p.get(event))
            .map(inventory::hook_commands)
            .unwrap_or_default(),
        ItemTarget::McpServer { name, .. } => payload
            .and_then(|p| p.get(name))
            .map(|server| vec![inventory::mcp_description(server)])
            .unwrap_or_default(),
        ItemTarget::Plugin { .. } => {
            let source = payload.and_then(|p| p.get("marketplace_source")).filter(|s| s.is_object());
            match source {
                Some(source) => vec![format!("마켓플레이스 소스: {source}")],
                None => vec!["마켓플레이스 소스 정보 없음 (이미 등록된 마켓플레이스가 필요합니다)".into()],
            }
        }
        ItemTarget::SettingsKey(key) if inventory::is_executable(item.kind, target) => {
            let Some(value) = payload.and_then(|p| p.get(key)) else {
                return Vec::new();
            };
            match (inventory::command_line(value), value.as_object()) {
                (Some(line), _) => vec![line],
                (None, Some(map)) => map.iter().map(|(k, v)| format!("{k} = {v}")).collect(),
                (None, None) => vec![value.to_string()],
            }
        }
        ItemTarget::Files { .. } if item.kind == ItemKind::HookScript => {
            item.files.iter().map(|f| format!("스크립트 파일: {}", f.path)).collect()
        }
        _ => Vec::new(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportMode {
    Merge,
    Overwrite,
    SkipExisting,
}

impl ImportMode {
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "merge" => Some(ImportMode::Merge),
            "overwrite" => Some(ImportMode::Overwrite),
            "skip_existing" | "skip" => Some(ImportMode::SkipExisting),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SkippedItem {
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ImportReport {
    pub imported: Vec<String>,
    pub skipped: Vec<SkippedItem>,
    pub warnings: Vec<String>,
    pub backup_id: Option<String>,
    /// 자동 백업을 만들지 않았을 때 그 이유
    pub backup_skipped_reason: Option<String>,
}

impl ImportReport {
    fn skip(&mut self, id: &str, reason: &str) {
        self.skipped.push(SkippedItem {
            id: id.to_string(),
            reason: reason.to_string(),
        });
    }
}

pub fn import(bundle: &mut Bundle, env: &ClaudeEnv, ids: &[String], mode: ImportMode) -> Result<ImportReport> {
    let statuses: Vec<(String, PreviewStatus)> = preview(bundle, env)?
        .into_iter()
        .map(|p| (p.id, p.status))
        .collect();
    let mut local = LocalJson::load(env)?;
    let paths = env.path_context();
    let mut report = ImportReport::default();

    for id in ids {
        let Some(item) = bundle.item(id).cloned() else {
            report.skip(id, "번들에 없는 항목");
            continue;
        };
        let status = statuses
            .iter()
            .find(|(sid, _)| sid == id)
            .map(|(_, s)| *s)
            .unwrap_or(PreviewStatus::New);
        if status == PreviewStatus::Same {
            report.skip(id, "이미 동일함");
            continue;
        }
        if status == PreviewStatus::Changed && mode == ImportMode::SkipExisting {
            report.skip(id, "기존 항목 유지");
            continue;
        }

        let target = target_of(&item)?;
        match &target {
            ItemTarget::Files { .. } => {
                for file in &item.files {
                    let bytes = bundle.read_file(file)?;
                    let destination = env.claude_dir.join(fsutil::safe_relative_path(&file.path)?);
                    fsutil::atomic_write(&destination, &bytes)?;
                    #[cfg(unix)]
                    if item.kind == ItemKind::HookScript {
                        use std::os::unix::fs::PermissionsExt;
                        std::fs::set_permissions(&destination, std::fs::Permissions::from_mode(0o755))?;
                    }
                }
            }
            _ => {
                let (incoming, unresolved) = resolve_payload(&item, &target, &local, &paths);
                let apply_mode = match mode {
                    ImportMode::Overwrite => ApplyMode::Overwrite,
                    ImportMode::Merge | ImportMode::SkipExisting => ApplyMode::Merge,
                };
                local.apply(&target, &incoming, apply_mode)?;
                for pointer in unresolved {
                    report.warnings.push(format!("{id}: 시크릿 {pointer} 값이 비어 있습니다. 직접 입력하세요."));
                }
                if inventory::is_executable(item.kind, &target) {
                    for missing in paths.missing_local_paths(&incoming) {
                        report.warnings.push(format!("{id}: 참조하는 경로가 이 기기에 없습니다: {missing}"));
                    }
                }
                if item.kind == ItemKind::Plugin {
                    report.warnings.push(format!(
                        "{id}: settings.json에 등록했습니다. Claude Code를 다시 시작한 뒤 /plugin에서 설치 상태를 확인하세요."
                    ));
                }
            }
        }
        report.imported.push(id.clone());
    }

    local.save(env)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SyncConfig;
    use crate::inventory::test_support::TempEnv;
    use serde_json::json;

    fn export_all(source: &TempEnv, ids: &[&str]) -> PathBuf {
        let inventory = inventory::scan(&source.env).unwrap();
        let out = source.dir.join("out.zip");
        let ids: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
        export(
            &source.env,
            &inventory,
            &ids,
            &SecretEngine::new(&SyncConfig::default_secret_patterns()),
            &ExportOptions {
                label: Some("test".into()),
                template: None,
                device_name: "src".into(),
            },
            &out,
        )
        .unwrap();
        out
    }

    fn source_env() -> TempEnv {
        let source = TempEnv::new("bundle-src");
        source.write("skills/review/SKILL.md", "---\ndescription: 리뷰\n---\n본문");
        source.write("skills/review/ref/guide.md", "guide");
        source.write(
            "settings.json",
            &json!({
                "permissions": {"allow": ["Read"]},
                "hooks": {"Stop": [{"hooks": [{"type": "command", "command": format!("node {}/hooks/a.mjs", source.env.claude_dir.display().to_string().replace('\\', "/"))}]}]}
            })
            .to_string(),
        );
        std::fs::write(
            &source.env.claude_json,
            json!({"mcpServers": {"exa": {"command": "npx", "env": {"EXA_API_KEY": "secret-value"}}}}).to_string(),
        )
        .unwrap();
        source
    }

    #[test]
    fn 내보낸_번들을_다른_기기에_가져오면_파일과_설정이_복원되고_시크릿은_비워진다() {
        let source = source_env();
        let bundle_path = export_all(&source, &["skill:review", "setting:permissions", "hook:Stop", "mcp:user:exa"]);
        let target = TempEnv::new("bundle-dst");
        target.write("settings.json", &json!({"permissions": {"allow": ["Bash(ls)"]}, "model": "opus"}).to_string());
        std::fs::write(&target.env.claude_json, json!({"projects": {"x": 1}}).to_string()).unwrap();

        let mut bundle = Bundle::open(&bundle_path).unwrap();
        let statuses: Vec<(String, PreviewStatus)> =
            preview(&bundle, &target.env).unwrap().into_iter().map(|p| (p.id, p.status)).collect();
        let ids = bundle.ids();
        let report = import(&mut bundle, &target.env, &ids, ImportMode::Merge).unwrap();

        assert_eq!(
            statuses,
            vec![
                ("skill:review".to_string(), PreviewStatus::New),
                ("setting:permissions".to_string(), PreviewStatus::Changed),
                ("hook:Stop".to_string(), PreviewStatus::New),
                ("mcp:user:exa".to_string(), PreviewStatus::New),
            ]
        );
        assert_eq!(report.imported.len(), 4);
        assert_eq!(target.read("skills/review/ref/guide.md"), "guide");
        let settings: Value = serde_json::from_str(&target.read("settings.json")).unwrap();
        assert_eq!(settings["permissions"]["allow"], json!(["Bash(ls)", "Read"]));
        assert_eq!(settings["model"], "opus");
        let hook_command = settings["hooks"]["Stop"][0]["hooks"][0]["command"].as_str().unwrap();
        let target_dir = target.env.claude_dir.display().to_string().replace('\\', "/");
        assert_eq!(hook_command, format!("node {target_dir}/hooks/a.mjs"));
        let claude_json: Value =
            serde_json::from_str(&std::fs::read_to_string(&target.env.claude_json).unwrap()).unwrap();
        assert_eq!(claude_json["projects"]["x"], 1);
        assert_eq!(claude_json["mcpServers"]["exa"]["env"]["EXA_API_KEY"], "");
        assert!(report.warnings.iter().any(|w| w.contains("/exa/env/EXA_API_KEY")));
    }

    #[test]
    fn 같은_기기로_다시_가져오면_시크릿이_유지되고_동일_항목은_건너뛴다() {
        let source = source_env();
        let bundle_path = export_all(&source, &["mcp:user:exa", "skill:review"]);

        let mut bundle = Bundle::open(&bundle_path).unwrap();
        let ids = bundle.ids();
        let report = import(&mut bundle, &source.env, &ids, ImportMode::Merge).unwrap();

        assert!(report.imported.is_empty());
        assert_eq!(report.skipped.len(), 2);
        let claude_json: Value =
            serde_json::from_str(&std::fs::read_to_string(&source.env.claude_json).unwrap()).unwrap();
        assert_eq!(claude_json["mcpServers"]["exa"]["env"]["EXA_API_KEY"], "secret-value");
    }

    #[test]
    fn 범위를_벗어난_파일을_담은_번들은_열리지_않는다() {
        let t = TempEnv::new("bundle-evil");
        let path = t.dir.join("evil.zip");
        let mut zip = ZipWriter::new(File::create(&path).unwrap());
        let opts = SimpleFileOptions::default();
        zip.start_file("files/settings.json", opts).unwrap();
        zip.write_all(b"{}").unwrap();
        let manifest = json!({
            "format": 1, "tool_version": "0", "created_at": "", "device_name": "x", "platform": "linux",
            "items": [{"id": "skill:x", "kind": "skill", "name": "x",
                       "files": [{"path": "settings.json", "size": 2, "sha256": sha256_hex(b"{}")}]}]
        });
        zip.start_file(MANIFEST_NAME, opts).unwrap();
        zip.write_all(manifest.to_string().as_bytes()).unwrap();
        zip.finish().unwrap();

        let error = Bundle::open(&path).err().unwrap().to_string();

        assert!(error.contains("범위를 벗어난"), "{error}");
    }

    #[test]
    fn 받은_번들의_env_설정은_실행_항목으로_표시되고_값을_보여준다() {
        let source = TempEnv::new("bundle-env");
        source.write("settings.json", &json!({"env": {"NODE_OPTIONS": "--require x.js"}}).to_string());
        let bundle_path = export_all(&source, &["setting:env"]);
        let target = TempEnv::new("bundle-env-dst");

        let bundle = Bundle::open(&bundle_path).unwrap();
        let item = preview(&bundle, &target.env).unwrap().remove(0);

        assert!(item.executable);
        assert_eq!(item.details, vec![r#"NODE_OPTIONS = "--require x.js""#]);
    }

    #[test]
    fn 상위_디렉토리를_가리키는_항목_id는_거부한다() {
        assert!(inventory::target_from_id(ItemKind::Agent, "agent:../../evil.md").is_none());
        assert!(inventory::target_from_id(ItemKind::Skill, "skill:a/b").is_none());
        assert!(inventory::target_from_id(ItemKind::Setting, "setting:hooks").is_none());
    }
}
