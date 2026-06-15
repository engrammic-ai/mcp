use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{json, Map, Value};
use serde_yaml::Value as YamlValue;

use crate::tools::{ConfigShape, TypeField};

const MCP_SERVER_KEY: &str = "engrammic";
const CODEX_SERVER_TABLE: &str = "mcp_servers";

pub enum InstallResult {
    Created,
    Updated { old_url: String },
    Unchanged,
}

// ---------------------------------------------------------------------------
// Public dispatchers
// ---------------------------------------------------------------------------

pub fn install(config_path: &Path, endpoint: &str, shape: ConfigShape) -> Result<InstallResult> {
    match shape {
        ConfigShape::JsonMap {
            container,
            type_field,
            url_field,
        } => install_json_map(config_path, endpoint, container, type_field, url_field),
        ConfigShape::CodexToml => install_toml(config_path, endpoint),
        ConfigShape::GooseYaml => install_goose_yaml(config_path, endpoint),
        ConfigShape::OpenCodeJson => install_opencode_json(config_path, endpoint),
        ConfigShape::ContinueYaml => install_continue_yaml(config_path, endpoint),
        ConfigShape::HermesYaml => install_hermes_yaml(config_path, endpoint),
    }
}

pub fn uninstall(config_path: &Path, shape: ConfigShape) -> Result<()> {
    match shape {
        ConfigShape::JsonMap { container, .. } => uninstall_json_map(config_path, container),
        ConfigShape::CodexToml => uninstall_toml(config_path),
        ConfigShape::GooseYaml => uninstall_goose_yaml(config_path),
        ConfigShape::OpenCodeJson => uninstall_json_map(config_path, "mcp"),
        ConfigShape::ContinueYaml => uninstall_continue_yaml(config_path),
        ConfigShape::HermesYaml => uninstall_hermes_yaml(config_path),
    }
}

pub fn get_installed_endpoint(config_path: &Path, shape: ConfigShape) -> Option<String> {
    match shape {
        ConfigShape::JsonMap {
            container,
            url_field,
            ..
        } => get_endpoint_json_map(config_path, container, url_field),
        ConfigShape::CodexToml => get_endpoint_toml(config_path),
        ConfigShape::GooseYaml => get_endpoint_goose_yaml(config_path),
        ConfigShape::OpenCodeJson => get_endpoint_json_map(config_path, "mcp", "url"),
        ConfigShape::ContinueYaml => get_endpoint_continue_yaml(config_path),
        ConfigShape::HermesYaml => get_endpoint_hermes_yaml(config_path),
    }
}

// ---------------------------------------------------------------------------
// Backup helper
// ---------------------------------------------------------------------------

/// Create `<path>.engrammic.bak` before our first mutation of a harness config.
/// Idempotent: an existing backup is never overwritten, so it always preserves
/// the pre-Engrammic state. Returns None when there is nothing to back up.
pub fn ensure_backup(config_path: &Path) -> Result<Option<std::path::PathBuf>> {
    if !config_path.exists() {
        return Ok(None);
    }
    let mut bak = config_path.as_os_str().to_owned();
    bak.push(".engrammic.bak");
    let bak = std::path::PathBuf::from(bak);
    if !bak.exists() {
        fs::copy(config_path, &bak).with_context(|| {
            format!(
                "failed to back up {} to {}",
                config_path.display(),
                bak.display()
            )
        })?;
    }
    Ok(Some(bak))
}

// ---------------------------------------------------------------------------
// JSON map shape (`{ "<container>": { "engrammic": { ... } } }`)
// ---------------------------------------------------------------------------

fn read_json(config_path: &Path) -> Result<Value> {
    if !config_path.exists() {
        return Ok(json!({}));
    }
    let content = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    if content.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(&content)
        .with_context(|| format!("failed to parse JSON in {}", config_path.display()))
}

fn write_json(config_path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create directories for {}", config_path.display())
        })?;
    }
    let content = serde_json::to_string_pretty(value).context("failed to serialize JSON")?;
    fs::write(config_path, content)
        .with_context(|| format!("failed to write {}", config_path.display()))
}

/// Build a single server entry, e.g. `{ "type": "http", "url": "..." }`.
/// `type` is omitted when `type_field` is `None`; the URL key name is `url_field`.
fn build_json_entry(endpoint: &str, type_field: TypeField, url_field: &str) -> Value {
    let mut map = Map::new();
    if let Some(t) = type_field.value() {
        map.insert("type".to_string(), json!(t));
    }
    map.insert(url_field.to_string(), json!(endpoint));
    Value::Object(map)
}

