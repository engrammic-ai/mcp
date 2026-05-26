use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{json, Value};

const MCP_SERVER_KEY: &str = "engrammic";

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
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directories for {}", config_path.display()))?;
    }
    let content = serde_json::to_string_pretty(value)
        .context("failed to serialize JSON")?;
    fs::write(config_path, content)
        .with_context(|| format!("failed to write {}", config_path.display()))
}

pub enum InstallResult {
    Created,
    Updated { old_url: String },
    Unchanged,
}

pub fn install(config_path: &Path, endpoint: &str) -> Result<InstallResult> {
    let mut root = read_json(config_path)?;

    let servers = root
        .as_object_mut()
        .context("config root is not a JSON object")?
        .entry("mcpServers")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .context("mcpServers is not a JSON object")?;

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
        servers.insert(
            MCP_SERVER_KEY.to_string(),
            json!({
                "type": "http",
                "url": endpoint
            }),
        );
        write_json(config_path, &root)?;
    }

    Ok(result)
}

pub fn uninstall(config_path: &Path) -> Result<()> {
    let mut root = read_json(config_path)?;

    if let Some(servers) = root
        .as_object_mut()
        .and_then(|o| o.get_mut("mcpServers"))
        .and_then(|v| v.as_object_mut())
    {
        servers.remove(MCP_SERVER_KEY);
    }

    write_json(config_path, &root)
}

pub fn is_installed(config_path: &Path, endpoint: &str) -> bool {
    let Ok(root) = read_json(config_path) else {
        return false;
    };

    root.get("mcpServers")
        .and_then(|v| v.get(MCP_SERVER_KEY))
        .and_then(|v| v.get("url"))
        .and_then(|v| v.as_str())
        .map(|url| url == endpoint)
        .unwrap_or(false)
}
