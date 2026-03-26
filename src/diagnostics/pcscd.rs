use anyhow::Result;
#[cfg(not(target_os = "macos"))]
use std::process::Command;

#[derive(Debug, Clone)]
pub struct PcscdStatus {
    pub running: bool,
    pub version: Option<String>,
}

pub fn check_pcscd() -> Result<PcscdStatus> {
    // Try to detect if pcscd is running
    // On macOS, CryptoTokenKit is part of the OS and launched on-demand by launchd
    // when a card reader is present. It is never a permanently-running daemon, so
    // checking for the process always returns false. Treat it as always available.
    #[cfg(target_os = "macos")]
    let running = true;

    #[cfg(target_os = "linux")]
    let running = Command::new("systemctl")
        .args(&["is-active", "pcscd"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    #[cfg(windows)]
    let running = {
        // On Windows, query the Smart Card service (SCardSvr)
        Command::new("sc")
            .args(["query", "SCardSvr"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("RUNNING"))
            .unwrap_or(false)
    };

    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    let running = false;

    // Try to get version info
    #[cfg(target_os = "macos")]
    let version = Some("macOS PC/SC (CryptoTokenKit)".to_string());

    #[cfg(windows)]
    let version = Some("Windows Smart Card Service (SCardSvr)".to_string());

    #[cfg(not(any(target_os = "macos", windows)))]
    let version = Command::new("pcscd")
        .arg("-v")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next().map(|l| l.to_string()));

    Ok(PcscdStatus { running, version })
}