fn install_json_map(
    config_path: &Path,
    endpoint: &str,
    container: &str,
    type_field: TypeField,
    url_field: &str,
) -> Result<InstallResult> {
    let mut root = read_json(config_path)?;

    let servers = root
        .as_object_mut()
        .context("config root is not a JSON object")?
        .entry(container)
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .with_context(|| format!("{container} is not a JSON object"))?;

    let old_url = servers
        .get(MCP_SERVER_KEY)
        .and_then(|v| v.get(url_field))
        .and_then(|v| v.as_str())
        .map(String::from);

    let result = match old_url {
        Some(ref url) if url == endpoint => InstallResult::Unchanged,
        Some(url) => InstallResult::Updated { old_url: url },
        None => InstallResult::Created,
    };

    if !matches!(result, InstallResult::Unchanged) {
        servers.insert(
            MCP_SERVER_KEY.to_string(),
            build_json_entry(endpoint, type_field, url_field),
        );
        write_json(config_path, &root)?;
    }

    Ok(result)
}

fn uninstall_json_map(config_path: &Path, container: &str) -> Result<()> {
    let mut root = read_json(config_path)?;

    if let Some(servers) = root
        .as_object_mut()
        .and_then(|o| o.get_mut(container))
        .and_then(|v| v.as_object_mut())
    {
        servers.remove(MCP_SERVER_KEY);
    }

    write_json(config_path, &root)
}

fn get_endpoint_json_map(config_path: &Path, container: &str, url_field: &str) -> Option<String> {
    let root = read_json(config_path).ok()?;
    root.get(container)?
        .get(MCP_SERVER_KEY)?
        .get(url_field)?
        .as_str()
        .map(String::from)
}

// ---------------------------------------------------------------------------
// Codex TOML shape (`[mcp_servers.engrammic]` with a `url` key)
//
// Uses `toml_edit` (not `toml::Value`) so the user's comments, key ordering, and
// unrelated tables in `config.toml` survive a round-trip.
// ---------------------------------------------------------------------------

fn read_toml_doc(config_path: &Path) -> Result<toml_edit::DocumentMut> {
    if !config_path.exists() {
        return Ok(toml_edit::DocumentMut::new());
    }
    let content = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    if content.trim().is_empty() {
        return Ok(toml_edit::DocumentMut::new());
    }
    content
        .parse::<toml_edit::DocumentMut>()
        .with_context(|| format!("failed to parse TOML in {}", config_path.display()))
}

fn write_toml_doc(config_path: &Path, doc: &toml_edit::DocumentMut) -> Result<()> {
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create directories for {}", config_path.display())
        })?;
    }
    fs::write(config_path, doc.to_string())
        .with_context(|| format!("failed to write {}", config_path.display()))
}

fn install_toml(config_path: &Path, endpoint: &str) -> Result<InstallResult> {
    use toml_edit::{value, Item, Table};

    let mut doc = read_toml_doc(config_path)?;

    // Ensure the `[mcp_servers]` parent table exists. Mark it implicit so we render
    // only `[mcp_servers.engrammic]`, not a bare `[mcp_servers]` header.
    if doc.get(CODEX_SERVER_TABLE).is_none() {
        let mut table = Table::new();
        table.set_implicit(true);
        doc[CODEX_SERVER_TABLE] = Item::Table(table);
    }

    let servers = doc[CODEX_SERVER_TABLE]
        .as_table_mut()
        .with_context(|| format!("{CODEX_SERVER_TABLE} is not a TOML table"))?;

    let old_url = servers
        .get(MCP_SERVER_KEY)
        .and_then(|i| i.get("url"))
        .and_then(|i| i.as_str())
        .map(String::from);

    let result = match old_url {
        Some(ref url) if url == endpoint => InstallResult::Unchanged,
        Some(url) => InstallResult::Updated { old_url: url },
        None => InstallResult::Created,
    };

    if !matches!(result, InstallResult::Unchanged) {
        let mut entry = Table::new();
        entry["url"] = value(endpoint);
        servers.insert(MCP_SERVER_KEY, Item::Table(entry));
        write_toml_doc(config_path, &doc)?;
    }

    Ok(result)
}

fn uninstall_toml(config_path: &Path) -> Result<()> {
    let mut doc = read_toml_doc(config_path)?;

    if let Some(servers) = doc
        .get_mut(CODEX_SERVER_TABLE)
        .and_then(|i| i.as_table_mut())
    {
        servers.remove(MCP_SERVER_KEY);
    }

    write_toml_doc(config_path, &doc)
}

fn get_endpoint_toml(config_path: &Path) -> Option<String> {
    let doc = read_toml_doc(config_path).ok()?;
    doc.get(CODEX_SERVER_TABLE)?
        .get(MCP_SERVER_KEY)?
        .get("url")?
        .as_str()
        .map(String::from)
}

// ---------------------------------------------------------------------------
// Goose YAML shape
//
// ~/.config/goose/config.yaml — `extensions` map keyed by server name.
// Entry: `{ type: streamable_http, name: engrammic, description: "...",
//           uri: <url>, enabled: true, timeout: 300 }`.
// Preserves all other top-level keys and other extensions.
// ---------------------------------------------------------------------------

const GOOSE_DESCRIPTION: &str = "Engrammic epistemic memory";
const GOOSE_TIMEOUT: u64 = 300;

