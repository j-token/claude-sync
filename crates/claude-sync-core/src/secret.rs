use serde_json::Value;

use crate::config::{SecretAction, SecretPattern};

/// 시크릿 매칭 결과
#[derive(Debug, Clone)]
pub struct SecretMatch {
    /// JSON 내 경로 (e.g., "mcpServers.hyperbrowser.env.HYPERBROWSER_API_KEY")
    pub json_path: String,
    /// 매칭된 패턴 이름
    pub pattern_name: String,
    /// 원본 값 (마스킹 전)
    pub original_value: String,
}

/// 알려진 시크릿 값 접두사
const KNOWN_SECRET_PREFIXES: &[&str] = &[
    "sk-",
    "sk-ant-",
    "hb_",
    "ctx7sk-",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "ghr_",
    "sbp_",
    "sba_",
    "eyJ", // JWT token prefix (base64 of '{"')
];

/// 시크릿 탐지 엔진
pub struct SecretEngine {
    patterns: Vec<CompiledPattern>,
}

struct CompiledPattern {
    name: String,
    /// 각 세그먼트를 regex로 컴파일
    segments: Vec<SegmentMatcher>,
    action: SecretAction,
}

enum SegmentMatcher {
    /// 정확히 일치
    Exact(String),
    /// 와일드카드 (*)
    Any,
    /// 접미사 와일드카드 (e.g., *_API_KEY)
    Suffix(String),
    /// 접두사 와일드카드 (e.g., API_*)
    Prefix(String),
}

impl SecretEngine {
    pub fn new(patterns: &[SecretPattern]) -> Self {
        let compiled = patterns
            .iter()
            .map(|p| CompiledPattern {
                name: p.name.clone(),
                segments: parse_json_path(&p.json_path),
                action: p.action.clone(),
            })
            .collect();
        Self { patterns: compiled }
    }

    /// JSON 값에서 시크릿 탐지
    pub fn detect(&self, value: &Value) -> Vec<SecretMatch> {
        let mut matches = Vec::new();
        self.walk_json(value, &[], &mut matches);
        matches
    }

    /// JSON 값의 시크릿을 마스킹 (빈 문자열로 대체)
    pub fn mask(&self, value: &Value) -> (Value, Vec<SecretMatch>) {
        let matches = self.detect(value);
        let mut masked = value.clone();

        for m in &matches {
            set_json_value(&mut masked, &m.json_path, Value::String(String::new()));
        }

        (masked, matches)
    }

    /// 마스킹된 JSON에 로컬 시크릿 값 복원
    pub fn unmask(&self, masked: &Value, local: &Value) -> Value {
        let matches = self.detect(masked);
        let mut result = masked.clone();

        for m in &matches {
            // 로컬에 해당 경로의 값이 있으면 그 값을 사용
            if let Some(local_value) = get_json_value(local, &m.json_path) {
                if let Value::String(s) = local_value {
                    if !s.is_empty() {
                        set_json_value(&mut result, &m.json_path, local_value.clone());
                    }
                }
            }
        }

        result
    }

    /// 재귀적으로 JSON 트리를 탐색하며 시크릿 탐지
    fn walk_json(&self, value: &Value, path: &[String], matches: &mut Vec<SecretMatch>) {
        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    let mut current_path = path.to_vec();
                    current_path.push(key.clone());

                    // 패턴 매칭 체크
                    if let Value::String(s) = val {
                        if !s.is_empty() {
                            for pattern in &self.patterns {
                                if matches_pattern(&current_path, &pattern.segments) {
                                    if pattern.action == SecretAction::Mask {
                                        matches.push(SecretMatch {
                                            json_path: current_path.join("."),
                                            pattern_name: pattern.name.clone(),
                                            original_value: s.clone(),
                                        });
                                    }
                                    break;
                                }
                            }
                            // 보조 탐지: 패턴에 안 걸렸어도 알려진 접두사 or 고엔트로피
                            if !matches.iter().any(|m| m.json_path == current_path.join(".")) {
                                if is_likely_secret(s, &current_path) {
                                    matches.push(SecretMatch {
                                        json_path: current_path.join("."),
                                        pattern_name: "heuristic_detection".to_string(),
                                        original_value: s.clone(),
                                    });
                                }
                            }
                        }
                    }

                    // 재귀 탐색
                    self.walk_json(val, &current_path, matches);
                }
            }
            Value::Array(arr) => {
                for (i, val) in arr.iter().enumerate() {
                    let mut current_path = path.to_vec();
                    current_path.push(i.to_string());
                    self.walk_json(val, &current_path, matches);
                }
            }
            _ => {}
        }
    }
}

