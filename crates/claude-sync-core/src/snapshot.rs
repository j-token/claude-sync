use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::config::SyncConfig;
use crate::error::{Result, SyncError};

/// 스냅샷 정보
#[derive(Debug, Clone)]
pub struct SnapshotInfo {
    pub id: String,
    pub path: PathBuf,
    pub created_at: String,
    pub file_count: usize,
}

/// Pull 전 현재 ~/.claude/ 설정의 스냅샷 생성
pub fn create_snapshot() -> Result<SnapshotInfo> {
    let claude_dir = SyncConfig::claude_dir();
    let snapshots_dir = SyncConfig::snapshots_path();
    std::fs::create_dir_all(&snapshots_dir)?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let snapshot_id = format!("snapshot_{}", timestamp);
    let snapshot_path = snapshots_dir.join(&snapshot_id);
    std::fs::create_dir_all(&snapshot_path)?;

    let files_to_backup = [
        "CLAUDE.md",
        "settings.json",
        "settings.local.json",
        ".mcp.json",
    ];

    let dirs_to_backup = ["rules", "commands", "agents", "hooks"];

    let mut file_count = 0;

    // 파일 백업
    for filename in &files_to_backup {
        let src = claude_dir.join(filename);
        if src.exists() {
            std::fs::copy(&src, snapshot_path.join(filename))?;
            file_count += 1;
        }
    }

    // 디렉토리 백업
    for dirname in &dirs_to_backup {
        let src_dir = claude_dir.join(dirname);
        if src_dir.exists() && src_dir.is_dir() {
            let dest_dir = snapshot_path.join(dirname);
            file_count += copy_dir_recursive(&src_dir, &dest_dir)?;
        }
    }

    Ok(SnapshotInfo {
        id: snapshot_id,
        path: snapshot_path,
        created_at: timestamp,
        file_count,
    })
}

/// 스냅샷에서 복원
pub fn restore_snapshot(snapshot_id: &str) -> Result<usize> {
    let snapshots_dir = SyncConfig::snapshots_path();
    let snapshot_path = snapshots_dir.join(snapshot_id);

    if !snapshot_path.exists() {
        return Err(SyncError::Snapshot(format!(
            "Snapshot not found: {}",
            snapshot_id
        )));
    }

    let claude_dir = SyncConfig::claude_dir();
    let mut restored_count = 0;

    // 스냅샷의 모든 파일을 claude 디렉토리로 복사
    restored_count += copy_dir_recursive(&snapshot_path, &claude_dir)?;

    Ok(restored_count)
}

/// 사용 가능한 스냅샷 목록
pub fn list_snapshots() -> Result<Vec<SnapshotInfo>> {
    let snapshots_dir = SyncConfig::snapshots_path();
    if !snapshots_dir.exists() {
        return Ok(Vec::new());
    }

    let mut snapshots = Vec::new();

    for entry in std::fs::read_dir(&snapshots_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("snapshot_") {
                let created_at = name
                    .strip_prefix("snapshot_")
                    .unwrap_or(&name)
                    .to_string();

                let file_count = count_files(&path);

                snapshots.push(SnapshotInfo {
                    id: name,
                    path,
                    created_at,
                    file_count,
                });
            }
        }
    }

    // 최신순 정렬
    snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(snapshots)
}

/// 오래된 스냅샷 정리
pub fn prune_snapshots(max_count: usize) -> Result<usize> {
    let mut snapshots = list_snapshots()?;
    let mut pruned = 0;

    if snapshots.len() > max_count {
        // 오래된 것부터 삭제
        let to_remove = snapshots.split_off(max_count);
        for snapshot in &to_remove {
            std::fs::remove_dir_all(&snapshot.path)?;
            pruned += 1;
        }
    }

    Ok(pruned)
}

/// 디렉토리 재귀 복사
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<usize> {
    std::fs::create_dir_all(dst)?;
    let mut count = 0;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            count += copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
            count += 1;
        }
    }

    Ok(count)
}

/// 디렉토리 내 파일 수 세기
fn count_files(dir: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += count_files(&path);
            } else {
                count += 1;
            }
        }
    }
    count
}