fn read_yaml(config_path: &Path) -> Result<YamlValue> {
    if !config_path.exists() {
        return Ok(YamlValue::Mapping(serde_yaml::Mapping::new()));
    }
    let content = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    if content.trim().is_empty() {
        return Ok(YamlValue::Mapping(serde_yaml::Mapping::new()));
    }
    serde_yaml::from_str(&content)
        .with_context(|| format!("failed to parse YAML in {}", config_path.display()))
}

fn write_yaml(config_path: &Path, value: &YamlValue) -> Result<()> {
    if let Some(parent) = config_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create directories for {}", config_path.display())
            })?;
        }
    }
    let content = serde_yaml::to_string(value).context("failed to serialize YAML")?;
    fs::write(config_path, content)
        .with_context(|| format!("failed to write {}", config_path.display()))
}

fn build_goose_entry(endpoint: &str) -> YamlValue {
    let mut map = serde_yaml::Mapping::new();
    map.insert(
        YamlValue::String("type".to_string()),
        YamlValue::String("streamable_http".to_string()),
    );
    map.insert(
        YamlValue::String("name".to_string()),
        YamlValue::String(MCP_SERVER_KEY.to_string()),
    );
    map.insert(
        YamlValue::String("description".to_string()),
        YamlValue::String(GOOSE_DESCRIPTION.to_string()),
    );
    map.insert(
        YamlValue::String("uri".to_string()),
        YamlValue::String(endpoint.to_string()),
    );
    map.insert(
        YamlValue::String("enabled".to_string()),
        YamlValue::Bool(true),
    );
    map.insert(
        YamlValue::String("timeout".to_string()),
        YamlValue::Number(serde_yaml::Number::from(GOOSE_TIMEOUT)),
    );
    YamlValue::Mapping(map)
}

fn install_goose_yaml(config_path: &Path, endpoint: &str) -> Result<InstallResult> {
    let mut root = read_yaml(config_path)?;

    let root_map = root
        .as_mapping_mut()
        .context("goose config root is not a YAML mapping")?;

    let extensions_key = YamlValue::String("extensions".to_string());
    if !root_map.contains_key(&extensions_key) {
        root_map.insert(
            extensions_key.clone(),
            YamlValue::Mapping(serde_yaml::Mapping::new()),
        );
    }

    let extensions = root_map
        .get_mut(&extensions_key)
        .and_then(|v| v.as_mapping_mut())
        .context("goose config 'extensions' is not a YAML mapping")?;

    let server_key = YamlValue::String(MCP_SERVER_KEY.to_string());
    let old_uri = extensions
        .get(&server_key)
        .and_then(|v| v.as_mapping())
        .and_then(|m| m.get(YamlValue::String("uri".to_string())))
        .and_then(|v| v.as_str())
        .map(String::from);

    let result = match old_uri {
        Some(ref uri) if uri == endpoint => InstallResult::Unchanged,
        Some(uri) => InstallResult::Updated { old_url: uri },
        None => InstallResult::Created,
    };

    if !matches!(result, InstallResult::Unchanged) {
        extensions.insert(server_key, build_goose_entry(endpoint));
        write_yaml(config_path, &root)?;
    }

    Ok(result)
}

fn uninstall_goose_yaml(config_path: &Path) -> Result<()> {
    let mut root = read_yaml(config_path)?;

    if let Some(extensions) = root
        .as_mapping_mut()
        .and_then(|m| m.get_mut(YamlValue::String("extensions".to_string())))
        .and_then(|v| v.as_mapping_mut())
    {
        extensions.remove(YamlValue::String(MCP_SERVER_KEY.to_string()));
    }

    write_yaml(config_path, &root)
}

fn get_endpoint_goose_yaml(config_path: &Path) -> Option<String> {
    let root = read_yaml(config_path).ok()?;
    root.as_mapping()?
        .get(YamlValue::String("extensions".to_string()))?
        .as_mapping()?
        .get(YamlValue::String(MCP_SERVER_KEY.to_string()))?
        .as_mapping()?
        .get(YamlValue::String("uri".to_string()))?
        .as_str()
        .map(String::from)
}

// ---------------------------------------------------------------------------
// OpenCode JSON shape
//
// ~/.config/opencode/opencode.json — `mcp` map keyed by server name.
// Entry: `{ "type": "remote", "url": <url>, "enabled": true }`.
// Preserves all other top-level keys and other mcp servers.
// ---------------------------------------------------------------------------

