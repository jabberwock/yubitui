# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project structure
- YubiKey detection via PC/SC
- Multi-model support (YubiKey 5, 4, NEO)
- System diagnostics (gpg-agent, pcscd, scdaemon, SSH agent)
- Dashboard with quick status overview
- PIN management view with retry counter display
- Key management view (OpenPGP and PIV)
- SSH setup wizard placeholder
- Comprehensive documentation (README, CONTRIBUTING)

### Architecture
- Ratatui-based TUI with Crossterm backend
- Modular design with clear separation of concerns
- Direct PC/SC communication for low-level operations
- GPG CLI integration for card editing operations
- Type-safe Rust implementation with comprehensive error handling

## [0.1.0] - TBD

Initial release - Foundation milestone

- Basic YubiKey detection and model identification
- System diagnostics for common configuration issues
- PIN status monitoring
- Foundation for future key management features
