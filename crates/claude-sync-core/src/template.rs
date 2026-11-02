//! 템플릿: 항목 id glob 패턴으로 정의한 이름 있는 선택 집합.

use serde::{Deserialize, Serialize};

use crate::error::{Result, SyncError};
use crate::fsutil;
use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(skip)]
    pub builtin: bool,
}

impl Template {
    pub fn matches(&self, id: &str) -> bool {
        let included = self.include.iter().any(|p| glob_match(p, id));
        let excluded = self.exclude.iter().any(|p| glob_match(p, id));
        included && !excluded
    }

    pub fn select<'a>(&self, ids: impl IntoIterator<Item = &'a String>) -> Vec<String> {
        ids.into_iter().filter(|id| self.matches(id)).cloned().collect()
    }
}

/// `*`는 `/`, `:`를 포함한 임의 문자열, `?`는 임의 한 글자와 일치한다.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    glob::Pattern::new(pattern)
        .map(|p| p.matches(text))
        .unwrap_or_else(|_| pattern == text)
}

fn builtin(name: &str, description: &str, include: &[&str], exclude: &[&str]) -> Template {
    Template {
        name: name.to_string(),
        description: description.to_string(),
        include: include.iter().map(|s| s.to_string()).collect(),
        exclude: exclude.iter().map(|s| s.to_string()).collect(),
        builtin: true,
    }
}

pub fn builtin_templates() -> Vec<Template> {
    vec![
        builtin("full", "모든 항목", &["*"], &[]),
        builtin(
            "essentials",
            "CLAUDE.md, 설정, 훅, 키바인딩",
            &["claude_md", "setting:*", "hook:*", "hook_script:*", "keybindings"],
            &[],
        ),
        builtin(
            "skills-agents",
            "스킬, 에이전트, 커맨드, 출력 스타일",
            &["skill:*", "agent:*", "command:*", "output_style:*"],
            &[],
        ),
        builtin("integrations", "MCP 서버와 플러그인", &["mcp:*", "plugin:*"], &[]),
        builtin(
            "safe-share",
            "남에게 공유하기 안전한 항목 (명령을 실행하는 항목 제외)",
            &["claude_md", "skill:*", "agent:*", "command:*", "rule:*", "output_style:*"],
            &[],
        ),
    ]
}

pub fn list() -> Result<Vec<Template>> {
    let mut templates = builtin_templates();
    let dir = paths::templates_dir();
    if dir.is_dir() {
        let mut user_templates = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "json") {
                let content = std::fs::read_to_string(&path)?;
                match serde_json::from_str::<Template>(&content) {
                    Ok(template) => user_templates.push(template),
                    Err(e) => tracing::warn!("템플릿 {} 무시: {e}", path.display()),
                }
            }
        }
        user_templates.sort_by(|a, b| a.name.cmp(&b.name));
        templates.extend(user_templates.into_iter().filter(|t| !is_builtin_name(&t.name)));
    }
    Ok(templates)
}

pub fn find(name: &str) -> Result<Template> {
    list()?
        .into_iter()
        .find(|t| t.name == name)
        .ok_or_else(|| SyncError::NotFound(format!("템플릿 '{name}'")))
}

fn is_builtin_name(name: &str) -> bool {
    builtin_templates().iter().any(|t| t.name == name)
}

fn file_for(name: &str) -> Result<std::path::PathBuf> {
    let valid = !name.trim().is_empty()
        && name.chars().count() <= 64
        && name.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ' ' | '.'))
        && !name.starts_with('.');
    if !valid {
        return Err(SyncError::Template(
            "이름은 1~64자의 글자, 숫자, 공백, '-', '_', '.'만 쓸 수 있습니다".into(),
        ));
    }
    Ok(paths::templates_dir().join(format!("{}.json", name.trim())))
}

pub fn save(template: &Template) -> Result<Template> {
    if is_builtin_name(&template.name) {
        return Err(SyncError::Template(format!("'{}'은 기본 템플릿 이름이라 쓸 수 없습니다", template.name)));
    }
    if template.include.iter().all(|p| p.trim().is_empty()) {
        return Err(SyncError::Template("포함 패턴을 하나 이상 지정하세요".into()));
    }
    let clean = |patterns: &[String]| -> Vec<String> {
        patterns.iter().map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect()
    };
    let saved = Template {
        name: template.name.trim().to_string(),
        description: template.description.trim().to_string(),
        include: clean(&template.include),
        exclude: clean(&template.exclude),
        builtin: false,
    };
    let path = file_for(&saved.name)?;
    fsutil::atomic_write(&path, serde_json::to_string_pretty(&saved)?.as_bytes())?;
    Ok(saved)
}

pub fn delete(name: &str) -> Result<()> {
    if is_builtin_name(name) {
        return Err(SyncError::Template("기본 템플릿은 삭제할 수 없습니다".into()));
    }
    let path = file_for(name)?;
    if !path.exists() {
        return Err(SyncError::NotFound(format!("템플릿 '{name}'")));
    }
    std::fs::remove_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 포함_패턴에_맞고_제외_패턴에_안_맞는_id만_고른다() {
        let template = Template {
            name: "t".into(),
            description: String::new(),
            include: vec!["skill:*".into(), "mcp:user:*".into(), "claude_md".into()],
            exclude: vec!["skill:old-*".into()],
            builtin: false,
        };
        let ids: Vec<String> = ["skill:review", "skill:old-x", "mcp:user:exa", "mcp:settings:y", "claude_md", "agent:a.md"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let selected = template.select(&ids);

        assert_eq!(selected, vec!["skill:review", "mcp:user:exa", "claude_md"]);
    }

    #[test]
    fn 별표는_슬래시가_들어간_id에도_일치한다() {
        assert!(glob_match("agent:*", "agent:sub/planner.md"));
    }
}
