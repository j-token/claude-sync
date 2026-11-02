use std::path::PathBuf;

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Claude Code 설정 디렉토리. `CLAUDE_SYNC_CLAUDE_DIR`는 테스트·샌드박스용 재정의다.
pub fn claude_dir() -> PathBuf {
    env_path("CLAUDE_SYNC_CLAUDE_DIR")
        .or_else(|| env_path("CLAUDE_CONFIG_DIR"))
        .unwrap_or_else(|| home_dir().join(".claude"))
}

/// 사용자 범위 MCP 서버가 저장되는 `.claude.json`.
/// `CLAUDE_CONFIG_DIR`가 설정되면 Claude Code는 이 파일도 그 디렉토리 안에 둔다.
pub fn claude_json() -> PathBuf {
    if let Some(path) = env_path("CLAUDE_SYNC_CLAUDE_JSON") {
        return path;
    }
    if let Some(dir) = env_path("CLAUDE_SYNC_CLAUDE_DIR").or_else(|| env_path("CLAUDE_CONFIG_DIR")) {
        return dir.join(".claude.json");
    }
    home_dir().join(".claude.json")
}

/// claude-sync 자체 데이터 디렉토리 (설정, 백업, 템플릿, 수신함).
pub fn sync_home() -> PathBuf {
    env_path("CLAUDE_SYNC_HOME").unwrap_or_else(|| home_dir().join(".claude-sync"))
}

pub fn config_file() -> PathBuf {
    sync_home().join("config.toml")
}

pub fn backups_dir() -> PathBuf {
    sync_home().join("backups")
}

pub fn templates_dir() -> PathBuf {
    sync_home().join("templates")
}

pub fn inbox_dir() -> PathBuf {
    sync_home().join("inbox")
}
