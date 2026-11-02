use serde_json::Value;

use crate::config::{SecretAction, SecretPattern};

#[derive(Debug, Clone)]
pub struct SecretMatch {
    /// RFC 6901 JSON Pointer (e.g., "/env/EXA_API_KEY")
    pub pointer: String,
    pub pattern_name: String,
    pub original_value: String,
}

impl SecretMatch {
    /// 값 앞 4글자만 보여주는 미리보기. 멀티바이트 문자에서도 안전하다.
    pub fn preview(&self) -> String {
        let head: String = self.original_value.chars().take(4).collect();
        format!("{head}…")
    }
}

const KNOWN_SECRET_PREFIXES: &[&str] = &[
    "sk-", "hb_", "ctx7sk-", "ghp_", "gho_", "ghu_", "ghs_", "ghr_", "github_pat_", "sbp_",
    "sba_", "xoxb-", "xoxp-", "AKIA", "AIza", "eyJ",
];

pub struct SecretEngine {
    patterns: Vec<CompiledPattern>,
}

struct CompiledPattern {
    name: String,
    segments: Vec<SegmentMatcher>,
    action: SecretAction,
}

enum SegmentMatcher {
    Exact(String),
    Any,
    Suffix(String),
    Prefix(String),
}

impl SegmentMatcher {
    fn matches(&self, key: &str) -> bool {
        match self {
            SegmentMatcher::Exact(s) => key.eq_ignore_ascii_case(s),
            SegmentMatcher::Any => true,
            SegmentMatcher::Suffix(suffix) => key.to_ascii_uppercase().ends_with(&suffix.to_ascii_uppercase()),
            SegmentMatcher::Prefix(prefix) => key.to_ascii_uppercase().starts_with(&prefix.to_ascii_uppercase()),
        }
    }
}

impl SecretEngine {
    pub fn new(patterns: &[SecretPattern]) -> Self {
        let compiled = patterns
            .iter()
            .map(|p| CompiledPattern {
                name: p.name.clone(),
                segments: parse_pattern(&p.json_path),
                action: p.action.clone(),
            })
            .collect();
        Self { patterns: compiled }
    }

    pub fn detect(&self, value: &Value) -> Vec<SecretMatch> {
        let mut matches = Vec::new();
        self.walk(value, &mut Vec::new(), &mut matches);
        matches
    }

    /// 시크릿 값을 빈 문자열로 바꾼 사본과 바뀐 위치(JSON Pointer)를 돌려준다.
    pub fn mask(&self, value: &Value) -> (Value, Vec<SecretMatch>) {
        let matches = self.detect(value);
        let mut masked = value.clone();
        for m in &matches {
            if let Some(slot) = masked.pointer_mut(&m.pointer) {
                *slot = Value::String(String::new());
            }
        }
        (masked, matches)
    }

    fn walk(&self, value: &Value, path: &mut Vec<String>, matches: &mut Vec<SecretMatch>) {
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    path.push(key.clone());
                    if let Value::String(s) = child {
                        if let Some(pattern_name) = self.classify(path, s) {
                            matches.push(SecretMatch {
                                pointer: to_pointer(path),
                                pattern_name,
                                original_value: s.clone(),
                            });
                        }
                    } else {
                        self.walk(child, path, matches);
                    }
                    path.pop();
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    path.push(index.to_string());
                    self.walk(child, path, matches);
                    path.pop();
                }
            }
            _ => {}
        }
    }

    fn classify(&self, path: &[String], value: &str) -> Option<String> {
        if value.is_empty() {
            return None;
        }
        if let Some(pattern) = self
            .patterns
            .iter()
            .find(|p| matches_suffix(path, &p.segments))
        {
            return (pattern.action != SecretAction::Keep).then(|| pattern.name.clone());
        }
        is_likely_secret(value, path).then(|| "heuristic".to_string())
    }
}

/// 마스킹되었던 위치에 로컬의 기존 값을 되돌린다. 로컬에도 값이 없으면 그 위치를 반환한다.
pub fn restore_masked(incoming: &mut Value, masked: &[String], local: Option<&Value>) -> Vec<String> {
    let mut unresolved = Vec::new();
    for pointer in masked {
        let local_value = local
            .and_then(|l| l.pointer(pointer))
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty());
        match (local_value, incoming.pointer_mut(pointer)) {
            (Some(value), Some(slot)) => *slot = Value::String(value.to_string()),
            (None, Some(_)) => unresolved.push(pointer.clone()),
            _ => {}
        }
    }
    unresolved
}

