use anyhow::Result;
use console::style;

use claude_sync_core::config::SyncConfig;
use claude_sync_core::discovery::{self, FileCategory};
use claude_sync_core::git_ops::GitRepo;
use claude_sync_core::manifest::{self, ManifestEntry, SyncManifest};
use claude_sync_core::platform;
use claude_sync_core::secret::SecretEngine;
use claude_sync_core::snapshot;

pub async fn run(dry_run: bool) -> Result<()> {
    let config = SyncConfig::load()?;

    println!("{}", style("Claude Sync Push").bold().cyan());
    println!("{}", style("─".repeat(30)).dim());

    // 1. 스냅샷 생성
    if !dry_run {
        let snap = snapshot::create_snapshot()?;
        println!(
            "  {} 스냅샷 생성: {} ({} files)",
            style("✓").green(),
            snap.id,
            snap.file_count
        );
        snapshot::prune_snapshots(config.snapshots.max_count)?;
    }

    // 2. 파일 탐색
    let discovery = discovery::discover(&config)?;
    println!(
        "  {} 싱크 대상: {} files, {} skills",
        style("✓").green(),
        discovery.syncable.len(),
        discovery.skills.len()
    );

    // 3. 시크릿 엔진 초기화
    let secret_engine = SecretEngine::new(&config.secret_patterns);
    let repo_path = SyncConfig::repo_path();
    let claude_dir = SyncConfig::claude_dir();

    // 4. 매니페스트 준비
    let mut manifest = SyncManifest::new(&config.device.id, config.device.platform.clone());

    // 5. 파일 복사 (시크릿 마스킹 적용)
    for file in &discovery.syncable {
        let src = claude_dir.join(&file.relative_path);
        let dst = repo_path.join(&file.relative_path);

        if !src.exists() {
            continue;
        }

        // 디렉토리 생성
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = std::fs::read(&src)?;
        let sha256 = manifest::compute_sha256(&content);
        let mut masked_secrets = Vec::new();
        let mut platform_specific = Vec::new();

        // JSON 파일은 시크릿 마스킹 적용
        match file.category {
            FileCategory::Settings | FileCategory::McpJson => {
                let json: serde_json::Value = serde_json::from_slice(&content)?;

                // 시크릿 마스킹
                let (masked_json, matches) = secret_engine.mask(&json);
                masked_secrets = matches.iter().map(|m| m.json_path.clone()).collect();

                // 플랫폼별 경로 감지
                platform_specific = platform::detect_platform_paths(&json);

                let masked_content = serde_json::to_string_pretty(&masked_json)?;

                if dry_run {
                    println!(
                        "  {} {} ({} secrets masked, {} platform paths)",
                        style("[DRY]").yellow(),
                        file.relative_path,
                        masked_secrets.len(),
                        platform_specific.len()
                    );
                } else {
                    std::fs::write(&dst, masked_content)?;
                }
            }
            _ => {
                if dry_run {
                    println!("  {} {}", style("[DRY]").yellow(), file.relative_path);
                } else {
                    std::fs::copy(&src, &dst)?;
                }
            }
        }

        manifest.upsert_entry(ManifestEntry {
            path: file.relative_path.clone(),
            sha256,
            category: file.category.clone(),
            masked_secrets,
            platform_specific,
        });
    }

    // 6. Skills 복사
    for skill in &discovery.skills {
        let skill_src = claude_dir.join(&skill.path);
        let skill_dst = repo_path.join(&skill.path);

        if dry_run {
            println!(
                "  {} skill: {} ({} files, {} bytes)",
                style("[DRY]").yellow(),
                skill.name,
                skill.files.len(),
                skill.size_bytes
            );
        } else {
            copy_dir_recursive(&skill_src, &skill_dst)?;
        }
    }

    // 7. 매니페스트 저장
    if !dry_run {
        manifest.save(&repo_path.join("manifest.json"))?;
    }

    // 8. Git commit & push
    if !dry_run {
        let repo = GitRepo::open_or_init(&repo_path)?;
        repo.add_all()?;

        let status = repo.status()?;
        if status.stdout.trim().is_empty() {
            println!("  {} 변경 사항 없음", style("✓").green());
            return Ok(());
        }

        let message = format!(
            "sync: {} @ {}",
            config.device.id,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
        repo.commit(&message)?;
        println!("  {} 커밋 완료: {}", style("✓").green(), message);

        match repo.push_with_recovery("origin", &config.repo.branch, &config) {
            Ok(result) => {
                if result.recovery_applied {
                    println!("  {} {}", style("⟳").yellow(), result.message);
                }
                println!("  {} 푸시 완료", style("✓").green());
            }
            Err(e) => {
                println!("  {} 푸시 실패: {}", style("✗").red(), e);
            }
        }
    }

    println!();
    println!("{}", style("Push 완료!").bold().green());

    Ok(())
}

/// GitHub 파일 크기 제한(100MB)보다 낮은 안전 한계
const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        let name = entry.file_name().to_string_lossy().to_string();
        if matches!(
            name.as_str(),
            "node_modules" | ".git" | "target" | "bin" | "obj" | "dist" | "build"
        ) {
            continue;
        }

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            if std::fs::metadata(&src_path)?.len() > MAX_FILE_SIZE {
                continue;
            }
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
