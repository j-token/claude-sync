use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;
use crate::fsutil;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    ClaudeMd,
    Setting,
    Hook,
    HookScript,
    Mcp,
    Plugin,
    Skill,
    Agent,
    Command,
    Rule,
    OutputStyle,
    Keybindings,
    Memory,
    Team,
}

impl ItemKind {
    pub const ALL: [ItemKind; 14] = [
        ItemKind::ClaudeMd,
        ItemKind::Setting,
        ItemKind::Hook,
        ItemKind::HookScript,
        ItemKind::Mcp,
        ItemKind::Plugin,
        ItemKind::Skill,
        ItemKind::Agent,
        ItemKind::Command,
        ItemKind::Rule,
        ItemKind::OutputStyle,
        ItemKind::Keybindings,
        ItemKind::Memory,
        ItemKind::Team,
    ];

    pub fn key(self) -> &'static str {
        match self {
            ItemKind::ClaudeMd => "claude_md",
            ItemKind::Setting => "setting",
            ItemKind::Hook => "hook",
            ItemKind::HookScript => "hook_script",
            ItemKind::Mcp => "mcp",
            ItemKind::Plugin => "plugin",
            ItemKind::Skill => "skill",
            ItemKind::Agent => "agent",
            ItemKind::Command => "command",
            ItemKind::Rule => "rule",
            ItemKind::OutputStyle => "output_style",
            ItemKind::Keybindings => "keybindings",
            ItemKind::Memory => "memory",
            ItemKind::Team => "team",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ItemKind::ClaudeMd => "CLAUDE.md",
            ItemKind::Setting => "설정",
            ItemKind::Hook => "훅",
            ItemKind::HookScript => "훅 스크립트",
            ItemKind::Mcp => "MCP 서버",
            ItemKind::Plugin => "플러그인",
            ItemKind::Skill => "스킬",
            ItemKind::Agent => "에이전트",
            ItemKind::Command => "커맨드",
            ItemKind::Rule => "룰",
            ItemKind::OutputStyle => "출력 스타일",
            ItemKind::Keybindings => "키바인딩",
            ItemKind::Memory => "메모리",
            ItemKind::Team => "팀",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.key() == key)
    }

    /// 파일 디렉토리를 그대로 담는 종류의 기준 디렉토리 (claude_dir 기준)
    fn files_root(self) -> Option<&'static str> {
        match self {
            ItemKind::HookScript => Some("hooks"),
            ItemKind::Skill => Some("skills"),
            ItemKind::Agent => Some("agents"),
            ItemKind::Command => Some("commands"),
            ItemKind::Rule => Some("rules"),
            ItemKind::OutputStyle => Some("output-styles"),
            ItemKind::Memory => Some("memory"),
            ItemKind::Team => Some("teams"),
            _ => None,
        }
    }
}

/// 설정 항목에서 제외하는 settings.json 최상위 키. 별도 종류로 다루거나 의미가 없는 키다.
const SETTINGS_KEYS_HANDLED_ELSEWHERE: &[&str] =
    &["$schema", "hooks", "mcpServers", "enabledPlugins", "extraKnownMarketplaces"];

/// 명령을 실행하거나, 실행 환경(`env`의 NODE_OPTIONS, ANTHROPIC_BASE_URL 등)·권한을 바꿔
/// 결과적으로 임의 코드 실행이나 트래픽 우회로 이어질 수 있는 키. 받은 번들에서는 직접 선택해야 한다.
const EXECUTABLE_SETTINGS_KEYS: &[&str] = &[
    "statusLine",
    "apiKeyHelper",
    "awsAuthRefresh",
    "awsCredentialExport",
    "otelHeadersHelper",
    "env",
    "permissions",
    "skipDangerousModePermissionPrompt",
    "skipAutoPermissionPrompt",
    "enableAllProjectMcpServers",
    "enabledMcpjsonServers",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpScope {
    /// `~/.claude.json`의 `mcpServers` (claude mcp add --scope user)
    User,
    /// `settings.json`의 `mcpServers`
    Settings,
}

impl McpScope {
    pub fn key(self) -> &'static str {
        match self {
            McpScope::User => "user",
            McpScope::Settings => "settings",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "user" => Some(McpScope::User),
            "settings" => Some(McpScope::Settings),
            _ => None,
        }
    }
}

/// 항목 데이터가 어디에 있는지
#[derive(Debug, Clone, PartialEq)]
pub enum ItemTarget {
    /// claude_dir 기준 상대 경로. 파일 하나 또는 디렉토리.
    Files { root: String },
    SettingsKey(String),
    HookEvent(String),
    McpServer { scope: McpScope, name: String },
    Plugin { id: String },
}

