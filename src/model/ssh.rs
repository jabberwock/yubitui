use anyhow::Result;
use std::process::Command;

#[derive(Debug, Clone, serde::Serialize)]
#[allow(dead_code)]
pub struct SshConfig {
    pub agent_running: bool,
    pub gpg_agent_ssh: bool,
    pub ssh_auth_sock: Option<String>,
    pub keys_available: Vec<String>,
}

#[allow(dead_code)]
pub fn get_ssh_config() -> Result<SshConfig> {
    let ssh_auth_sock = std::env::var("SSH_AUTH_SOCK").ok();

    // Check if it's pointing to GPG agent
    let gpg_agent_ssh = ssh_auth_sock
        .as_ref()
        .map(|s| s.contains("gpg-agent"))
        .unwrap_or(false);

    // Check if ssh-agent or gpg-agent is running
    let agent_running = ssh_auth_sock.is_some();

    // Get list of keys from ssh-add
    let keys_available = if agent_running {
        get_ssh_keys().unwrap_or_default()
    } else {
        vec![]
    };

    Ok(SshConfig {
        agent_running,
        gpg_agent_ssh,
        ssh_auth_sock,
        keys_available,
    })
}

#[allow(dead_code)]
fn get_ssh_keys() -> Result<Vec<String>> {
    let output = Command::new("ssh-add").arg("-L").output()?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let keys: Vec<String> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            // Extract just the key type and comment
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if parts.len() >= 3 {
                    format!("{} ... {}", parts[0], parts[2])
                } else {
                    parts[0].to_string()
                }
            } else {
                line.to_string()
            }
        })
        .collect();

    Ok(keys)
}

#[allow(dead_code)]
pub fn export_ssh_key() -> Result<String> {
    // Export the authentication key as SSH public key
    let output = Command::new("gpg")
        .args(["--export-ssh-key", ""])
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        anyhow::bail!("Failed to export SSH key")
    }
}

#[allow(dead_code)]
pub fn configure_ssh_agent() -> Result<()> {
    // Get GPG agent SSH socket path
    let output = Command::new("gpgconf")
        .args(["--list-dirs", "agent-ssh-socket"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to get GPG agent SSH socket path");
    }

    let socket_path = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Provide instructions to user
    eprintln!("\nTo configure SSH agent, add this to your shell config:");
    eprintln!("  export SSH_AUTH_SOCK=\"{}\"", socket_path);
    eprintln!("\nThen restart your shell or run:");
    eprintln!("  source ~/.bashrc  # or ~/.zshrc");

    Ok(())
}
