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
    let running = {
        // On macOS, check for com.apple.ctkpcscd process
        let ps_check = Command::new("pgrep")
            .args(&["-f", "com.apple.ctkpcscd"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        
        // Also try launchctl
        let launchctl_check = Command::new("launchctl")
            .args(&["list"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.contains("com.apple.ctkpcscd"))
            .unwrap_or(false);
        
        ps_check || launchctl_check
    };

    #[cfg(target_os = "linux")]
    let running = Command::new("systemctl")
        .args(&["is-active", "pcscd"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let running = false;

    // Try to get version info
    #[cfg(target_os = "macos")]
    let version = Some("macOS PC/SC (CryptoTokenKit)".to_string());
    
    #[cfg(not(target_os = "macos"))]
    let version = Command::new("pcscd")
        .arg("-v")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next().map(|l| l.to_string()));

    Ok(PcscdStatus { running, version })
}