#[derive(Debug, Clone)]
pub struct Item {
    pub id: String,
    pub kind: ItemKind,
    pub name: String,
    pub description: Option<String>,
    pub size_bytes: u64,
    pub target: ItemTarget,
    /// Files 대상일 때 claude_dir 기준 상대 경로 목록
    pub files: Vec<String>,
}

impl Item {
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn is_executable(&self) -> bool {
        is_executable(self.kind, &self.target)
    }
}

pub fn is_executable(kind: ItemKind, target: &ItemTarget) -> bool {
    match (kind, target) {
        (ItemKind::Hook | ItemKind::HookScript | ItemKind::Mcp | ItemKind::Plugin, _) => true,
        (ItemKind::Setting, ItemTarget::SettingsKey(key)) => EXECUTABLE_SETTINGS_KEYS.contains(&key.as_str()),
        _ => false,
    }
}

/// 번들에서 온 항목 id를 해석해 쓰기 대상 위치를 정한다. 형식이 틀리거나 위험한 id는 None.
pub fn target_from_id(kind: ItemKind, id: &str) -> Option<ItemTarget> {
    match kind {
        ItemKind::ClaudeMd => (id == "claude_md").then(|| ItemTarget::Files { root: "CLAUDE.md".into() }),
        ItemKind::Keybindings => {
            (id == "keybindings").then(|| ItemTarget::Files { root: "keybindings.json".into() })
        }
        _ => {
            let rest = id.strip_prefix(kind.key())?.strip_prefix(':')?;
            if rest.is_empty() {
                return None;
            }
            target_for_name(kind, rest)
        }
    }
}

fn target_for_name(kind: ItemKind, rest: &str) -> Option<ItemTarget> {
    match kind {
        ItemKind::Setting => {
            let handled = SETTINGS_KEYS_HANDLED_ELSEWHERE.contains(&rest);
            (!handled).then(|| ItemTarget::SettingsKey(rest.to_string()))
        }
        ItemKind::Hook => Some(ItemTarget::HookEvent(rest.to_string())),
        ItemKind::Mcp => {
            let (scope, name) = rest.split_once(':')?;
            let scope = McpScope::from_key(scope)?;
            (!name.is_empty()).then(|| ItemTarget::McpServer { scope, name: name.to_string() })
        }
        ItemKind::Plugin => Some(ItemTarget::Plugin { id: rest.to_string() }),
        ItemKind::Skill | ItemKind::Team if rest.contains('/') => None,
        _ => {
            fsutil::safe_relative_path(rest).ok()?;
            Some(ItemTarget::Files { root: format!("{}/{rest}", kind.files_root()?) })
        }
    }
}

/// 스캔 대상 위치. 테스트에서는 임시 디렉토리를 넣는다.
#[derive(Debug, Clone)]
pub struct ClaudeEnv {
    pub claude_dir: PathBuf,
    pub claude_json: PathBuf,
    pub home: PathBuf,
}

impl ClaudeEnv {
    pub fn local() -> Self {
        Self {
            claude_dir: crate::paths::claude_dir(),
            claude_json: crate::paths::claude_json(),
            home: dirs::home_dir().unwrap_or_default(),
        }
    }

    pub fn settings_path(&self) -> PathBuf {
        self.claude_dir.join("settings.json")
    }

    pub fn path_context(&self) -> crate::portable::PathContext {
        crate::portable::PathContext::new(&self.claude_dir, &self.home)
    }

    pub fn read_settings(&self) -> Result<Value> {
        Ok(fsutil::read_json(&self.settings_path())?.unwrap_or_else(empty_object))
    }

    pub fn read_claude_json(&self) -> Result<Value> {
        Ok(fsutil::read_json(&self.claude_json)?.unwrap_or_else(empty_object))
    }

    pub fn known_marketplaces(&self) -> Value {
        fsutil::read_json(&self.claude_dir.join("plugins/known_marketplaces.json"))
            .ok()
            .flatten()
            .unwrap_or_else(empty_object)
    }
}

pub fn empty_object() -> Value {
    Value::Object(Default::default())
}

#[derive(Debug, Clone, Default)]
pub struct Inventory {
    pub items: Vec<Item>,
}

