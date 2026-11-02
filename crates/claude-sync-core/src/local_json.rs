//! settings.json과 .claude.json 안에 있는 JSON 조각 항목(설정 키, 훅, MCP 서버, 플러그인)을 읽고 쓴다.

use serde_json::{Map, Value, json};

use crate::error::{Result, SyncError};
use crate::fsutil;
use crate::inventory::{ClaudeEnv, ItemTarget, McpScope, empty_object};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyMode {
    /// 객체는 키 단위로 합치고 배열은 합집합, 값은 들어오는 쪽을 쓴다.
    Merge,
    Overwrite,
}

pub struct LocalJson {
    settings: Value,
    claude_json: Value,
    known_marketplaces: Value,
    installed_plugins: Vec<String>,
    settings_dirty: bool,
    claude_json_dirty: bool,
}

impl LocalJson {
    pub fn load(env: &ClaudeEnv) -> Result<Self> {
        let installed_plugins = fsutil::read_json(&env.claude_dir.join("plugins/installed_plugins.json"))
            .ok()
            .flatten()
            .and_then(|v| v.get("plugins").and_then(Value::as_object).map(|m| m.keys().cloned().collect()))
            .unwrap_or_default();
        Ok(Self {
            settings: env.read_settings()?,
            claude_json: env.read_claude_json()?,
            known_marketplaces: env.known_marketplaces(),
            installed_plugins,
            settings_dirty: false,
            claude_json_dirty: false,
        })
    }

    /// 항목의 현재 로컬 값을 `{이름: 값}` 형태로 돌려준다. 번들 payload와 같은 모양이다.
    pub fn payload(&self, target: &ItemTarget) -> Option<Value> {
        match target {
            ItemTarget::SettingsKey(key) => wrap(key, self.settings.get(key)?),
            ItemTarget::HookEvent(event) => wrap(event, self.settings.get("hooks")?.get(event)?),
            ItemTarget::McpServer { scope, name } => wrap(name, self.mcp_root(*scope).get("mcpServers")?.get(name)?),
            ItemTarget::Plugin { id } => self.plugin_payload(id),
            ItemTarget::Files { .. } => None,
        }
    }

    fn mcp_root(&self, scope: McpScope) -> &Value {
        match scope {
            McpScope::User => &self.claude_json,
            McpScope::Settings => &self.settings,
        }
    }

    fn plugin_payload(&self, id: &str) -> Option<Value> {
        let enabled = self.settings.get("enabledPlugins").and_then(|m| m.get(id));
        if enabled.is_none() && !self.installed_plugins.iter().any(|p| p == id) {
            return None;
        }
        let marketplace = id.split_once('@').map(|(_, m)| m.to_string()).unwrap_or_default();
        let marketplace_source = self
            .settings
            .get("extraKnownMarketplaces")
            .and_then(|m| m.get(&marketplace))
            .or_else(|| self.known_marketplaces.get(&marketplace))
            .and_then(|m| m.get("source"))
            .cloned()
            .unwrap_or(Value::Null);
        Some(json!({
            "id": id,
            "enabled": enabled.and_then(Value::as_bool).unwrap_or(false),
            "marketplace": marketplace,
            "marketplace_source": marketplace_source,
        }))
    }

    pub fn apply(&mut self, target: &ItemTarget, incoming: &Value, mode: ApplyMode) -> Result<()> {
        match target {
            ItemTarget::SettingsKey(key) => {
                let value = unwrap(incoming, key)?;
                let slot = object_mut(&mut self.settings)?.entry(key.clone()).or_insert(Value::Null);
                *slot = match mode {
                    ApplyMode::Merge => deep_merge(slot, value),
                    ApplyMode::Overwrite => value.clone(),
                };
                self.settings_dirty = true;
            }
            ItemTarget::HookEvent(event) => {
                let groups = unwrap(incoming, event)?;
                let hooks = child_object(&mut self.settings, "hooks")?;
                let slot = hooks.entry(event.clone()).or_insert_with(|| Value::Array(Vec::new()));
                *slot = match mode {
                    ApplyMode::Merge => union_arrays(slot, groups),
                    ApplyMode::Overwrite => groups.clone(),
                };
                self.settings_dirty = true;
            }
            ItemTarget::McpServer { scope, name } => {
                let server = unwrap(incoming, name)?.clone();
                let root = match scope {
                    McpScope::User => {
                        self.claude_json_dirty = true;
                        &mut self.claude_json
                    }
                    McpScope::Settings => {
                        self.settings_dirty = true;
                        &mut self.settings
                    }
                };
                // MCP 서버는 args 배열이 합쳐지면 다른 명령이 되므로 항상 통째로 바꾼다.
                child_object(root, "mcpServers")?.insert(name.clone(), server);
            }
            ItemTarget::Plugin { id } => self.apply_plugin(id, incoming)?,
            ItemTarget::Files { .. } => {
                return Err(SyncError::Bundle("파일 항목은 JSON으로 적용할 수 없습니다".into()));
            }
        }
        Ok(())
    }

