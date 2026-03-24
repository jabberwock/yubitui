use anyhow::Result;
use std::path::PathBuf;

#[allow(dead_code)]
pub fn gnupg_home() -> Result<PathBuf> {
    if let Ok(gnupg_home) = std::env::var("GNUPGHOME") {
        Ok(PathBuf::from(gnupg_home))
    } else {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        Ok(home.join(".gnupg"))
    }
}

#[allow(dead_code)]
pub fn gpg_agent_conf() -> Result<PathBuf> {
    Ok(gnupg_home()?.join("gpg-agent.conf"))
}

#[allow(dead_code)]
pub fn scdaemon_conf() -> Result<PathBuf> {
    Ok(gnupg_home()?.join("scdaemon.conf"))
}