impl Inventory {
    pub fn get(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn ids(&self) -> Vec<String> {
        self.items.iter().map(|i| i.id.clone()).collect()
    }

    pub fn count(&self, kind: ItemKind) -> usize {
        self.items.iter().filter(|i| i.kind == kind).count()
    }
}

pub fn scan(env: &ClaudeEnv) -> Result<Inventory> {
    let mut items = Vec::new();
    let settings = env.read_settings()?;

    scan_single_file(env, ItemKind::ClaudeMd, "claude_md", "CLAUDE.md", &mut items);
    scan_settings_keys(&settings, &mut items);
    scan_hooks(&settings, &mut items);
    scan_mcp(env, &settings, &mut items)?;
    scan_plugins(env, &settings, &mut items);
    for kind in [ItemKind::Skill, ItemKind::Team] {
        scan_directory_items(env, kind, &mut items)?;
    }
    for kind in [
        ItemKind::Agent,
        ItemKind::Command,
        ItemKind::Rule,
        ItemKind::OutputStyle,
        ItemKind::HookScript,
        ItemKind::Memory,
    ] {
        scan_file_items(env, kind, &mut items)?;
    }
    scan_single_file(env, ItemKind::Keybindings, "keybindings", "keybindings.json", &mut items);

    items.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(Inventory { items })
}

fn scan_single_file(env: &ClaudeEnv, kind: ItemKind, id: &str, file: &str, items: &mut Vec<Item>) {
    let path = env.claude_dir.join(file);
    let Ok(metadata) = std::fs::metadata(&path) else {
        return;
    };
    if !metadata.is_file() {
        return;
    }
    items.push(Item {
        id: id.to_string(),
        kind,
        name: file.to_string(),
        description: None,
        size_bytes: metadata.len(),
        target: ItemTarget::Files { root: file.to_string() },
        files: vec![file.to_string()],
    });
}

fn scan_settings_keys(settings: &Value, items: &mut Vec<Item>) {
    let Some(map) = settings.as_object() else {
        return;
    };
    for (key, value) in map {
        if SETTINGS_KEYS_HANDLED_ELSEWHERE.contains(&key.as_str()) {
            continue;
        }
        items.push(Item {
            id: format!("setting:{key}"),
            kind: ItemKind::Setting,
            name: key.clone(),
            description: Some(summarize_json(value)),
            size_bytes: json_size(value),
            target: ItemTarget::SettingsKey(key.clone()),
            files: Vec::new(),
        });
    }
}

fn scan_hooks(settings: &Value, items: &mut Vec<Item>) {
    let Some(hooks) = settings.get("hooks").and_then(Value::as_object) else {
        return;
    };
    for (event, groups) in hooks {
        let commands = hook_commands(groups);
        let description = match commands.len() {
            0 => None,
            1 => Some(commands[0].clone()),
            n => Some(format!("{} 외 {}개", commands[0], n - 1)),
        };
        items.push(Item {
            id: format!("hook:{event}"),
            kind: ItemKind::Hook,
            name: event.clone(),
            description,
            size_bytes: json_size(groups),
            target: ItemTarget::HookEvent(event.clone()),
            files: Vec::new(),
        });
    }
}

/// 훅 이벤트 배열에서 실행 명령 줄만 뽑는다.
pub fn hook_commands(groups: &Value) -> Vec<String> {
    let mut commands = Vec::new();
    for group in groups.as_array().into_iter().flatten() {
        let matcher = group.get("matcher").and_then(Value::as_str).unwrap_or("*");
        for hook in group.get("hooks").and_then(Value::as_array).into_iter().flatten() {
            let line = command_line(hook)
                .or_else(|| hook.get("url").and_then(Value::as_str).map(|u| format!("POST {u}")))
                .or_else(|| hook.get("prompt").and_then(Value::as_str).map(|p| format!("prompt: {p}")));
            if let Some(line) = line {
                commands.push(format!("[{matcher}] {line}"));
            }
        }
    }
    commands
}

/// `{command, args}` 형태에서 실행될 명령 줄을 만든다.
pub fn command_line(value: &Value) -> Option<String> {
    let command = value.get("command").and_then(Value::as_str)?;
    let args: Vec<&str> = value
        .get("args")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    Some(std::iter::once(command).chain(args).collect::<Vec<_>>().join(" "))
}

pub fn mcp_description(server: &Value) -> String {
    command_line(server)
        .or_else(|| server.get("url").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| summarize_json(server))
}

fn scan_mcp(env: &ClaudeEnv, settings: &Value, items: &mut Vec<Item>) -> Result<()> {
    let claude_json = env.read_claude_json()?;
    for (scope, root) in [(McpScope::User, &claude_json), (McpScope::Settings, settings)] {
        let Some(servers) = root.get("mcpServers").and_then(Value::as_object) else {
            continue;
        };
        for (name, server) in servers {
            items.push(Item {
                id: format!("mcp:{}:{name}", scope.key()),
                kind: ItemKind::Mcp,
                name: match scope {
                    McpScope::User => name.clone(),
                    McpScope::Settings => format!("{name} (settings.json)"),
                },
                description: Some(truncate(&mcp_description(server), 160)),
                size_bytes: json_size(server),
                target: ItemTarget::McpServer { scope, name: name.clone() },
                files: Vec::new(),
            });
        }
    }
    Ok(())
}

fn scan_plugins(env: &ClaudeEnv, settings: &Value, items: &mut Vec<Item>) {
    let mut ids: Vec<String> = settings
        .get("enabledPlugins")
        .and_then(Value::as_object)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    let installed = fsutil::read_json(&env.claude_dir.join("plugins/installed_plugins.json"))
        .ok()
        .flatten();
    if let Some(plugins) = installed.as_ref().and_then(|v| v.get("plugins")).and_then(Value::as_object) {
        ids.extend(plugins.keys().cloned());
    }
    ids.sort();
    ids.dedup();

    for id in ids {
        let enabled = settings
            .pointer(&format!("/enabledPlugins/{}", id.replace('~', "~0").replace('/', "~1")))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let marketplace = id.split_once('@').map(|(_, m)| m).unwrap_or("");
        items.push(Item {
            id: format!("plugin:{id}"),
            kind: ItemKind::Plugin,
            name: id.clone(),
            description: Some(format!(
                "{} · 마켓플레이스 {}",
                if enabled { "활성" } else { "비활성" },
                if marketplace.is_empty() { "-" } else { marketplace }
            )),
            size_bytes: 0,
            target: ItemTarget::Plugin { id },
            files: Vec::new(),
        });
    }
}

/// 하위 디렉토리 하나가 항목 하나인 종류 (skills/<name>/, teams/<name>/)
fn scan_directory_items(env: &ClaudeEnv, kind: ItemKind, items: &mut Vec<Item>) -> Result<()> {
    let Some(root) = kind.files_root() else {
        return Ok(());
    };
    let dir = env.claude_dir.join(root);
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let item_root = format!("{root}/{name}");
        let files: Vec<(String, u64)> = fsutil::collect_files(&path)?;
        if files.is_empty() {
            continue;
        }
        let description = match kind {
            ItemKind::Skill => frontmatter_description(&path.join("SKILL.md")),
            _ => None,
        };
        items.push(Item {
            id: format!("{}:{name}", kind.key()),
            kind,
            name,
            description,
            size_bytes: files.iter().map(|(_, size)| size).sum(),
            files: files.into_iter().map(|(rel, _)| format!("{item_root}/{rel}")).collect(),
            target: ItemTarget::Files { root: item_root },
        });
    }
    Ok(())
}

