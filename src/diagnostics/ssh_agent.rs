use anyhow::Result;
use std::env;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct SshAgentStatus {
    pub configured: bool,
    pub gpg_agent_has_ssh_support: bool,
    pub ssh_auth_sock_correct: bool,
    pub auth_sock: Option<String>,
    pub expected_sock: Option<String>,
}

pub fn check_ssh_agent() -> Result<SshAgentStatus> {
    let auth_sock = env::var("SSH_AUTH_SOCK").ok();
    
    // Check if gpg-agent.conf has enable-ssh-support
    let gpg_agent_conf = dirs::home_dir()
        .map(|h| h.join(".gnupg/gpg-agent.conf"))
        .and_then(|p| std::fs::read_to_string(p).ok());
    
    let gpg_agent_has_ssh_support = gpg_agent_conf
        .as_ref()
        .map(|conf| conf.contains("enable-ssh-support"))
        .unwrap_or(false);
    
    // Get the expected GPG agent SSH socket path
    let expected_sock = Command::new("gpgconf")
        .arg("--list-dirs")
        .arg("agent-ssh-socket")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        });
    
    // Check if SSH_AUTH_SOCK points to gpg-agent
    let ssh_auth_sock_correct = if let (Some(sock), Some(expected)) = (&auth_sock, &expected_sock) {
        sock == expected
    } else {
        false
    };
    
    // Overall configured status: both conditions must be true
    let configured = gpg_agent_has_ssh_support && ssh_auth_sock_correct;

    Ok(SshAgentStatus {
        configured,
        gpg_agent_has_ssh_support,
        ssh_auth_sock_correct,
        auth_sock,
        expected_sock,
    })
}
