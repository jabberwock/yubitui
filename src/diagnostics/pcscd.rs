use anyhow::Result;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct PcscdStatus {
    pub running: bool,
    pub version: Option<String>,
}

pub fn check_pcscd() -> Result<PcscdStatus> {
    // Try to detect if pcscd is running
    #[cfg(target_os = "macos")]
    let running = Command::new("launchctl")
        .args(&["list", "com.apple.securityd"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    #[cfg(target_os = "linux")]
    let running = Command::new("systemctl")
        .args(&["is-active", "pcscd"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let running = false;

    // Try pcsc_scan to get version
    let version = Command::new("pcscd")
        .arg("-v")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next().map(|l| l.to_string()));

    Ok(PcscdStatus { running, version })
}
