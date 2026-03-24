# YubiTUI 🔐

**A blazingly fast, intelligent TUI for YubiKey management**

YubiTUI is a terminal user interface (TUI) written in Rust that provides comprehensive YubiKey management with a focus on SSH and GPG key operations. It intelligently detects configuration issues, guides users through secure setup, and provides full access to all YubiKey features.

## Features

### 🎯 Core Capabilities
- **Smart Diagnostics**: Automatically detects gpg-agent issues, configuration problems, and locked keys
- **PIN Management**: Check retry counters, detect locks, unblock PINs with admin PIN
- **Key Import/Generation**: Import existing keys or generate new ones directly on the YubiKey
- **SSH Configuration**: Guide users through the optimal setup for SSH authentication
- **Multi-Model Support**: Automatically detects YubiKey model and adapts features accordingly
- **Card Editing**: Full access to all `gpg --card-edit` functionality through an intuitive UI

### 🚀 Performance
- **Native Speed**: Written in Rust for sub-millisecond rendering
- **Efficient Backend**: Direct PC/SC communication via `pcsc` crate
- **Zero Overhead**: Immediate-mode rendering with minimal allocations

### 🧠 Intelligence
- **Configuration Analysis**: Detects missing or misconfigured gpg-agent, scdaemon, pcscd
- **Lock Detection**: Identifies PIN retry counter status and provides recovery options
- **Key Recognition**: Understands why keys aren't being detected and suggests fixes
- **Best Practices**: Recommends secure, efficient configurations for SSH usage

## Architecture

### Technology Stack
- **TUI Framework**: [Ratatui](https://ratatui.rs) with Crossterm backend
- **YubiKey Communication**: 
  - `yubikey` crate for PIV operations (via PC/SC)
  - `openpgp-card` crate for OpenPGP card operations
  - Direct `gpg` CLI integration for card editing
- **State Management**: Elm-inspired architecture with message passing

### Key Components
```
src/
├── main.rs              # Entry point, TUI initialization
├── app.rs               # Application state and event loop
├── ui/                  # UI rendering
│   ├── mod.rs
│   ├── dashboard.rs     # Main dashboard view
│   ├── diagnostics.rs   # Configuration diagnostics view
│   ├── keys.rs          # Key management view
│   ├── pin.rs           # PIN management view
│   └── ssh.rs           # SSH configuration wizard
├── yubikey/             # YubiKey operations
│   ├── mod.rs
│   ├── detection.rs     # Device detection and model info
│   ├── piv.rs           # PIV operations
│   ├── openpgp.rs       # OpenPGP card operations
│   ├── pin.rs           # PIN/PUK operations
│   └── ssh.rs           # SSH-specific operations
├── diagnostics/         # System diagnostics
│   ├── mod.rs
│   ├── gpg_agent.rs     # GPG agent detection/analysis
│   ├── scdaemon.rs      # Scdaemon configuration
│   ├── pcscd.rs         # PC/SC daemon status
│   └── ssh_agent.rs     # SSH agent configuration
└── utils/
    ├── mod.rs
    ├── gpg_cli.rs       # GPG CLI wrapper
    └── config.rs        # Configuration helpers
```

## Requirements

### Runtime Dependencies
- **GPG 2.1+**: For OpenPGP operations and card editing
- **PC/SC Lite**: For smart card communication (pcscd daemon)
- **YubiKey**: Firmware 4.0+ recommended (full feature support)

### Build Dependencies
- **Rust 1.75+**: Latest stable Rust toolchain
- **PC/SC Development Libraries**: 
  - macOS: `brew install pcsc-lite`
  - Linux: `apt-get install libpcsclite-dev` or equivalent
  - Windows: Windows SDK (pre-installed)

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/yubitui
cd yubitui

# Build and run
cargo run --release
```

## Usage

```bash
# Launch the TUI
yubitui

# Quick diagnostics
yubitui --check

# Show detected YubiKeys
yubitui --list
```

### Navigation
- `Tab` / `Shift+Tab`: Navigate between sections
- `←` `→` `↑` `↓`: Navigate within sections
- `Enter`: Select / Activate
- `Esc`: Go back / Cancel
- `q`: Quit
- `?`: Show help

## YubiKey Model Support

| Model | PIV | OpenPGP | FIDO2 | Firmware Detection |
|-------|-----|---------|-------|-------------------|
| YubiKey 5 Series | ✅ | ✅ | ✅ | ✅ |
| YubiKey 4 Series | ✅ | ✅ | ❌ | ✅ |
| YubiKey NEO | ⚠️ | ✅ | ❌ | ✅ |

⚠️ = Limited support

## Development

### Running Tests
```bash
# Run all tests
cargo test

# Run tests with a YubiKey connected (requires device)
cargo test --features device-tests -- --ignored

# Run with logging
RUST_LOG=debug cargo run
```

### Code Structure Philosophy
- **Separation of Concerns**: UI rendering separate from business logic
- **Type Safety**: Leverage Rust's type system to prevent invalid states
- **Error Handling**: Comprehensive error types with user-friendly messages
- **Testability**: Mock YubiKey operations for CI/CD testing

## Roadmap

### Phase 1: Core Functionality ✅
- [x] Project structure
- [ ] YubiKey detection
- [ ] Basic dashboard UI
- [ ] PIN retry counter display
- [ ] GPG agent diagnostics

### Phase 2: Key Management
- [ ] View existing keys
- [ ] Import keys via PIV
- [ ] Import keys via OpenPGP
- [ ] Generate keys on-device
- [ ] Key attribute configuration

### Phase 3: SSH Integration
- [ ] SSH configuration wizard
- [ ] SSH agent integration
- [ ] Public key export
- [ ] authorized_keys management

### Phase 4: Advanced Features
- [ ] Touch policy configuration
- [ ] Attestation support
- [ ] Multiple YubiKey support
- [ ] Backup/restore workflows

## Contributing

Contributions welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Security

⚠️ **IMPORTANT**: This tool handles sensitive cryptographic material. Always:
- Verify signatures on releases
- Backup your keys before any destructive operations
- Use the reset function only when you understand the consequences
- Change default PINs immediately after setup

Report security vulnerabilities to: security@example.com

## License

Apache-2.0 OR MIT

## Acknowledgments

- [Ratatui](https://ratatui.rs) - Excellent TUI framework
- [yubikey.rs](https://github.com/iqlusioninc/yubikey.rs) - YubiKey PIV driver
- [openpgp-card](https://codeberg.org/openpgp-card/openpgp-card) - OpenPGP card library
- [drduh's YubiKey Guide](https://github.com/drduh/YubiKey-Guide) - Comprehensive YubiKey documentation

## See Also

- [YubiKey Manager (ykman)](https://developers.yubico.com/yubikey-manager/) - Official Yubico CLI
- [gpg-card-automation](https://github.com/ixydo/gpg-smartcard-automation) - GPG smartcard automation scripts
