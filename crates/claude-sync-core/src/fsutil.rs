use std::io::Write;
use std::path::{Component, Path, PathBuf};

use serde_json::Value;

use crate::error::{Result, SyncError};

/// 디렉토리 복사·수집 시 건너뛰는 빌드 산출물과 VCS 디렉토리
pub const SKIPPED_DIR_NAMES: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "__pycache__",
    ".venv",
    "dist",
    "build",
];

/// 번들에 담지 않는 단일 파일 크기 상한
pub const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;

/// 같은 디렉토리의 임시 파일에 쓴 뒤 rename한다.
/// Claude Code가 동시에 읽는 settings.json, .claude.json이 반쯤 쓰인 상태로 보이지 않게 하기 위함이다.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let temp_path = parent.join(format!(".{file_name}.claude-sync-{}.tmp", std::process::id()));
    {
        let mut temp = std::fs::File::create(&temp_path)?;
        temp.write_all(bytes)?;
        temp.sync_all()?;
    }
    if let Err(e) = std::fs::rename(&temp_path, path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(e.into());
    }
    Ok(())
}

pub fn read_json(path: &Path) -> Result<Option<Value>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)?;
    if content.trim().is_empty() {
        return Ok(Some(Value::Object(Default::default())));
    }
    let value = serde_json::from_str(&content)
        .map_err(|e| SyncError::Config(format!("{}: JSON 파싱 실패: {e}", path.display())))?;
    Ok(Some(value))
}

pub fn write_json(path: &Path, value: &Value) -> Result<()> {
    let mut content = serde_json::to_string_pretty(value)?;
    content.push('\n');
    atomic_write(path, content.as_bytes())
}

/// 디렉토리 아래 파일들을 `/` 구분 상대 경로로 수집한다. 결과는 정렬되어 있다.
pub fn collect_files(dir: &Path) -> Result<Vec<(String, u64)>> {
    let mut files = Vec::new();
    if dir.is_dir() {
        collect_files_inner(dir, dir, &mut files)?;
    }
    files.sort();
    Ok(files)
}

fn collect_files_inner(base: &Path, dir: &Path, files: &mut Vec<(String, u64)>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name().to_string_lossy().to_string();
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if !SKIPPED_DIR_NAMES.contains(&name.as_str()) {
                collect_files_inner(base, &entry.path(), files)?;
            }
            continue;
        }
        let size = entry.metadata()?.len();
        if size > MAX_FILE_BYTES {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(base)
            .map(to_slash_path)
            .unwrap_or(name);
        files.push((relative, size));
    }
    Ok(())
}

pub fn to_slash_path(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

/// 외부에서 받은 상대 경로를 검증한다. 절대 경로, `..`, 드라이브 접두사는 거부한다.
pub fn safe_relative_path(raw: &str) -> Result<PathBuf> {
    let reject = || SyncError::Bundle(format!("허용되지 않는 경로: {raw}"));
    if raw.is_empty() || raw.contains('\\') || raw.contains(':') || raw.contains('\0') {
        return Err(reject());
    }
    let path = Path::new(raw);
    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => clean.push(part),
            _ => return Err(reject()),
        }
    }
    if clean.as_os_str().is_empty() {
        return Err(reject());
    }
    Ok(clean)
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let value = bytes as f64;
    if value >= KB * KB {
        format!("{:.1} MB", value / (KB * KB))
    } else if value >= KB {
        format!("{:.1} KB", value / KB)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 상위_디렉토리나_절대_경로는_거부한다() {
        assert!(safe_relative_path("../evil").is_err());
        assert!(safe_relative_path("skills/../../evil").is_err());
        assert!(safe_relative_path("/etc/passwd").is_err());
        assert!(safe_relative_path("C:/Windows").is_err());
        assert!(safe_relative_path("a\\b").is_err());
        assert!(safe_relative_path("").is_err());
    }

    #[test]
    fn 정상_상대_경로는_허용한다() {
        assert_eq!(
            safe_relative_path("skills/foo/SKILL.md").unwrap(),
            PathBuf::from("skills").join("foo").join("SKILL.md")
        );
    }
}
