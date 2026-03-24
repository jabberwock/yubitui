use anyhow::Result;
use std::path::PathBuf;

/// Authoritative gnupg home directory resolution.
/// Priority: $GNUPGHOME > gpgconf --list-dirs homedir > platform fallback.
/// All code that needs the gnupg home MUST call this function.
pub fn gnupg_home() -> Result<PathBuf> {
    // 1. Explicit override wins
    if let Ok(gnupg_home) = std::env::var("GNUPGHOME") {
        return Ok(PathBuf::from(gnupg_home));
    }

    // 2. Ask gpgconf what it actually uses -- works on all platforms
    if let Ok(output) = std::process::Command::new("gpgconf")
        .arg("--list-dirs")
        .arg("homedir")
        .output()
    {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                return Ok(PathBuf::from(path_str));
            }
        }
    }

    // 3. Platform-aware fallback
    #[cfg(target_os = "windows")]
    {
        // Windows GPG4Win uses %APPDATA%\gnupg
        if let Some(appdata) = dirs::config_dir() {
            let gnupg = appdata.join("gnupg");
            if gnupg.exists() {
                return Ok(gnupg);
            }
        }
    }

    // 4. Unix fallback
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    Ok(home.join(".gnupg"))
}

pub fn gpg_agent_conf() -> Result<PathBuf> {
    Ok(gnupg_home()?.join("gpg-agent.conf"))
}

pub fn scdaemon_conf() -> Result<PathBuf> {
    Ok(gnupg_home()?.join("scdaemon.conf"))
}
