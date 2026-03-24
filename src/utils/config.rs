use anyhow::Result;
use std::path::PathBuf;

/// Get the GPG home directory
pub fn gnupg_home() -> Result<PathBuf> {
    if let Ok(home) = std::env::var("GNUPGHOME") {
        return Ok(PathBuf::from(home));
    }

    dirs::home_dir()
        .map(|h| h.join(".gnupg"))
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))
}

/// Get the gpg-agent.conf path
pub fn gpg_agent_conf() -> Result<PathBuf> {
    Ok(gnupg_home()?.join("gpg-agent.conf"))
}

/// Get the scdaemon.conf path
pub fn scdaemon_conf() -> Result<PathBuf> {
    Ok(gnupg_home()?.join("scdaemon.conf"))
}

/// Check if a config file exists
pub fn config_exists(path: &PathBuf) -> bool {
    path.exists() && path.is_file()
}

/// Read config file contents
pub fn read_config(path: &PathBuf) -> Result<String> {
    std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))
}

/// Check if gpg-agent.conf has enable-ssh-support
pub fn has_ssh_support_enabled() -> Result<bool> {
    let conf_path = gpg_agent_conf()?;
    
    if !config_exists(&conf_path) {
        return Ok(false);
    }

    let contents = read_config(&conf_path)?;
    Ok(contents.lines().any(|l| {
        let trimmed = l.trim();
        !trimmed.starts_with('#') && trimmed == "enable-ssh-support"
    }))
}