/// 파일 하나가 항목 하나인 종류 (agents/x.md, commands/sub/y.md, ...)
fn scan_file_items(env: &ClaudeEnv, kind: ItemKind, items: &mut Vec<Item>) -> Result<()> {
    let Some(root) = kind.files_root() else {
        return Ok(());
    };
    for (relative, size) in fsutil::collect_files(&env.claude_dir.join(root))? {
        let is_hidden = relative.split('/').any(|segment| segment.starts_with('.'));
        if is_hidden {
            continue;
        }
        let path = format!("{root}/{relative}");
        let is_markdown = relative.ends_with(".md");
        let name = if is_markdown && kind != ItemKind::Memory {
            relative.trim_end_matches(".md").to_string()
        } else {
            relative.clone()
        };
        let description = if is_markdown {
            frontmatter_description(&env.claude_dir.join(&path))
        } else {
            None
        };
        items.push(Item {
            id: format!("{}:{relative}", kind.key()),
            kind,
            name,
            description,
            size_bytes: size,
            target: ItemTarget::Files { root: path.clone() },
            files: vec![path],
        });
    }
    Ok(())
}

/// YAML frontmatter의 `description:` 값을 읽는다. `>-`, `|` 같은 블록 값은 들여쓴 줄을 이어 붙인다.
fn frontmatter_description(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    while let Some(line) = lines.next() {
        if line.trim() == "---" {
            break;
        }
        let Some(rest) = line.strip_prefix("description:") else {
            continue;
        };
        let value = rest.trim();
        let is_block_scalar = value.starts_with('|') || value.starts_with('>');
        let text = if is_block_scalar {
            lines
                .take_while(|l| l.trim() != "---" && (l.starts_with(' ') || l.starts_with('\t') || l.trim().is_empty()))
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            value.trim_matches(|c| c == '"' || c == '\'').to_string()
        };
        return (!text.is_empty()).then(|| truncate(&text, 200));
    }
    None
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let head: String = s.chars().take(max_chars).collect();
    format!("{head}…")
}

