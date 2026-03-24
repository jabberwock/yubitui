use anyhow::Result;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GpgAgentStatus {
    pub running: bool,
    pub version: Option<String>,
    pub socket_path: Option<String>,
}

pub fn check_gpg_agent() -> Result<GpgAgentStatus> {
    // Check if gpg-agent is running
    let output = Command::new("gpgconf")
        .arg("--list-dirs")
        .output();

    let running = output.is_ok();
    
    let socket_path = if running {
        output
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("agent-socket:"))
                    .map(|l| l.trim_start_matches("agent-socket:").to_string())
            })
    } else {
        None
    };

    // Get version
    let version = Command::new("gpg")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next().map(|l| l.to_string()));

    Ok(GpgAgentStatus {
        running,
        version,
        socket_path,
    })
}
