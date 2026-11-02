//! 기기마다 다른 절대 경로를 번들 안에서는 토큰으로 바꿔 둔다.

use std::path::Path;

use serde_json::Value;

pub const CLAUDE_DIR_TOKEN: &str = "{{CLAUDE_DIR}}";
pub const HOME_TOKEN: &str = "{{HOME}}";

pub struct PathContext {
    claude_dir: String,
    home: String,
}

impl PathContext {
    pub fn new(claude_dir: &Path, home: &Path) -> Self {
        Self {
            claude_dir: forward_slashes(&claude_dir.to_string_lossy()),
            home: forward_slashes(&home.to_string_lossy()),
        }
    }

    pub fn local() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        Self::new(&crate::paths::claude_dir(), &home)
    }

    pub fn tokenize(&self, value: &Value) -> Value {
        map_strings(value, &|s| self.tokenize_str(s))
    }

    pub fn detokenize(&self, value: &Value) -> Value {
        map_strings(value, &|s| {
            s.replace(CLAUDE_DIR_TOKEN, &self.claude_dir)
                .replace(HOME_TOKEN, &self.home)
        })
    }

    fn tokenize_str(&self, s: &str) -> String {
        let mut out = s.to_string();
        // claude_dir는 보통 home 아래에 있으므로 더 긴 쪽을 먼저 바꾼다.
        for (prefix, token) in [(&self.claude_dir, CLAUDE_DIR_TOKEN), (&self.home, HOME_TOKEN)] {
            if prefix.len() < 3 {
                continue;
            }
            out = replace_ignoring_separator(&out, prefix, token);
        }
        out
    }

    /// 문자열 안에서 로컬 home 아래를 가리키는 경로 중 실제로 없는 것을 찾는다.
    pub fn missing_local_paths(&self, value: &Value) -> Vec<String> {
        let mut missing = Vec::new();
        visit_strings(value, &mut |s| {
            for candidate in path_candidates(s, &self.home) {
                if !Path::new(&candidate).exists() && !missing.contains(&candidate) {
                    missing.push(candidate);
                }
            }
        });
        missing
    }
}

fn forward_slashes(s: &str) -> String {
    s.replace('\\', "/").trim_end_matches('/').to_string()
}

/// `prefix`(슬래시 형태)를 `/`와 `\` 어느 쪽 구분자로 쓰였든 찾아 토큰으로 바꾼다.
/// Windows 경로는 대소문자를 구분하지 않으므로 ASCII 대소문자도 무시한다.
fn replace_ignoring_separator(haystack: &str, prefix: &str, token: &str) -> String {
    let normalized: String = haystack
        .chars()
        .map(|c| if c == '\\' { '/' } else { c.to_ascii_lowercase() })
        .collect();
    let needle = prefix.to_ascii_lowercase();
    if !normalized.contains(&needle) || normalized.len() != haystack.len() {
        return haystack.to_string();
    }

    let mut out = String::with_capacity(haystack.len());
    let mut cursor = 0;
    while let Some(found) = normalized[cursor..].find(&needle) {
        let start = cursor + found;
        let end = start + needle.len();
        let at_boundary = normalized[end..]
            .chars()
            .next()
            .is_none_or(|c| matches!(c, '/' | '"' | '\'' | ' '));
        out.push_str(&haystack[cursor..start]);
        if at_boundary {
            out.push_str(token);
        } else {
            out.push_str(&haystack[start..end]);
        }
        cursor = end;
    }
    out.push_str(&haystack[cursor..]);
    out
}

fn path_candidates(s: &str, home: &str) -> Vec<String> {
    let home_lower = home.to_ascii_lowercase();
    s.split(|c: char| c.is_whitespace() || c == '"' || c == '\'')
        .map(|part| part.replace('\\', "/"))
        .filter(|part| part.to_ascii_lowercase().starts_with(&home_lower) && part.len() > home.len())
        .collect()
}

fn map_strings(value: &Value, f: &dyn Fn(&str) -> String) -> Value {
    match value {
        Value::String(s) => Value::String(f(s)),
        Value::Array(items) => Value::Array(items.iter().map(|v| map_strings(v, f)).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), map_strings(v, f)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn visit_strings(value: &Value, f: &mut dyn FnMut(&str)) {
    match value {
        Value::String(s) => f(s),
        Value::Array(items) => items.iter().for_each(|v| visit_strings(v, f)),
        Value::Object(map) => map.values().for_each(|v| visit_strings(v, f)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn 내보낼_때_경로를_토큰으로_바꾸고_가져올_때_로컬_경로로_되돌린다() {
        let source = PathContext::new(Path::new("C:\\Users\\a\\.claude"), Path::new("C:\\Users\\a"));
        let target = PathContext::new(Path::new("/home/b/.claude"), Path::new("/home/b"));
        let hook = json!({"command": "node C:/Users/a/.claude/hooks/x.mjs", "cwd": "C:\\Users\\a\\work"});

        let portable = source.tokenize(&hook);

        assert_eq!(portable["command"], "node {{CLAUDE_DIR}}/hooks/x.mjs");
        assert_eq!(portable["cwd"], "{{HOME}}\\work");
        assert_eq!(target.detokenize(&portable)["command"], "node /home/b/.claude/hooks/x.mjs");
    }

    #[test]
    fn 이름이_겹치는_다른_디렉토리는_바꾸지_않는다() {
        let ctx = PathContext::new(Path::new("/home/a/.claude"), Path::new("/home/a"));

        let out = ctx.tokenize(&json!("/home/abc/file"));

        assert_eq!(out, "/home/abc/file");
    }
}
