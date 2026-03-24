use anyhow::Result;

#[derive(Debug, Clone)]
pub struct ScdaemonStatus {
    pub configured: bool,
    pub issues: Option<String>,
}

pub fn check_scdaemon() -> Result<ScdaemonStatus> {
    // Check if scdaemon is configured
    let config_path = dirs::home_dir()
        .map(|h| h.join(".gnupg/scdaemon.conf"))
        .and_then(|p| if p.exists() { Some(p) } else { None });

    let configured = config_path.is_some();

    // Check for common issues
    let issues = if configured {
        // TODO: Parse scdaemon.conf and check for common misconfigurations
        None
    } else {
        Some("scdaemon.conf not found".to_string())
    };

    Ok(ScdaemonStatus { configured, issues })
}
