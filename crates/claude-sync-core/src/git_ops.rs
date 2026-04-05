use std::path::Path;
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::error::{Result, SyncError};

/// Windows에서 콘솔 창을 숨기는 플래그
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Git 명령 실행 결과
#[derive(Debug)]
pub struct GitOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// 인증 상태 정보
#[derive(Debug, Clone)]
pub struct AuthStatus {
    pub git_available: bool,
    pub git_version: Option<String>,
    pub gh_cli_available: bool,
    pub gh_authenticated: bool,
    pub gh_username: Option<String>,
    pub ssh_key_found: bool,
}

/// 콘솔 창을 숨기고 Command 생성하는 헬퍼
fn git_command() -> Command {
    let mut cmd = Command::new("git");
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// 임의의 실행파일에 대해 콘솔 숨김 Command 생성
fn silent_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// 시스템 git 명령어를 사용하는 Git 래퍼
pub struct GitRepo {
    work_dir: std::path::PathBuf,
}

impl GitRepo {
    /// 기존 레포 열기 또는 새로 초기화
    pub fn open_or_init(path: &Path) -> Result<Self> {
        std::fs::create_dir_all(path)?;

        let repo = Self {
            work_dir: path.to_path_buf(),
        };

        if !path.join(".git").exists() {
            repo.run_git(&["init"])?;
        }

        Ok(repo)
    }

    /// 원격 레포 클론
    pub fn clone_repo(url: &str, path: &Path) -> Result<Self> {
        std::fs::create_dir_all(path)?;

        let output = git_command()
            .args(["clone", url, &path.to_string_lossy()])
            .output()
            .map_err(|e| SyncError::Git(format!("Failed to run git clone: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SyncError::Git(format!("git clone failed: {stderr}")));
        }

        Ok(Self {
            work_dir: path.to_path_buf(),
        })
    }

    /// remote 설정
    pub fn set_remote(&self, name: &str, url: &str) -> Result<()> {
        let _ = self.run_git(&["remote", "remove", name]);
        self.run_git(&["remote", "add", name, url])?;
        Ok(())
    }

    /// 브랜치 설정
    pub fn set_branch(&self, branch: &str) -> Result<()> {
        let result = self.run_git(&["branch", "--show-current"]);
        match result {
            Ok(output) if output.stdout.trim().is_empty() => {}
            _ => {
                let _ = self.run_git(&["branch", "-M", branch]);
            }
        }
        Ok(())
    }

    /// 파일 스테이징
    pub fn add(&self, paths: &[&str]) -> Result<()> {
        let mut args = vec!["add"];
        args.extend(paths);
        self.run_git(&args)?;
        Ok(())
    }

    /// 모든 변경사항 스테이징
    pub fn add_all(&self) -> Result<()> {
        self.run_git(&["add", "-A"])?;
        Ok(())
    }

    /// 커밋
    pub fn commit(&self, message: &str) -> Result<()> {
        let output = self.run_git(&["commit", "-m", message])?;
        if output.stdout.contains("nothing to commit")
            || output.stderr.contains("nothing to commit")
        {
            tracing::info!("Nothing to commit");
        }
        Ok(())
    }

    /// 푸시
    pub fn push(&self, remote: &str, branch: &str) -> Result<()> {
        self.run_git(&["push", "-u", remote, &format!("{branch}:{branch}")])?;
        Ok(())
    }

    /// 풀 (fetch + merge)
    pub fn pull(&self, remote: &str, branch: &str) -> Result<()> {
        self.run_git(&["pull", remote, branch, "--allow-unrelated-histories"])?;
        Ok(())
    }

    /// 페치
    pub fn fetch(&self, remote: &str) -> Result<()> {
        self.run_git(&["fetch", remote])?;
        Ok(())
    }

    /// 현재 상태
    pub fn status(&self) -> Result<GitOutput> {
        self.run_git(&["status", "--porcelain"])
    }

    /// 로컬과 원격 간의 diff 파일 목록
    pub fn diff_names(&self, remote: &str, branch: &str) -> Result<Vec<String>> {
        let output =
            self.run_git(&["diff", "--name-only", &format!("{remote}/{branch}"), "HEAD"]);
        match output {
            Ok(out) => Ok(out
                .stdout
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect()),
            Err(_) => Ok(Vec::new()),
        }
    }

    /// 커밋 로그
    pub fn log(&self, count: usize) -> Result<GitOutput> {
        self.run_git(&["log", &format!("--max-count={count}"), "--oneline"])
    }

    /// 로컬이 원격보다 앞서 있는 커밋 수
    pub fn ahead_behind(&self, remote: &str, branch: &str) -> Result<(usize, usize)> {
        let output = self.run_git(&[
            "rev-list",
            "--left-right",
            "--count",
            &format!("HEAD...{remote}/{branch}"),
        ]);

        match output {
            Ok(out) => {
                let parts: Vec<&str> = out.stdout.trim().split('\t').collect();
                if parts.len() == 2 {
                    let ahead = parts[0].parse().unwrap_or(0);
                    let behind = parts[1].parse().unwrap_or(0);
                    Ok((ahead, behind))
                } else {
                    Ok((0, 0))
                }
            }
            Err(_) => Ok((0, 0)),
        }
    }

    /// HTTPS credential 설정 (PAT 기반)
    pub fn set_https_credential(&self, token: &str) -> Result<()> {
        // extraheader로 토큰 설정
        self.run_git(&[
            "config",
            "http.extraHeader",
            &format!("Authorization: token {token}"),
        ])?;
        Ok(())
    }

    /// HTTPS URL로 remote 변경 (PAT 임베드)
    pub fn set_remote_with_token(&self, name: &str, url: &str, token: &str) -> Result<()> {
        // https://github.com/user/repo.git → https://TOKEN@github.com/user/repo.git
        let authed_url = if url.starts_with("https://") {
            url.replacen("https://", &format!("https://{}@", token), 1)
        } else {
            url.to_string()
        };
        let _ = self.run_git(&["remote", "remove", name]);
        self.run_git(&["remote", "add", name, &authed_url])?;
        Ok(())
    }

    /// git 명령 실행 (콘솔 창 숨김)
    fn run_git(&self, args: &[&str]) -> Result<GitOutput> {
        let output = git_command()
            .current_dir(&self.work_dir)
            .args(args)
            .output()
            .map_err(|e| {
                SyncError::Git(format!(
                    "Failed to run git {}: {e}",
                    args.first().unwrap_or(&"")
                ))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            let cmd = args.join(" ");
            tracing::warn!("git {cmd} failed: {stderr}");
            return Err(SyncError::Git(format!("git {cmd}: {stderr}")));
        }

        Ok(GitOutput {
            success: output.status.success(),
            stdout,
            stderr,
        })
    }
}

/// git이 설치되어 있는지 확인
pub fn check_git_available() -> Result<String> {
    let output = git_command()
        .args(["--version"])
        .output()
        .map_err(|_| SyncError::Git("git is not installed or not in PATH".to_string()))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(SyncError::Git("git is not available".to_string()))
    }
}

/// gh CLI를 통한 인증 토큰 확인
pub fn get_gh_token() -> Result<String> {
    let output = silent_command("gh")
        .args(["auth", "token"])
        .output()
        .map_err(|_| SyncError::Auth("gh CLI is not installed".to_string()))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(SyncError::Auth(
            "gh auth token failed. Run 'gh auth login' first.".to_string(),
        ))
    }
}

/// gh CLI 로그인된 유저 정보
pub fn get_gh_user() -> Result<String> {
    let output = silent_command("gh")
        .args(["api", "user", "--jq", ".login"])
        .output()
        .map_err(|_| SyncError::Auth("gh CLI is not installed".to_string()))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(SyncError::Auth("Not logged in to gh CLI".to_string()))
    }
}

/// SSH 키 존재 여부 확인
pub fn find_ssh_keys() -> Vec<String> {
    let home = dirs::home_dir().unwrap_or_default();
    let ssh_dir = home.join(".ssh");
    let candidates = ["id_ed25519", "id_rsa", "id_ecdsa"];

    candidates
        .iter()
        .filter(|name| ssh_dir.join(name).exists())
        .map(|name| name.to_string())
        .collect()
}

/// 종합 인증 상태 확인
pub fn check_auth_status() -> AuthStatus {
    let git_check = check_git_available();
    let git_available = git_check.is_ok();
    let git_version = git_check.ok();

    let gh_token = get_gh_token();
    let gh_cli_available = gh_token.is_ok() || {
        silent_command("gh")
            .args(["--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };
    let gh_authenticated = gh_token.is_ok();
    let gh_username = if gh_authenticated {
        get_gh_user().ok()
    } else {
        None
    };

    let ssh_keys = find_ssh_keys();

    AuthStatus {
        git_available,
        git_version,
        gh_cli_available,
        gh_authenticated,
        gh_username,
        ssh_key_found: !ssh_keys.is_empty(),
    }
}
