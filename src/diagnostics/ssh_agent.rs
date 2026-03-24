use anyhow::Result;
use std::env;

#[derive(Debug, Clone)]
pub struct SshAgentStatus {
    pub configured: bool,
    pub auth_sock: Option<String>,
}

pub fn check_ssh_agent() -> Result<SshAgentStatus> {
    let auth_sock = env::var("SSH_AUTH_SOCK").ok();
    
    // Check if SSH_AUTH_SOCK points to gpg-agent
    let configured = auth_sock
        .as_ref()
        .map(|s| s.contains("gpg-agent"))
        .unwrap_or(false);

    Ok(SshAgentStatus {
        configured,
        auth_sock,
    })
}
