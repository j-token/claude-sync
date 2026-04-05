use anyhow::Result;
use console::style;

use claude_sync_core::config::SyncConfig;
use claude_sync_core::discovery::FileCategory;
use claude_sync_core::git_ops::GitRepo;
use claude_sync_core::manifest::SyncManifest;
use claude_sync_core::secret::SecretEngine;
use claude_sync_core::snapshot;

pub async fn run(force: bool, dry_run: bool) -> Result<()> {
    let config = SyncConfig::load()?;

    println!("{}", style("Claude Sync Pull").bold().cyan());
    println!("{}", style("─".repeat(30)).dim());

    let repo_path = SyncConfig::repo_path();
    let claude_dir = SyncConfig::claude_dir();

    // 1. 스냅샷 생성 (복원 대비)
    if !dry_run {
        let snap = snapshot::create_snapshot()?;
        println!(
            "  {} 스냅샷 생성: {} ({} files)",
            style("✓").green(),
            snap.id,
            snap.file_count
        );
    }

    // 2. Git pull
    let repo = GitRepo::open_or_init(&repo_path)?;

    if !dry_run {
        println!("  {} 원격에서 가져오는 중...", style("→").yellow());
        match repo.pull("origin", &config.repo.branch) {
            Ok(_) => println!("  {} Pull 완료", style("✓").green()),
            Err(e) => {
                if force {
                    println!("  {} Pull 실패, force 모드로 계속: {}", style("!").yellow(), e);
                } else {
                    anyhow::bail!("Pull 실패: {}. --force 옵션으로 재시도하세요.", e);
                }
            }
        }
    }

    // 3. 매니페스트 로드
    let manifest_path = repo_path.join("manifest.json");
    let manifest = if manifest_path.exists() {
        SyncManifest::load(&manifest_path)?
    } else {
        println!("  {} 매니페스트 없음 — 원격이 비어있습니다", style("!").yellow());
        return Ok(());
    };

    // 4. 시크릿 엔진
    let secret_engine = SecretEngine::new(&config.secret_patterns);

    // 5. 파일 복원 (시크릿 언마스킹 포함)
    let mut restored_count = 0;

    for entry in &manifest.files {
        let src = repo_path.join(&entry.path);
        let dst = claude_dir.join(&entry.path);

        if !src.exists() {
            continue;
        }

        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // JSON 파일은 시크릿 언마스킹
        match entry.category {
            FileCategory::Settings | FileCategory::McpJson => {
                let remote_content = std::fs::read_to_string(&src)?;
                let remote_json: serde_json::Value = serde_json::from_str(&remote_content)?;

                // 로컬 JSON이 있으면 시크릿 값 복원
                let final_json = if dst.exists() {
                    let local_content = std::fs::read_to_string(&dst)?;
                    let local_json: serde_json::Value = serde_json::from_str(&local_content)?;
                    secret_engine.unmask(&remote_json, &local_json)
                } else {
                    remote_json
                };

                if dry_run {
                    println!(
                        "  {} {} (secrets: {})",
                        style("[DRY]").yellow(),
                        entry.path,
                        entry.masked_secrets.len()
                    );
                } else {
                    let content = serde_json::to_string_pretty(&final_json)?;
                    std::fs::write(&dst, content)?;
                    restored_count += 1;
                }
            }
            _ => {
                if dry_run {
                    println!("  {} {}", style("[DRY]").yellow(), entry.path);
                } else {
                    std::fs::copy(&src, &dst)?;
                    restored_count += 1;
                }
            }
        }
    }

    // 6. Skills 복원
    let skills_src = repo_path.join("skills");
    if skills_src.exists() && config.sync.sync_skills {
        let skills_dst = claude_dir.join("skills");
        for entry in std::fs::read_dir(&skills_src)? {
            let entry = entry?;
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let dst = skills_dst.join(&name);

                if dry_run {
                    println!("  {} skill: {}", style("[DRY]").yellow(), name);
                } else {
                    copy_dir_recursive(&entry.path(), &dst)?;
                    restored_count += 1;
                }
            }
        }
    }

    println!();
    if dry_run {
        println!("{}", style("Dry run 완료 — 변경 없음").bold().yellow());
    } else {
        println!(
            "{} ({} items restored)",
            style("Pull 완료!").bold().green(),
            restored_count
        );
    }

    Ok(())
}

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