fn install_opencode_json(config_path: &Path, endpoint: &str) -> Result<InstallResult> {
    let mut root = read_json(config_path)?;

    let servers = root
        .as_object_mut()
        .context("opencode config root is not a JSON object")?
        .entry("mcp")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .context("opencode config 'mcp' is not a JSON object")?;

    let old_url = servers
        .get(MCP_SERVER_KEY)
        .and_then(|v| v.get("url"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let result = match old_url {
        Some(ref url) if url == endpoint => InstallResult::Unchanged,
        Some(url) => InstallResult::Updated { old_url: url },
        None => InstallResult::Created,
    };

    if !matches!(result, InstallResult::Unchanged) {
        let mut entry = build_json_entry(endpoint, TypeField::Remote, "url");
        entry
            .as_object_mut()
            .expect("build_json_entry always returns an object")
            .insert("enabled".to_string(), json!(true));
        servers.insert(MCP_SERVER_KEY.to_string(), entry);
        write_json(config_path, &root)?;
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Continue.dev project YAML shape
//
// .continue/mcpServers/engrammic.yaml — top-level `mcpServers` list.
// Entry: `{ name: engrammic, type: streamable-http, url: <url> }`.
// Other list items in mcpServers are preserved on uninstall.
// ---------------------------------------------------------------------------

fn build_continue_entry(endpoint: &str) -> YamlValue {
    let mut map = serde_yaml::Mapping::new();
    map.insert(
        YamlValue::String("name".to_string()),
        YamlValue::String(MCP_SERVER_KEY.to_string()),
    );
    map.insert(
        YamlValue::String("type".to_string()),
        YamlValue::String("streamable-http".to_string()),
    );
    map.insert(
        YamlValue::String("url".to_string()),
        YamlValue::String(endpoint.to_string()),
    );
    YamlValue::Mapping(map)
}

fn install_continue_yaml(config_path: &Path, endpoint: &str) -> Result<InstallResult> {
    let mut root = read_yaml(config_path)?;

    let root_map = root
        .as_mapping_mut()
        .context("continue config root is not a YAML mapping")?;

    let servers_key = YamlValue::String("mcpServers".to_string());
    if !root_map.contains_key(&servers_key) {
        root_map.insert(servers_key.clone(), YamlValue::Sequence(vec![]));
    }

    let servers = root_map
        .get_mut(&servers_key)
        .and_then(|v| v.as_sequence_mut())
        .context("continue config 'mcpServers' is not a YAML sequence")?;

    // Find existing engrammic entry by name field.
    let existing_idx = servers.iter().position(|item| {
        item.as_mapping()
            .and_then(|m| m.get(YamlValue::String("name".to_string())))
            .and_then(|v| v.as_str())
            .map(|n| n == MCP_SERVER_KEY)
            .unwrap_or(false)
    });

    let old_url = existing_idx.and_then(|idx| {
        servers[idx]
            .as_mapping()
            .and_then(|m| m.get(YamlValue::String("url".to_string())))
            .and_then(|v| v.as_str())
            .map(String::from)
    });

    let result = match old_url {
        Some(ref url) if url == endpoint => InstallResult::Unchanged,
        Some(url) => InstallResult::Updated { old_url: url },
        None => InstallResult::Created,
    };

    if !matches!(result, InstallResult::Unchanged) {
        let entry = build_continue_entry(endpoint);
        if let Some(idx) = existing_idx {
            servers[idx] = entry;
        } else {
            servers.push(entry);
        }
        write_yaml(config_path, &root)?;
    }

    Ok(result)
}

fn uninstall_continue_yaml(config_path: &Path) -> Result<()> {
    let mut root = read_yaml(config_path)?;

    if let Some(servers) = root
        .as_mapping_mut()
        .and_then(|m| m.get_mut(YamlValue::String("mcpServers".to_string())))
        .and_then(|v| v.as_sequence_mut())
    {
        servers.retain(|item| {
            item.as_mapping()
                .and_then(|m| m.get(YamlValue::String("name".to_string())))
                .and_then(|v| v.as_str())
                .map(|n| n != MCP_SERVER_KEY)
                .unwrap_or(true)
        });
    }

    write_yaml(config_path, &root)
}

fn get_endpoint_continue_yaml(config_path: &Path) -> Option<String> {
    let root = read_yaml(config_path).ok()?;
    let servers = root
        .as_mapping()?
        .get(YamlValue::String("mcpServers".to_string()))?
        .as_sequence()?;
    for item in servers {
        let m = item.as_mapping()?;
        let is_engrammic = m
            .get(YamlValue::String("name".to_string()))
            .and_then(|v| v.as_str())
            .map(|n| n == MCP_SERVER_KEY)
            .unwrap_or(false);
        if is_engrammic {
            return m
                .get(YamlValue::String("url".to_string()))
                .and_then(|v| v.as_str())
                .map(String::from);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Hermes YAML shape
//
// ~/.hermes/config.yaml — `mcp_servers` map keyed by server name.
// Entry: `{ url: <url> }`.
// Preserves all other top-level keys and other mcp_servers entries.
// ---------------------------------------------------------------------------

fn install_hermes_yaml(config_path: &Path, endpoint: &str) -> Result<InstallResult> {
    let mut root = read_yaml(config_path)?;

    let root_map = root
        .as_mapping_mut()
        .context("hermes config root is not a YAML mapping")?;

    let servers_key = YamlValue::String("mcp_servers".to_string());
    if !root_map.contains_key(&servers_key) {
        root_map.insert(
            servers_key.clone(),
            YamlValue::Mapping(serde_yaml::Mapping::new()),
        );
    }

    let servers = root_map
        .get_mut(&servers_key)
        .and_then(|v| v.as_mapping_mut())
        .context("hermes config 'mcp_servers' is not a YAML mapping")?;

    let server_key = YamlValue::String(MCP_SERVER_KEY.to_string());
    let old_url = servers
        .get(&server_key)
        .and_then(|v| v.as_mapping())
        .and_then(|m| m.get(YamlValue::String("url".to_string())))
        .and_then(|v| v.as_str())
        .map(String::from);

    let result = match old_url {
        Some(ref url) if url == endpoint => InstallResult::Unchanged,
        Some(url) => InstallResult::Updated { old_url: url },
        None => InstallResult::Created,
    };

    if !matches!(result, InstallResult::Unchanged) {
        let mut entry = serde_yaml::Mapping::new();
        entry.insert(
            YamlValue::String("url".to_string()),
            YamlValue::String(endpoint.to_string()),
        );
        servers.insert(server_key, YamlValue::Mapping(entry));
        write_yaml(config_path, &root)?;
    }

    Ok(result)
}

fn uninstall_hermes_yaml(config_path: &Path) -> Result<()> {
    let mut root = read_yaml(config_path)?;

    if let Some(servers) = root
        .as_mapping_mut()
        .and_then(|m| m.get_mut(YamlValue::String("mcp_servers".to_string())))
        .and_then(|v| v.as_mapping_mut())
    {
        servers.remove(YamlValue::String(MCP_SERVER_KEY.to_string()));
    }

    write_yaml(config_path, &root)
}

fn get_endpoint_hermes_yaml(config_path: &Path) -> Option<String> {
    let root = read_yaml(config_path).ok()?;
    root.as_mapping()?
        .get(YamlValue::String("mcp_servers".to_string()))?
        .as_mapping()?
        .get(YamlValue::String(MCP_SERVER_KEY.to_string()))?
        .as_mapping()?
        .get(YamlValue::String("url".to_string()))?
        .as_str()
        .map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const STANDARD: ConfigShape = ConfigShape::JsonMap {
        container: "mcpServers",
        type_field: TypeField::Http,
        url_field: "url",
    };
    const EP: &str = "https://beta.engrammic.ai/mcp/";
    const EP2: &str = "http://localhost:8000/mcp";

    fn is_installed(config_path: &Path, endpoint: &str, shape: ConfigShape) -> bool {
        get_installed_endpoint(config_path, shape)
            .map(|ep| ep == endpoint)
            .unwrap_or(false)
    }

    // --- JSON map (regression for the original behavior) ---

    #[test]
    fn json_install_creates_and_is_installed() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".claude/settings.json");

        assert!(matches!(
            install(&path, EP, STANDARD).unwrap(),
            InstallResult::Created
        ));
        assert!(is_installed(&path, EP, STANDARD));
        assert!(!is_installed(&path, EP2, STANDARD));

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"mcpServers\""));
        assert!(content.contains("\"type\": \"http\""));
        assert!(content.contains(EP));
    }

    #[test]
    fn json_preserves_other_servers_and_keys() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            r#"{ "theme": "dark", "mcpServers": { "other": { "url": "https://other.example/mcp" } } }"#,
        )
        .unwrap();

        install(&path, EP, STANDARD).unwrap();
        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["theme"], "dark");
        assert_eq!(v["mcpServers"]["other"]["url"], "https://other.example/mcp");
        assert_eq!(v["mcpServers"]["engrammic"]["url"], EP);

        uninstall(&path, STANDARD).unwrap();
        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(v["mcpServers"].get("engrammic").is_none());
        assert_eq!(v["mcpServers"]["other"]["url"], "https://other.example/mcp");
        assert_eq!(v["theme"], "dark");
    }

    #[test]
    fn json_update_reports_old_url() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        install(&path, EP2, STANDARD).unwrap();
        match install(&path, EP, STANDARD).unwrap() {
            InstallResult::Updated { old_url } => assert_eq!(old_url, EP2),
            other => panic!(
                "expected Updated, got {:?}",
                matches!(other, InstallResult::Created)
            ),
        }
        assert!(matches!(
            install(&path, EP, STANDARD).unwrap(),
            InstallResult::Unchanged
        ));
    }

    #[test]
    fn json_container_and_type_are_parameterized() {
        // VS Code uses `servers`; some harnesses omit the `type` field entirely.
        let dir = tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        let shape = ConfigShape::JsonMap {
            container: "servers",
            type_field: TypeField::None,
            url_field: "url",
        };
        install(&path, EP, shape).unwrap();
        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["servers"]["engrammic"]["url"], EP);
        assert!(v["servers"]["engrammic"].get("type").is_none());
        assert!(is_installed(&path, EP, shape));
    }

    // --- Codex TOML ---

    #[test]
    fn toml_install_creates_table() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".codex/config.toml");

        assert!(matches!(
            install(&path, EP, ConfigShape::CodexToml).unwrap(),
            InstallResult::Created
        ));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("[mcp_servers.engrammic]"));
        assert!(content.contains(&format!("url = \"{EP}\"")));
        // implicit parent: no bare `[mcp_servers]` header
        assert!(!content.contains("[mcp_servers]\n"));
        assert!(is_installed(&path, EP, ConfigShape::CodexToml));
        assert!(!is_installed(&path, EP2, ConfigShape::CodexToml));
    }

    #[test]
    fn toml_preserves_comments_other_tables_and_servers() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let original = "\
# my codex config
model = \"o3\"