pub fn summarize_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let keys: Vec<&str> = map.keys().take(4).map(String::as_str).collect();
            let more = if map.len() > 4 { format!(" 외 {}개", map.len() - 4) } else { String::new() };
            format!("{{{}}}{more}", keys.join(", "))
        }
        Value::Array(items) => format!("[{}개 항목]", items.len()),
        Value::String(s) => truncate(s, 80),
        other => other.to_string(),
    }
}

fn json_size(value: &Value) -> u64 {
    serde_json::to_vec(value).map(|v| v.len() as u64).unwrap_or(0)
}

#[cfg(test)]
pub mod test_support {
    use super::*;

    pub struct TempEnv {
        pub dir: PathBuf,
        pub env: ClaudeEnv,
    }

    impl TempEnv {
        pub fn new(label: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "claude-sync-test-{label}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let home = dir.join("home");
            let env = ClaudeEnv {
                claude_dir: home.join(".claude"),
                claude_json: home.join(".claude.json"),
                home,
            };
            std::fs::create_dir_all(&env.claude_dir).unwrap();
            Self { dir, env }
        }

        pub fn write(&self, relative: &str, content: &str) {
            let path = self.env.claude_dir.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }

        pub fn read(&self, relative: &str) -> String {
            std::fs::read_to_string(self.env.claude_dir.join(relative)).unwrap()
        }
    }

    impl Drop for TempEnv {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::TempEnv;
    use super::*;
    use serde_json::json;

    #[test]
    fn 여러_줄_블록_description은_이어_붙여_읽는다() {
        let t = TempEnv::new("block-desc");
        t.write("skills/guide/SKILL.md", "---\nname: guide\ndescription: >-\n  첫 줄\n  둘째 줄\nversion: 1\n---\nbody");

        let inventory = scan(&t.env).unwrap();

        assert_eq!(inventory.get("skill:guide").unwrap().description.as_deref(), Some("첫 줄 둘째 줄"));
    }

    #[test]
    fn 종류별로_항목을_찾고_id를_붙인다() {
        let t = TempEnv::new("scan");
        t.write("CLAUDE.md", "# hi");
        t.write("skills/review/SKILL.md", "---\nname: review\ndescription: 코드 리뷰\n---\nbody");
        t.write("skills/review/node_modules/x.js", "skip");
        t.write("agents/sub/planner.md", "---\ndescription: \"계획\"\n---\n");
        t.write("agents/.claude/settings.local.json", "{}");
        t.write("hooks/guard.mjs", "//");
        t.write(
            "settings.json",
            &json!({
                "model": "opus",
                "hooks": {"PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "node guard.mjs"}]}]},
                "enabledPlugins": {"p@market": true},
                "mcpServers": {"local": {"command": "x"}}
            })
            .to_string(),
        );
        std::fs::write(&t.env.claude_json, json!({"mcpServers": {"exa": {"command": "npx", "args": ["exa"]}}, "projects": {}}).to_string()).unwrap();

        let inventory = scan(&t.env).unwrap();

        assert_eq!(
            inventory.ids(),
            vec![
                "claude_md",
                "setting:model",
                "hook:PreToolUse",
                "hook_script:guard.mjs",
                "mcp:user:exa",
                "mcp:settings:local",
                "plugin:p@market",
                "skill:review",
                "agent:sub/planner.md",
            ]
        );
        let skill = inventory.get("skill:review").unwrap();
        assert_eq!(skill.description.as_deref(), Some("코드 리뷰"));
        assert_eq!(skill.file_count(), 1);
        assert_eq!(inventory.get("agent:sub/planner.md").unwrap().description.as_deref(), Some("계획"));
        assert_eq!(inventory.get("mcp:user:exa").unwrap().description.as_deref(), Some("npx exa"));
        assert!(inventory.get("hook:PreToolUse").unwrap().is_executable());
        assert!(!inventory.get("setting:model").unwrap().is_executable());
        assert!(is_executable(ItemKind::Setting, &ItemTarget::SettingsKey("env".into())));
    }
}