fn parse_pattern(pattern: &str) -> Vec<SegmentMatcher> {
    pattern
        .split('.')
        .map(|seg| {
            if seg == "*" {
                SegmentMatcher::Any
            } else if let Some(suffix) = seg.strip_prefix('*') {
                SegmentMatcher::Suffix(suffix.to_string())
            } else if let Some(prefix) = seg.strip_suffix('*') {
                SegmentMatcher::Prefix(prefix.to_string())
            } else {
                SegmentMatcher::Exact(seg.to_string())
            }
        })
        .collect()
}

fn matches_suffix(path: &[String], segments: &[SegmentMatcher]) -> bool {
    if segments.is_empty() || path.len() < segments.len() {
        return false;
    }
    let tail = &path[path.len() - segments.len()..];
    tail.iter().zip(segments).all(|(key, seg)| seg.matches(key))
}

fn is_likely_secret(value: &str, path: &[String]) -> bool {
    let in_secret_context = path.iter().any(|p| p == "env" || p == "headers");
    if !in_secret_context {
        return false;
    }
    if KNOWN_SECRET_PREFIXES.iter().any(|prefix| value.starts_with(prefix)) {
        return true;
    }
    if url_has_credentials(value) {
        return true;
    }
    let looks_like_path_or_url = value.contains('/') || value.contains('\\') || value.contains(' ');
    value.len() >= 20 && !looks_like_path_or_url && shannon_entropy(value) > 3.5
}

/// `scheme://user:pass@host` 형태 (프록시 URL 등)
fn url_has_credentials(value: &str) -> bool {
    let Some((_, rest)) = value.split_once("://") else {
        return false;
    };
    let authority = rest.split('/').next().unwrap_or_default();
    authority.contains('@')
}

fn shannon_entropy(s: &str) -> f64 {
    let len = s.chars().count() as f64;
    if len == 0.0 {
        return 0.0;
    }
    let mut freq = std::collections::HashMap::new();
    for c in s.chars() {
        *freq.entry(c).or_insert(0u32) += 1;
    }
    freq.values().fold(0.0, |acc, &count| {
        let p = count as f64 / len;
        acc - p * p.log2()
    })
}

pub fn to_pointer(path: &[String]) -> String {
    path.iter()
        .map(|seg| format!("/{}", seg.replace('~', "~0").replace('/', "~1")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SyncConfig;
    use serde_json::json;

    fn engine() -> SecretEngine {
        SecretEngine::new(&SyncConfig::default_secret_patterns())
    }

    #[test]
    fn env_아래_api_key는_마스킹되고_원래_값은_복원된다() {
        let local = json!({"command": "npx", "env": {"EXA_API_KEY": "abc123", "MODE": "fast"}});

        let (mut masked, matches) = engine().mask(&local);
        let pointers: Vec<String> = matches.iter().map(|m| m.pointer.clone()).collect();

        assert_eq!(pointers, vec!["/env/EXA_API_KEY"]);
        assert_eq!(masked["env"]["EXA_API_KEY"], "");
        assert_eq!(masked["env"]["MODE"], "fast");

        let unresolved = restore_masked(&mut masked, &pointers, Some(&local));
        assert!(unresolved.is_empty());
        assert_eq!(masked["env"]["EXA_API_KEY"], "abc123");
    }

    #[test]
    fn 인증정보가_들어간_프록시_url은_시크릿으로_본다() {
        let settings = json!({"env": {"HTTPS_PROXY": "http://me:pw@proxy:8080", "NO_PROXY": "http://proxy:8080"}});

        let pointers: Vec<String> = engine().detect(&settings).into_iter().map(|m| m.pointer).collect();

        assert_eq!(pointers, vec!["/env/HTTPS_PROXY"]);
    }

    #[test]
    fn 로컬에_값이_없으면_복원되지_않은_위치를_알린다() {
        let mut incoming = json!({"env": {"EXA_API_KEY": ""}});

        let unresolved = restore_masked(&mut incoming, &["/env/EXA_API_KEY".to_string()], None);

        assert_eq!(unresolved, vec!["/env/EXA_API_KEY"]);
    }

    #[test]
    fn 멀티바이트_값의_미리보기에서_패닉하지_않는다() {
        let m = SecretMatch {
            pointer: "/env/X".into(),
            pattern_name: "p".into(),
            original_value: "한글비밀값입니다".into(),
        };

        assert_eq!(m.preview(), "한글비밀…");
    }
}
