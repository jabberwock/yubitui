#![allow(dead_code)] // save_theme_name wired in future theme-settings UI
use std::path::PathBuf;

pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("yubitui").join("config.toml"))
}

pub fn read_theme_name() -> Option<String> {
    let path = config_path()?;
    let content = std::fs::read_to_string(path).ok()?;
    let value: toml::Value = toml::from_str(&content).ok()?;
    value.get("theme")?.as_str().map(String::from)
}

pub fn save_theme_name(name: &str) -> anyhow::Result<()> {
    let path = config_path()
        .ok_or_else(|| anyhow::anyhow!("no config directory found"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = format!("theme = \"{}\"\n", name);
    std::fs::write(path, content)?;
    Ok(())
}
