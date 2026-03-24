use anyhow::Result;

#[derive(Debug, Clone)]
pub struct SshConfig {
    pub gpg_agent_configured: bool,
    pub ssh_support_enabled: bool,
    pub public_key: Option<String>,
}

pub fn get_ssh_config() -> Result<SshConfig> {
    // TODO: Check gpg-agent.conf for enable-ssh-support
    // TODO: Check if gpg-agent is running
    // TODO: Extract SSH public key from authentication key
    Ok(SshConfig {
        gpg_agent_configured: false,
        ssh_support_enabled: false,
        public_key: None,
    })
}
