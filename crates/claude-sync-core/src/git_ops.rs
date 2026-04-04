use std::path::Path;
use std::process::Command;

use crate::error::{Result, SyncError};

/// Git 명령 실행 결과
#[derive(Debug)]
pub struct GitOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
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

        // .git이 없으면 init
        if !path.join(".git").exists() {
            repo.run_git(&["init"])?;
        }

        Ok(repo)
    }

    /// 원격 레포 클론
    pub fn clone_repo(url: &str, path: &Path) -> Result<Self> {
        std::fs::create_dir_all(path)?;

        let output = Command::new("git")
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
        // 기존 remote 제거 시도 (에러 무시)
        let _ = self.run_git(&["remote", "remove", name]);
        self.run_git(&["remote", "add", name, url])?;
        Ok(())
    }

    /// 브랜치 설정
    pub fn set_branch(&self, branch: &str) -> Result<()> {
        // 현재 브랜치가 없으면 orphan 브랜치 생성
        let result = self.run_git(&["branch", "--show-current"]);
        match result {
            Ok(output) if output.stdout.trim().is_empty() => {
                // 커밋이 없는 상태 — 첫 커밋 후에 브랜치 설정
            }
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
        if output.stdout.contains("nothing to commit") || output.stderr.contains("nothing to commit") {
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
        let output = self.run_git(&["diff", "--name-only", &format!("{remote}/{branch}"), "HEAD"]);
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

    /// git 명령 실행
    fn run_git(&self, args: &[&str]) -> Result<GitOutput> {
        let output = Command::new("git")
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
            // 일부 명령은 비정상 종료해도 괜찮은 경우가 있음
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
    let output = Command::new("git")
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
    let output = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .map_err(|_| SyncError::Auth("gh CLI is not installed".to_string()))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(SyncError::Auth("gh auth token failed. Run 'gh auth login' first.".to_string()))
    }
}
