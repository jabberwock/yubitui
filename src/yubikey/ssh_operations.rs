use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

/// Check if enable-ssh-support is in gpg-agent.conf
pub fn check_ssh_support_enabled() -> Result<bool> {
    let conf_path = get_gpg_agent_conf_path()?;
    
    if !conf_path.exists() {
        return Ok(false);
    }
    
    let content = fs::read_to_string(&conf_path)?;
    Ok(content.contains("enable-ssh-support"))
}

/// Enable SSH support in gpg-agent.conf
pub fn enable_ssh_support() -> Result<String> {
    let conf_path = get_gpg_agent_conf_path()?;
    
    // Read existing content
    let mut content = if conf_path.exists() {
        fs::read_to_string(&conf_path)?
    } else {
        String::new()
    };
    
    // Check if already enabled
    if content.contains("enable-ssh-support") {
        return Ok("SSH support already enabled".to_string());
    }
    
    // Add enable-ssh-support
    if !content.ends_with('\n') && !content.is_empty() {
        content.push('\n');
    }
    content.push_str("enable-ssh-support\n");
    
    // Write back
    fs::write(&conf_path, content)?;
    
    Ok(format!("Added enable-ssh-support to {}", conf_path.display()))
}

/// Get the GPG agent SSH socket path
pub fn get_gpg_ssh_socket() -> Result<String> {
    let output = Command::new("gpgconf")
        .arg("--list-dirs")
        .arg("agent-ssh-socket")
        .output()?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        anyhow::bail!("Failed to get GPG SSH socket path")
    }
}

/// Add SSH_AUTH_SOCK export to shell config
pub fn configure_shell_ssh() -> Result<String> {
    let socket_path = get_gpg_ssh_socket()?;

    // Reject paths that would be unsafe inside a double-quoted shell string.
    // `$` triggers variable/command expansion; `"` and `` ` `` break quoting.
    if socket_path.contains('"') || socket_path.contains('$') || socket_path.contains('`') {
        anyhow::bail!(
            "gpgconf returned a socket path with unsafe characters; aborting shell config write"
        );
    }

    let export_line = format!("export SSH_AUTH_SOCK=\"{}\"", socket_path);
    
    // Detect shell
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let rc_file = if shell.contains("zsh") {
        home.join(".zshrc")
    } else {
        home.join(".bashrc")
    };
    
    // Check if already configured
    if rc_file.exists() {
        let content = fs::read_to_string(&rc_file)?;
        if content.contains("SSH_AUTH_SOCK") && content.contains("gpg-agent") {
            return Ok(format!("SSH_AUTH_SOCK already configured in {}", rc_file.display()));
        }
    }
    
    // Append configuration
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&rc_file)?;
    
    writeln!(file)?;
    writeln!(file, "# GPG SSH agent configuration (added by YubiTUI)")?;
    writeln!(file, "{}", export_line)?;
    
    Ok(format!("Added SSH_AUTH_SOCK to {}", rc_file.display()))
}

/// Restart GPG agent
pub fn restart_gpg_agent() -> Result<String> {
    let output = Command::new("gpgconf")
        .arg("--kill")
        .arg("gpg-agent")
        .output()?;
    
    if output.status.success() {
        // Launch it again
        Command::new("gpgconf")
            .arg("--launch")
            .arg("gpg-agent")
            .output()?;
        
        Ok("GPG agent restarted successfully".to_string())
    } else {
        Ok("Failed to restart GPG agent".to_string())
    }
}

/// Export SSH public key and save to file
#[allow(dead_code)]
pub fn export_ssh_key_to_file(path: &PathBuf) -> Result<String> {
    let ssh_key = crate::yubikey::key_operations::export_ssh_public_key()?;
    
    fs::write(path, ssh_key)?;
    
    Ok(format!("SSH public key saved to {}", path.display()))
}

/// Add SSH public key to authorized_keys on remote server
#[allow(dead_code)]
pub fn add_to_remote_authorized_keys(ssh_key: &str, user: &str, host: &str) -> Result<String> {
    validate_ssh_target(user, host)?;

    // Pass the key via stdin to avoid shell injection — never interpolate key material into a shell command
    let mut child = Command::new("ssh")
        .arg("-l").arg(user)
        .arg("--")
        .arg(host)
        .arg("mkdir -p ~/.ssh && cat >> ~/.ssh/authorized_keys")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        writeln!(stdin, "{}", ssh_key)?;
    }

    let output = child.wait()?;

    if output.success() {
        Ok("SSH key added to remote authorized_keys".to_string())
    } else {
        Ok("Failed to add SSH key to remote server".to_string())
    }
}

/// Test SSH connection
pub fn test_ssh_connection(user: &str, host: &str) -> Result<String> {
    validate_ssh_target(user, host)?;

    let mut child = Command::new("ssh")
        .arg("-l").arg(user)
        .arg("--")
        .arg(host)
        .arg("echo 'SSH connection successful'")
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    let output = child.wait()?;

    if output.success() {
        Ok("SSH connection test successful".to_string())
    } else {
        Ok("SSH connection test failed".to_string())
    }
}

/// Reject user/host values that could be used for SSH option injection.
/// Passed as separate arguments (not through a shell), so spaces are harmless,
/// but a leading `-` would be interpreted as an SSH flag.
fn validate_ssh_target(user: &str, host: &str) -> Result<()> {
    if user.is_empty() || host.is_empty() {
        anyhow::bail!("user and host must not be empty");
    }
    if user.starts_with('-') || host.starts_with('-') {
        anyhow::bail!("Invalid user or host: must not start with '-'");
    }
    let user_ok = user.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.');
    let host_ok = host.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '.' || c == ':' || c == '[' || c == ']');
    if !user_ok {
        anyhow::bail!("Invalid characters in username");
    }
    if !host_ok {
        anyhow::bail!("Invalid characters in hostname");
    }
    Ok(())
}

fn get_gpg_agent_conf_path() -> Result<PathBuf> {
    let gnupg_home = if let Ok(home) = std::env::var("GNUPGHOME") {
        PathBuf::from(home)
    } else {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
            .join(".gnupg")
    };
    
    // Create .gnupg directory if it doesn't exist
    if !gnupg_home.exists() {
        fs::create_dir_all(&gnupg_home)?;
        // Set proper permissions (700)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o700);
            fs::set_permissions(&gnupg_home, perms)?;
        }
    }
    
    Ok(gnupg_home.join("gpg-agent.conf"))
}
