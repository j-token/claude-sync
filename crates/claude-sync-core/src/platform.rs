use serde_json::Value;

use crate::config::{Platform, PlatformPathRule};

/// 플랫폼별 경로를 감지하여 JSON에서 태깅
pub fn detect_platform_paths(value: &Value) -> Vec<String> {
    let mut platform_paths = Vec::new();
    walk_for_platform_paths(value, &[], &mut platform_paths);
    platform_paths
}

/// Windows 절대 경로 패턴 체크
fn is_windows_path(s: &str) -> bool {
    // C:\, D:\, /c/ 등의 패턴
    let s_trimmed = s.trim();
    (s_trimmed.len() >= 3 && s_trimmed.chars().nth(1) == Some(':') && s_trimmed.chars().nth(2) == Some('\\'))
        || (s_trimmed.len() >= 3 && s_trimmed.starts_with('/') && s_trimmed.chars().nth(2) == Some('/'))
}

/// macOS/Linux 절대 경로 패턴 체크 (홈 디렉토리 포함)
fn is_unix_absolute_path(s: &str) -> bool {
    let s_trimmed = s.trim();
    s_trimmed.starts_with("/Users/")
        || s_trimmed.starts_with("/home/")
        || s_trimmed.starts_with("/opt/")
        || s_trimmed.starts_with("/usr/")
}

/// 재귀적으로 JSON을 탐색하여 플랫폼별 경로 감지
fn walk_for_platform_paths(value: &Value, path: &[String], results: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                let mut current_path = path.to_vec();
                current_path.push(key.clone());

                if let Value::String(s) = val {
                    if is_windows_path(s) || is_unix_absolute_path(s) {
                        results.push(current_path.join("."));
                    }
                }

                walk_for_platform_paths(val, &current_path, results);
            }
        }
        Value::Array(arr) => {
            for (i, val) in arr.iter().enumerate() {
                let mut current_path = path.to_vec();
                current_path.push(i.to_string());

                if let Value::String(s) = val {
                    if is_windows_path(s) || is_unix_absolute_path(s) {
                        results.push(current_path.join("."));
                    }
                }

                walk_for_platform_paths(val, &current_path, results);
            }
        }
        _ => {}
    }
}

/// Push 시: 플랫폼별 규��에 따라 JSON 필드 처리
/// action이 "skip"인 필드는 다른 플랫폼에서 pull할 때 건드리지 않도록 태깅
pub fn should_skip_field(field_path: &str, rules: &[PlatformPathRule], current_platform: &Platform) -> bool {
    for rule in rules {
        if rule.field_path == field_path && rule.platform != *current_platform && rule.action == "skip" {
            return true;
        }
    }
    false
}