[mcp_servers.other]
url = \"https://other.example/mcp\"

[shell]
approval = \"on-request\"  # inline comment
";
        fs::write(&path, original).unwrap();

        install(&path, EP, ConfigShape::CodexToml).unwrap();
        let after = fs::read_to_string(&path).unwrap();
        assert!(after.contains("# my codex config"));
        assert!(after.contains("model = \"o3\""));
        assert!(after.contains("[mcp_servers.other]"));
        assert!(after.contains("https://other.example/mcp"));
        assert!(after.contains("# inline comment"));
        assert!(after.contains("[mcp_servers.engrammic]"));

        uninstall(&path, ConfigShape::CodexToml).unwrap();
        let cleaned = fs::read_to_string(&path).unwrap();
        assert!(!cleaned.contains("[mcp_servers.engrammic]"));
        assert!(cleaned.contains("[mcp_servers.other]"));
        assert!(cleaned.contains("# my codex config"));
        assert!(cleaned.contains("# inline comment"));
        assert!(cleaned.contains("model = \"o3\""));
    }

    #[test]
    fn toml_update_and_unchanged() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        install(&path, EP2, ConfigShape::CodexToml).unwrap();
        assert!(matches!(
            install(&path, EP, ConfigShape::CodexToml).unwrap(),
            InstallResult::Updated { .. }
        ));
        assert!(matches!(
            install(&path, EP, ConfigShape::CodexToml).unwrap(),
            InstallResult::Unchanged
        ));
    }

    // --- Goose YAML ---

    #[test]
    fn goose_yaml_install_creates_and_is_installed() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".config/goose/config.yaml");

        assert!(matches!(
            install(&path, EP, ConfigShape::GooseYaml).unwrap(),
            InstallResult::Created
        ));
        assert!(is_installed(&path, EP, ConfigShape::GooseYaml));
        assert!(!is_installed(&path, EP2, ConfigShape::GooseYaml));

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("extensions:"));
        assert!(content.contains("engrammic:"));
        assert!(content.contains("streamable_http"));
        assert!(content.contains("uri:"));
        assert!(content.contains(EP));
        assert!(content.contains("enabled: true"));
        assert!(content.contains("timeout: 300"));
        assert!(content.contains("description:"));
    }

    #[test]
    fn goose_yaml_preserves_other_extensions_and_keys() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            "theme: dark\nextensions:\n  other:\n    type: streamable_http\n    uri: https://other.example/mcp\n    enabled: true\n",
        )
        .unwrap();

        install(&path, EP, ConfigShape::GooseYaml).unwrap();
        assert!(is_installed(&path, EP, ConfigShape::GooseYaml));

        let v: YamlValue = serde_yaml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["theme"].as_str().unwrap(), "dark");
        assert!(v["extensions"]["other"].is_mapping());
        assert!(v["extensions"]["engrammic"].is_mapping());

        uninstall(&path, ConfigShape::GooseYaml).unwrap();
        let v: YamlValue = serde_yaml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(v["extensions"].get("engrammic").is_none());
        assert!(v["extensions"]["other"].is_mapping());
        assert_eq!(v["theme"].as_str().unwrap(), "dark");
    }

    #[test]
    fn goose_yaml_update_and_unchanged() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        install(&path, EP2, ConfigShape::GooseYaml).unwrap();
        match install(&path, EP, ConfigShape::GooseYaml).unwrap() {
            InstallResult::Updated { old_url } => assert_eq!(old_url, EP2),
            other => panic!(
                "expected Updated, got {:?}",
                matches!(other, InstallResult::Created)
            ),
        }
        assert!(matches!(
            install(&path, EP, ConfigShape::GooseYaml).unwrap(),
            InstallResult::Unchanged
        ));
    }

    // --- Amp flat dotted key ---

    #[test]
    fn amp_flat_key_install_and_is_installed() {
        let amp_shape = ConfigShape::JsonMap {
            container: "amp.mcpServers",
            type_field: TypeField::None,
            url_field: "url",
        };
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");

        assert!(matches!(
            install(&path, EP, amp_shape).unwrap(),
            InstallResult::Created
        ));
        assert!(is_installed(&path, EP, amp_shape));

        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        // Root key is the literal string "amp.mcpServers"
        assert_eq!(v["amp.mcpServers"]["engrammic"]["url"], EP);
        assert!(v["amp.mcpServers"]["engrammic"].get("type").is_none());

        // Preserves other servers under the same container key
        uninstall(&path, amp_shape).unwrap();
        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(v["amp.mcpServers"].get("engrammic").is_none());
    }

    #[test]
    fn amp_preserves_unrelated_config() {
        let amp_shape = ConfigShape::JsonMap {
            container: "amp.mcpServers",
            type_field: TypeField::None,
            url_field: "url",
        };
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            r#"{ "editor.fontSize": 14, "amp.mcpServers": { "other": { "url": "https://other.example/mcp" } } }"#,
        )
        .unwrap();

        install(&path, EP, amp_shape).unwrap();
        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["editor.fontSize"], 14);
        assert_eq!(
            v["amp.mcpServers"]["other"]["url"],
            "https://other.example/mcp"
        );
        assert_eq!(v["amp.mcpServers"]["engrammic"]["url"], EP);
    }

    // --- Zed context_servers ---

    #[test]
    fn zed_context_servers_install_and_is_installed() {
        let zed_shape = ConfigShape::JsonMap {
            container: "context_servers",
            type_field: TypeField::None,
            url_field: "url",
        };
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");

        assert!(matches!(
            install(&path, EP, zed_shape).unwrap(),
            InstallResult::Created
        ));
        assert!(is_installed(&path, EP, zed_shape));

        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["context_servers"]["engrammic"]["url"], EP);
        assert!(v["context_servers"]["engrammic"].get("type").is_none());

        uninstall(&path, zed_shape).unwrap();
        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(v["context_servers"].get("engrammic").is_none());
    }

    // --- OpenCode enabled field ---

    #[test]
    fn opencode_json_install_creates_and_is_installed() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("opencode.json");

        assert!(matches!(
            install(&path, EP, ConfigShape::OpenCodeJson).unwrap(),
            InstallResult::Created
        ));
        assert!(is_installed(&path, EP, ConfigShape::OpenCodeJson));
        assert!(!is_installed(&path, EP2, ConfigShape::OpenCodeJson));

        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["mcp"]["engrammic"]["type"], "remote");
        assert_eq!(v["mcp"]["engrammic"]["url"], EP);
        assert_eq!(v["mcp"]["engrammic"]["enabled"], true);
    }

    #[test]
    fn opencode_json_preserves_other_servers_and_keys() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("opencode.json");
        fs::write(
            &path,
            r#"{ "theme": "dark", "mcp": { "other": { "type": "remote", "url": "https://other.example/mcp", "enabled": true } } }"#,
        )
        .unwrap();

        install(&path, EP, ConfigShape::OpenCodeJson).unwrap();
        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["theme"], "dark");
        assert_eq!(v["mcp"]["other"]["url"], "https://other.example/mcp");
        assert_eq!(v["mcp"]["engrammic"]["url"], EP);
        assert_eq!(v["mcp"]["engrammic"]["enabled"], true);

        uninstall(&path, ConfigShape::OpenCodeJson).unwrap();
        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(v["mcp"].get("engrammic").is_none());
        assert_eq!(v["mcp"]["other"]["url"], "https://other.example/mcp");
        assert_eq!(v["theme"], "dark");
    }

    #[test]
    fn opencode_json_update_and_unchanged() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("opencode.json");
        install(&path, EP2, ConfigShape::OpenCodeJson).unwrap();
        match install(&path, EP, ConfigShape::OpenCodeJson).unwrap() {
            InstallResult::Updated { old_url } => assert_eq!(old_url, EP2),
            other => panic!(
                "expected Updated, got {:?}",
                matches!(other, InstallResult::Created)
            ),
        }
        assert!(matches!(
            install(&path, EP, ConfigShape::OpenCodeJson).unwrap(),
            InstallResult::Unchanged
        ));
    }

    // --- Roo Code streamable-http ---

    #[test]
    fn roo_streamable_http_type_field() {
        let roo_shape = ConfigShape::JsonMap {
            container: "mcpServers",
            type_field: TypeField::StreamableHttp,
            url_field: "url",
        };
        let dir = tempdir().unwrap();
        let path = dir.path().join("mcp.json");

        install(&path, EP, roo_shape).unwrap();
        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["mcpServers"]["engrammic"]["type"], "streamable-http");
        assert_eq!(v["mcpServers"]["engrammic"]["url"], EP);
        assert!(is_installed(&path, EP, roo_shape));
    }

    // --- Continue.dev YAML ---

    #[test]
    fn continue_yaml_install_creates_and_is_installed() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("engrammic.yaml");

        assert!(matches!(
            install(&path, EP, ConfigShape::ContinueYaml).unwrap(),
            InstallResult::Created
        ));
        assert!(is_installed(&path, EP, ConfigShape::ContinueYaml));
        assert!(!is_installed(&path, EP2, ConfigShape::ContinueYaml));

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("mcpServers:"));
        assert!(content.contains("name: engrammic"));
        assert!(content.contains("streamable-http"));
        assert!(content.contains(EP));
    }

    #[test]
    fn continue_yaml_preserves_other_servers() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("engrammic.yaml");
        fs::write(
            &path,
            "mcpServers:\n- name: other\n  type: streamable-http\n  url: https://other.example/mcp\n",
        )
        .unwrap();

        install(&path, EP, ConfigShape::ContinueYaml).unwrap();
        assert!(is_installed(&path, EP, ConfigShape::ContinueYaml));

        let v: YamlValue = serde_yaml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let servers = v["mcpServers"].as_sequence().unwrap();
        assert_eq!(servers.len(), 2);

        uninstall(&path, ConfigShape::ContinueYaml).unwrap();
        let v: YamlValue = serde_yaml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let servers = v["mcpServers"].as_sequence().unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0]["name"].as_str().unwrap(), "other");
    }

    #[test]
    fn continue_yaml_update_and_unchanged() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("engrammic.yaml");
        install(&path, EP2, ConfigShape::ContinueYaml).unwrap();
        match install(&path, EP, ConfigShape::ContinueYaml).unwrap() {
            InstallResult::Updated { old_url } => assert_eq!(old_url, EP2),
            other => panic!(
                "expected Updated, got {:?}",
                matches!(other, InstallResult::Created)
            ),
        }
        assert!(matches!(
            install(&path, EP, ConfigShape::ContinueYaml).unwrap(),
            InstallResult::Unchanged
        ));
    }

    // --- Hermes YAML ---

    #[test]
    fn hermes_yaml_install_creates_and_is_installed() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".hermes/config.yaml");

        assert!(matches!(
            install(&path, EP, ConfigShape::HermesYaml).unwrap(),
            InstallResult::Created
        ));
        assert!(is_installed(&path, EP, ConfigShape::HermesYaml));
        assert!(!is_installed(&path, EP2, ConfigShape::HermesYaml));

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("mcp_servers:"));
        assert!(content.contains("engrammic:"));
        assert!(content.contains("url:"));
        assert!(content.contains(EP));
    }

    #[test]
    fn hermes_yaml_preserves_other_servers_and_keys() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            "timeout: 60\nmcp_servers:\n  other:\n    url: https://other.example/mcp\n",
        )
        .unwrap();

        install(&path, EP, ConfigShape::HermesYaml).unwrap();
        assert!(is_installed(&path, EP, ConfigShape::HermesYaml));

        let v: YamlValue = serde_yaml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["timeout"].as_i64().unwrap(), 60);
        assert!(v["mcp_servers"]["other"].is_mapping());
        assert!(v["mcp_servers"]["engrammic"].is_mapping());

        uninstall(&path, ConfigShape::HermesYaml).unwrap();
        let v: YamlValue = serde_yaml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(v["mcp_servers"].get("engrammic").is_none());
        assert!(v["mcp_servers"]["other"].is_mapping());
        assert_eq!(v["timeout"].as_i64().unwrap(), 60);
    }

    #[test]
    fn hermes_yaml_update_and_unchanged() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        install(&path, EP2, ConfigShape::HermesYaml).unwrap();
        match install(&path, EP, ConfigShape::HermesYaml).unwrap() {
            InstallResult::Updated { old_url } => assert_eq!(old_url, EP2),
            other => panic!(
                "expected Updated, got {:?}",
                matches!(other, InstallResult::Created)
            ),
        }
        assert!(matches!(
            install(&path, EP, ConfigShape::HermesYaml).unwrap(),
            InstallResult::Unchanged
        ));
    }
}

#[cfg(test)]
mod backup_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_bak_once_and_never_overwrites() {
        let dir = tempdir().unwrap();
        let cfg = dir.path().join("mcp.json");
        std::fs::write(&cfg, "{\"original\": true}").unwrap();

        let bak = ensure_backup(&cfg).unwrap().expect("backup path");
        assert_eq!(
            std::fs::read_to_string(&bak).unwrap(),
            "{\"original\": true}"
        );

        // Mutate the config, call again: backup must keep the ORIGINAL content.
        std::fs::write(&cfg, "{\"mutated\": true}").unwrap();
        let bak2 = ensure_backup(&cfg).unwrap().expect("backup path");
        assert_eq!(bak, bak2);
        assert_eq!(
            std::fs::read_to_string(&bak).unwrap(),
            "{\"original\": true}"
        );
    }

    #[test]
    fn missing_config_yields_no_backup() {
        let dir = tempdir().unwrap();
        let cfg = dir.path().join("does-not-exist.json");
        assert!(ensure_backup(&cfg).unwrap().is_none());
        assert!(!dir
            .path()
            .join("does-not-exist.json.engrammic.bak")
            .exists());
    }
}