/// JSONPath 패턴 파싱 ("mcpServers.*.env.*_API_KEY" 등)
fn parse_json_path(path: &str) -> Vec<SegmentMatcher> {
    path.split('.')
        .map(|seg| {
            if seg == "*" {
                SegmentMatcher::Any
            } else if seg.starts_with('*') {
                SegmentMatcher::Suffix(seg[1..].to_string())
            } else if seg.ends_with('*') {
                SegmentMatcher::Prefix(seg[..seg.len() - 1].to_string())
            } else {
                SegmentMatcher::Exact(seg.to_string())
            }
        })
        .collect()
}

/// 실��� JSON 경로가 패턴과 매칭되는지 확인
fn matches_pattern(path: &[String], segments: &[SegmentMatcher]) -> bool {
    if path.len() != segments.len() {
        return false;
    }

    path.iter().zip(segments.iter()).all(|(key, seg)| match seg {
        SegmentMatcher::Exact(s) => key == s,
        SegmentMatcher::Any => true,
        SegmentMatcher::Suffix(suffix) => key.ends_with(suffix),
        SegmentMatcher::Prefix(prefix) => key.starts_with(prefix),
    })
}

/// 휴리스틱 시크릿 탐지
fn is_likely_secret(value: &str, path: &[String]) -> bool {
    // env 또는 headers 컨텍스트에서만 휴리스틱 적용
    let in_secret_context = path.iter().any(|p| p == "env" || p == "headers");
    if !in_secret_context {
        return false;
    }

    // 알려진 접두사 체크
    if KNOWN_SECRET_PREFIXES
        .iter()
        .any(|prefix| value.starts_with(prefix))
    {
        return true;
    }

    // 고엔트로피 문자열 체크 (20자 이상, 엔트로피 > 3.5)
    if value.len() >= 20 {
        let entropy = shannon_entropy(value);
        if entropy > 3.5 {
            return true;
        }
    }

    false
}

/// Shannon 엔트로피 계산
fn shannon_entropy(s: &str) -> f64 {
    let len = s.len() as f64;
    if len == 0.0 {
        return 0.0;
    }

    let mut freq = std::collections::HashMap::new();
    for c in s.chars() {
        *freq.entry(c).or_insert(0u32) += 1;
    }

    freq.values().fold(0.0f64, |acc, &count| {
        let p = count as f64 / len;
        acc - p * p.log2()
    })
}

/// JSON 경로("a.b.c")를 따라 값 가져오기
fn get_json_value<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = root;

    for part in &parts {
        match current {
            Value::Object(map) => {
                current = map.get(*part)?;
            }
            Value::Array(arr) => {
                let idx: usize = part.parse().ok()?;
                current = arr.get(idx)?;
            }
            _ => return None,
        }
    }

    Some(current)
}

/// JSON 경로("a.b.c")를 따라 값 설정
fn set_json_value(root: &mut Value, path: &str, new_value: Value) {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = root;

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // 마지막 세그먼트: 값 설정
            if let Value::Object(map) = current {
                map.insert(part.to_string(), new_value);
                return;
            }
        } else {
            // 중간 세그먼트: 하위 탐색
            match current {
                Value::Object(map) => {
                    current = map.get_mut(*part).unwrap();
                }
                Value::Array(arr) => {
                    let idx: usize = part.parse().unwrap();
                    current = arr.get_mut(idx).unwrap();
                }
                _ => return,
            }
        }
    }
}
