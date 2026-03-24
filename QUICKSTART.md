# YubiTUI Quick Reference

## Installation

```bash
cargo install --path .
# or
cargo build --release
./target/release/yubitui
```

## Command Line

```bash
yubitui              # Launch interactive TUI
yubitui --list       # List detected YubiKeys
yubitui --check      # Run system diagnostics
yubitui --debug      # Enable verbose logging
yubitui --help       # Show help
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `1` | Dashboard |
| `2` | System Diagnostics |
| `3` | Key Management |
| `4` | PIN Management |
| `5` | SSH Setup Wizard |
| `R` | Refresh YubiKey state |
| `Q` / `Esc` | Quit / Go back |

## System Requirements

### macOS
- **PC/SC**: Built-in (CryptoTokenKit)
- **GPG**: `brew install gnupg`
- **YubiKey Manager** (optional): `brew install ykman`

### Linux
- **PC/SC**: `sudo apt-get install pcscd pcsc-tools`
- **GPG**: `sudo apt-get install gnupg`
- **Start pcscd**: `sudo systemctl start pcscd`

### Windows
- **PC/SC**: Built-in (SCardSvr)
- **GPG**: Install from [gnupg.org](https://gnupg.org/download/)

## Troubleshooting

### YubiKey Not Detected
1. Unplug and replug the YubiKey
2. Check PC/SC daemon: `yubitui --check`
3. List readers: `pcsc_scan`
4. Try debug mode: `yubitui --debug --list`

### PIN Locked
- **User PIN locked**: Use Admin PIN or Reset Code to unblock
- **Admin PIN locked**: Factory reset required (DESTROYS ALL KEYS)
- **Prevention**: Change from default PINs immediately

### GPG Agent Issues
```bash
# Restart gpg-agent
gpgconf --kill gpg-agent
gpgconf --launch gpg-agent

# Check status
gpg --card-status
```

### SSH Not Working
1. Enable SSH support in `~/.gnupg/gpg-agent.conf`:
   ```
   enable-ssh-support
   ```
2. Set SSH_AUTH_SOCK:
   ```bash
   # Add to ~/.bashrc or ~/.zshrc
   export SSH_AUTH_SOCK=$(gpgconf --list-dirs agent-ssh-socket)
   ```
3. Restart gpg-agent: `gpgconf --kill gpg-agent`

## Default PINs

⚠️ **Change these immediately!**

- **User PIN**: `123456`
- **Admin PIN**: `12345678`
- **Reset Code**: Not set by default

## Common Tasks

### View Logs

When running the TUI, logs are written to `/tmp/yubitui.log`:

```bash
# Watch logs in real-time
tail -f /tmp/yubitui.log

# View recent logs
tail -50 /tmp/yubitui.log
```

### View Card Status
```bash
gpg --card-status
```

### Change PIN
```bash
gpg --card-edit
> admin
> passwd
> 1  # Change User PIN
> q
```

### Import Key to YubiKey
```bash
gpg --edit-key <KEY_ID>
> keytocard
```

### Export SSH Public Key
```bash
# Using YubiKey's authentication key
ssh-add -L
```

### Reset YubiKey (DESTRUCTIVE)
```bash
ykman openpgp reset
```

## Tips

1. **Backup Keys**: Before moving keys to YubiKey, ensure you have backups
2. **Multiple YubiKeys**: Keep a backup YubiKey with the same encryption key
3. **Touch Policy**: Enable touch requirement for additional security
4. **PIN Retries**: Default is 3 attempts before lock
5. **Text Selection**: Mouse capture is disabled - select/copy text normally

## Resources

- [Official YubiKey Guide](https://github.com/drduh/YubiKey-Guide)
- [Yubico Developers](https://developers.yubico.com/)
- [GnuPG Manual](https://gnupg.org/documentation/)
- [YubiTUI Issues](https://github.com/yourusername/yubitui/issues)