    fn apply_plugin(&mut self, id: &str, incoming: &Value) -> Result<()> {
        let enabled = incoming.get("enabled").and_then(Value::as_bool).unwrap_or(true);
        let marketplace = incoming.get("marketplace").and_then(Value::as_str).unwrap_or_default().to_string();
        let source = incoming.get("marketplace_source").filter(|s| s.is_object()).cloned();

        child_object(&mut self.settings, "enabledPlugins")?.insert(id.to_string(), Value::Bool(enabled));

        let already_known = self.known_marketplaces.get(&marketplace).is_some()
            || self.settings.get("extraKnownMarketplaces").and_then(|m| m.get(&marketplace)).is_some();
        if let (false, false, Some(source)) = (already_known, marketplace.is_empty(), source) {
            child_object(&mut self.settings, "extraKnownMarketplaces")?
                .insert(marketplace, json!({ "source": source }));
        }
        self.settings_dirty = true;
        Ok(())
    }

    /// 바뀐 파일만 쓴다. `.claude.json`은 실행 중인 Claude Code도 계속 고쳐 쓰므로
    /// 쓰기 직전에 다시 읽어 `mcpServers`만 교체한다.
    pub fn save(&self, env: &ClaudeEnv) -> Result<()> {
        if self.settings_dirty {
            fsutil::write_json(&env.settings_path(), &self.settings)?;
        }
        if self.claude_json_dirty {
            let mut fresh = env.read_claude_json()?;
            let servers = self.claude_json.get("mcpServers").cloned().unwrap_or_else(empty_object);
            object_mut(&mut fresh)?.insert("mcpServers".into(), servers);
            fsutil::write_json(&env.claude_json, &fresh)?;
        }
        Ok(())
    }
}

fn wrap(key: &str, value: &Value) -> Option<Value> {
    let mut map = Map::new();
    map.insert(key.to_string(), value.clone());
    Some(Value::Object(map))
}

fn unwrap<'a>(payload: &'a Value, key: &str) -> Result<&'a Value> {
    payload
        .get(key)
        .ok_or_else(|| SyncError::Bundle(format!("payload에 '{key}' 값이 없습니다")))
}

fn object_mut(value: &mut Value) -> Result<&mut Map<String, Value>> {
    if value.is_null() {
        *value = empty_object();
    }
    value
        .as_object_mut()
        .ok_or_else(|| SyncError::Config("JSON 최상위가 객체가 아닙니다".into()))
}

fn child_object<'a>(root: &'a mut Value, key: &str) -> Result<&'a mut Map<String, Value>> {
    let slot = object_mut(root)?.entry(key.to_string()).or_insert_with(empty_object);
    object_mut(slot)
}

pub fn deep_merge(local: &Value, incoming: &Value) -> Value {
    match (local, incoming) {
        (Value::Object(local_map), Value::Object(incoming_map)) => {
            let mut merged = local_map.clone();
            for (key, value) in incoming_map {
                let next = match local_map.get(key) {
                    Some(existing) => deep_merge(existing, value),
                    None => value.clone(),
                };
                merged.insert(key.clone(), next);
            }
            Value::Object(merged)
        }
        (Value::Array(_), Value::Array(_)) => union_arrays(local, incoming),
        _ => incoming.clone(),
    }
}

fn union_arrays(local: &Value, incoming: &Value) -> Value {
    let mut merged = local.as_array().cloned().unwrap_or_default();
    for item in incoming.as_array().into_iter().flatten() {
        if !merged.contains(item) {
            merged.push(item.clone());
        }
    }
    Value::Array(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 병합은_객체를_합치고_배열은_중복없이_이어붙인다() {
        let local = json!({"allow": ["Bash(ls)"], "defaultMode": "plan", "keep": 1});
        let incoming = json!({"allow": ["Bash(ls)", "Read"], "defaultMode": "auto"});

        let merged = deep_merge(&local, &incoming);

        assert_eq!(merged, json!({"allow": ["Bash(ls)", "Read"], "defaultMode": "auto", "keep": 1}));
    }
}
