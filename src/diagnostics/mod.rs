pub mod gpg_agent;
pub mod pcscd;
pub mod scdaemon;
pub mod ssh_agent;

use anyhow::Result;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Diagnostics {
    pub gpg_agent: gpg_agent::GpgAgentStatus,
    pub scdaemon: scdaemon::ScdaemonStatus,
    pub pcscd: pcscd::PcscdStatus,
    pub ssh_agent: ssh_agent::SshAgentStatus,
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self {
            gpg_agent: gpg_agent::GpgAgentStatus {
                running: true,
                version: Some("mock".to_string()),
                socket_path: None,
            },
            scdaemon: scdaemon::ScdaemonStatus {
                configured: true,
                issues: None,
            },
            pcscd: pcscd::PcscdStatus {
                running: true,
                version: Some("mock".to_string()),
            },
            ssh_agent: ssh_agent::SshAgentStatus {
                configured: true,
                gpg_agent_has_ssh_support: true,
                ssh_auth_sock_correct: true,
                auth_sock: None,
                expected_sock: None,
            },
        }
    }
}

impl Diagnostics {
    pub fn run() -> Result<Self> {
        Ok(Self {
            gpg_agent: gpg_agent::check_gpg_agent()?,
            scdaemon: scdaemon::check_scdaemon()?,
            pcscd: pcscd::check_pcscd()?,
            ssh_agent: ssh_agent::check_ssh_agent()?,
        })
    }

    pub fn has_errors(&self) -> bool {
        !self.gpg_agent.running || !self.pcscd.running || !self.scdaemon.configured
    }

    #[allow(dead_code)]
    pub fn has_warnings(&self) -> bool {
        !self.ssh_agent.configured || self.scdaemon.issues.is_some()
    }
}

impl fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "System Diagnostics:")?;
        writeln!(f, "==================\n")?;

        writeln!(f, "PC/SC Daemon (pcscd):")?;
        writeln!(
            f,
            "  Status: {}",
            if self.pcscd.running {
                "✅ Running"
            } else {
                "❌ Not running"
            }
        )?;
        if let Some(ref version) = self.pcscd.version {
            writeln!(f, "  Version: {}", version)?;
        }
        writeln!(f)?;

        writeln!(f, "GPG Agent:")?;
        writeln!(
            f,
            "  Status: {}",
            if self.gpg_agent.running {
                "✅ Running"
            } else {
                "❌ Not running"
            }
        )?;
        if let Some(ref version) = self.gpg_agent.version {
            writeln!(f, "  Version: {}", version)?;
        }
        if let Some(ref socket) = self.gpg_agent.socket_path {
            writeln!(f, "  Socket: {}", socket)?;
        }
        writeln!(f)?;

        writeln!(f, "Scdaemon:")?;
        writeln!(
            f,
            "  Configured: {}",
            if self.scdaemon.configured {
                "✅ Yes"
            } else {
                "⚠️  No"
            }
        )?;
        if let Some(ref issues) = self.scdaemon.issues {
            writeln!(f, "  Issues: {}", issues)?;
        }
        writeln!(f)?;

        writeln!(f, "SSH Agent Integration:")?;
        writeln!(
            f,
            "  Configured: {}",
            if self.ssh_agent.configured {
                "✅ Yes"
            } else {
                "⚠️  No"
            }
        )?;

        if !self.ssh_agent.configured {
            if !self.ssh_agent.gpg_agent_has_ssh_support {
                writeln!(
                    f,
                    "  Issues: enable-ssh-support not found in ~/.gnupg/gpg-agent.conf"
                )?;
            }
            if !self.ssh_agent.ssh_auth_sock_correct {
                writeln!(f, "  Issues: SSH_AUTH_SOCK not pointing to GPG agent")?;
                if let Some(ref current) = self.ssh_agent.auth_sock {
                    writeln!(f, "    Current: {}", current)?;
                }
                if let Some(ref expected) = self.ssh_agent.expected_sock {
                    writeln!(f, "    Expected: {}", expected)?;
                    writeln!(f, "    Add to ~/.bashrc or ~/.zshrc:")?;
                    writeln!(f, "      export SSH_AUTH_SOCK=\"{}\"", expected)?;
                }
            }
        } else if let Some(ref sock) = self.ssh_agent.auth_sock {
            writeln!(f, "  SSH_AUTH_SOCK: {}", sock)?;
        }

        Ok(())
    }
}
