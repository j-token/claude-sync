use serde_json::Value;

/// 머지 전략
#[derive(Debug, Clone, PartialEq)]
pub enum MergeStrategy {
    /// JSON 파일: 필드 레벨 딥 머지
    JsonDeepMerge,
    /// Markdown: 로컬 우선
    PreferLocal,
    /// Markdown: 원격 우선
    PreferRemote,
}

/// 머지 충돌 정보
#[derive(Debug, Clone)]
pub struct MergeConflict {
    pub field_path: String,
    pub local_value: String,
    pub remote_value: String,
}

/// 머지 결과
#[derive(Debug)]
pub struct MergeResult {
    pub merged: Value,
    pub conflicts: Vec<MergeConflict>,
}

/// JSON 필드 레벨 딥 머지
///
/// - base: 마지막 싱크 시점의 공통 조상
/// - local: 로컬 현재 값
/// - remote: 원격에서 가져온 값
///
/// 규칙:
/// - 한쪽만 변경: 변경된 값 채택
/// - 양쪽 모두 변경 + 같은 값: 그 값 채택
/// - 양쪽 모두 변경 + 다른 값: 충돌 (로컬 우선, 충돌 보고)
/// - 배열 (permissions.allow 등): 합집합 (union)
pub fn merge_json(base: &Value, local: &Value, remote: &Value) -> MergeResult {
    let mut conflicts = Vec::new();
    let merged = merge_value(base, local, remote, &[], &mut conflicts);
    MergeResult { merged, conflicts }
}

fn merge_value(
    base: &Value,
    local: &Value,
    remote: &Value,
    path: &[String],
    conflicts: &mut Vec<MergeConflict>,
) -> Value {
    match (local, remote) {
        // 양쪽 모두 Object
        (Value::Object(local_map), Value::Object(remote_map)) => {
            let base_map = if let Value::Object(m) = base {
                Some(m)
            } else {
                None
            };

            let mut result = serde_json::Map::new();
            let mut all_keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            all_keys.extend(local_map.keys().cloned());
            all_keys.extend(remote_map.keys().cloned());

            for key in all_keys {
                let mut current_path = path.to_vec();
                current_path.push(key.clone());

                let base_val = base_map.and_then(|m| m.get(&key)).unwrap_or(&Value::Null);
                let local_val = local_map.get(&key);
                let remote_val = remote_map.get(&key);

                match (local_val, remote_val) {
                    (Some(lv), Some(rv)) => {
                        // 양쪽 모두 존재
                        result.insert(
                            key,
                            merge_value(base_val, lv, rv, &current_path, conflicts),
                        );
                    }
                    (Some(lv), None) => {
                        // 로컬에만 존재 (원격에서 삭제됨)
                        if base_map.is_some_and(|m| m.contains_key(&key)) {
                            // base에 있었는데 원격에서 삭제됨 → 삭제 존중
                        } else {
                            // 로컬에서 새로 추가됨
                            result.insert(key, lv.clone());
                        }
                    }
                    (None, Some(rv)) => {
                        // 원격에만 존재 (로컬에서 삭제됨)
                        if base_map.is_some_and(|m| m.contains_key(&key)) {
                            // base에 있었는데 로컬에서 삭제됨 → 삭제 존중
                        } else {
                            // 원격에서 새로 추가됨
                            result.insert(key, rv.clone());
                        }
                    }
                    (None, None) => unreachable!(),
                }
            }

            Value::Object(result)
        }

        // 양쪽 모두 Array — 합집합 (permissions.allow 등)
        (Value::Array(local_arr), Value::Array(remote_arr)) => {
            let mut merged_arr = local_arr.clone();
            for item in remote_arr {
                if !merged_arr.contains(item) {
                    merged_arr.push(item.clone());
                }
            }
            Value::Array(merged_arr)
        }

        // 같은 값
        _ if local == remote => local.clone(),

        // 다른 값 — 충돌 (로컬 우선)
        _ => {
            let local_changed = local != base;
            let remote_changed = remote != base;

            match (local_changed, remote_changed) {
                (true, false) => local.clone(),   // 로컬만 변경
                (false, true) => remote.clone(),  // 원격만 변경
                (false, false) => local.clone(),  // 둘 다 안 변경 (base와 같음)
                (true, true) => {
                    // 양쪽 모두 변경 — 충돌, 로컬 우선
                    conflicts.push(MergeConflict {
                        field_path: path.join("."),
                        local_value: serde_json::to_string_pretty(local).unwrap_or_default(),
                        remote_value: serde_json::to_string_pretty(remote).unwrap_or_default(),
                    });
                    local.clone()
                }
            }
        }
    }
}

/// Markdown 파일 머지 (단순 전략)
pub fn merge_markdown(
    _base: &str,
    local: &str,
    remote: &str,
    strategy: MergeStrategy,
) -> String {
    match strategy {
        MergeStrategy::PreferLocal => local.to_string(),
        MergeStrategy::PreferRemote => remote.to_string(),
        MergeStrategy::JsonDeepMerge => local.to_string(), // Markdown에는 적용 불가
    }
}
