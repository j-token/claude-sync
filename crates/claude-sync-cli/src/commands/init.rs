use anyhow::Result;
use console::style;
use dialoguer::{Confirm, Input, Select};

use claude_sync_core::config::{
    AuthConfig, AuthMethod, DeviceConfig, Platform, RepoConfig, SnapshotConfig, SyncConfig,
    SyncOptions,
};
use claude_sync_core::git_ops::{self, GitRepo};

pub async fn run() -> Result<()> {
    println!("{}", style("Claude Sync Setup Wizard").bold().cyan());
    println!("{}", style("─".repeat(40)).dim());
    println!();

    // 1. Git 확인
    let git_version = git_ops::check_git_available()
        .map_err(|_| anyhow::anyhow!("git이 설치되어 있지 않습니다. git을 먼저 설치해주세요."))?;
    println!("  {} {}", style("✓").green(), git_version);

    // 2. GitHub 레포 URL 입력
    println!();
    let repo_url: String = Input::new()
        .with_prompt("GitHub 레포 URL (비공개 추천)")
        .with_initial_text("git@github.com:")
        .interact_text()?;

    // 3. 인증 방식 선택
    let auth_options = &["SSH Agent (추천)", "SSH Key", "HTTPS Token", "gh CLI"];
    let auth_choice = Select::new()
        .with_prompt("인증 방식")
        .items(auth_options)
        .default(0)
        .interact()?;

    let auth_method = match auth_choice {
        0 => AuthMethod::SshAgent,
        1 => AuthMethod::SshKey,
        2 => AuthMethod::HttpsToken,
        3 => AuthMethod::GhCli,
        _ => AuthMethod::SshAgent,
    };

    // 4. 디바이스 이름
    let default_hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "my-device".to_string());

    let device_id: String = Input::new()
        .with_prompt("디바이스 이름")
        .default(default_hostname)
        .interact_text()?;

    // 5. 싱크 옵션
    println!();
    println!("{}", style("싱크 옵션").bold());
    let sync_memory = Confirm::new()
        .with_prompt("memory/ 디렉토리 싱크?")
        .default(false)
        .interact()?;

    let sync_teams = Confirm::new()
        .with_prompt("teams/ 디렉토리 싱크?")
        .default(true)
        .interact()?;

    let sync_skills = Confirm::new()
        .with_prompt("skills/ 디렉토리 싱크?")
        .default(true)
        .interact()?;

    // 6. 설정 저장
    let config = SyncConfig {
        schema_version: 1,
        repo: RepoConfig {
            url: repo_url.clone(),
            branch: "main".to_string(),
        },
        auth: AuthConfig {
            method: auth_method,
            ssh_key_path: None,
        },
        device: DeviceConfig {
            id: device_id.clone(),
            platform: Platform::current(),
        },
        sync: SyncOptions {
            auto_sync: false,
            auto_sync_interval_secs: 300,
            sync_memory,
            sync_teams,
            sync_skills,
        },
        secret_patterns: SyncConfig::default_secret_patterns(),
        platform_path_rules: Vec::new(),
        snapshots: SnapshotConfig::default(),
    };

    config.save()?;
    println!();
    println!(
        "  {} 설정 저장: {}",
        style("✓").green(),
        SyncConfig::config_path().display()
    );

    // 7. 싱크 레포 초기화
    let repo_path = SyncConfig::repo_path();
    println!("  {} 싱크 레포 초기화 중...", style("→").yellow());

    if repo_path.join(".git").exists() {
        println!(
            "  {} 기존 싱크 레포 발견: {}",
            style("!").yellow(),
            repo_path.display()
        );
    } else {
        // 원격 레포 클론 시도, 실패하면 로컬 초기화
        match GitRepo::clone_repo(&repo_url, &repo_path) {
            Ok(_repo) => {
                println!("  {} 원격 레포 클론 완료", style("✓").green());
            }
            Err(_) => {
                println!("  {} 원격 레포가 비어있거나 접근 불가. 로컬 초기화합니다.", style("!").yellow());
                let repo = GitRepo::open_or_init(&repo_path)?;
                repo.set_remote("origin", &repo_url)?;
                repo.set_branch("main")?;
                println!("  {} 로컬 레포 초기화 완료", style("✓").green());
            }
        }
    }

    // 8. 완료
    println!();
    println!("{}", style("Setup 완료!").bold().green());
    println!();
    println!("다음 명령어로 설정을 싱크할 수 있습니다:");
    println!("  {} — 로컬 설정을 원격에 푸시", style("claude-sync push").cyan());
    println!(
        "  {} — 원격 설정을 로컬에 풀",
        style("claude-sync pull").cyan()
    );
    println!(
        "  {} — 싱크 상태 확인",
        style("claude-sync status").cyan()
    );

    Ok(())
}
