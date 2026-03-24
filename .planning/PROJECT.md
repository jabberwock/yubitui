# yubitui

## What This Is

A terminal user interface (TUI) for YubiKey management, written in Rust. Provides comprehensive YubiKey operations through an intuitive keyboard-driven interface: diagnostics, PIN management, key import/generation, and SSH setup wizard.

## Why It Exists

Managing YubiKeys currently requires juggling multiple CLI tools (`gpg`, `ykman`, `gpgconf`, `ssh-add`) with cryptic commands. yubitui puts everything in one place with guided workflows and real-time status.

## Who It's For

Developers and security-conscious users who use YubiKeys for SSH authentication and GPG signing — primarily on Linux and macOS, with Windows support required.

## Core Value

Zero-friction YubiKey management: detect problems automatically, guide users through fixes, expose all operations without requiring memorization of CLI incantations.

## Platform Requirement

**Must be cross-platform: Linux, macOS, Windows. No exceptions.**
All diagnostics, hints, file paths, and operations must be platform-aware.

## Current State (as of 2026-03-24)

### Done
- YubiKey detection via `gpg --card-status` and PC/SC
- Dashboard with live status
- Full diagnostics screen (gpg-agent, pcscd, scdaemon, SSH agent)
- PIN management: change user/admin PIN, set reset code, unblock
- Key operations: view card status, import key to card, generate on-device, export SSH public key
- SSH wizard: enable SSH support, configure shell rc, restart agent, export key, test connection
- CLI flags: `--check`, `--list`, `--debug`
- Security hardening: no flag injection, no shell injection, no sensitive values in logs

### Known Gaps
- Key selection UI: import always uses first key in list (no picker)
- `?` help key documented in README but not wired up
- Phase 4 features: touch policy, attestation, multi-YubiKey, backup/restore
- README roadmap checkboxes are stale

## Requirements

### Validated
- Cross-platform support (Linux, macOS, Windows)
- YubiKey detection without holding card lock
- PIN retry counter display and lock detection
- Interactive PIN management via gpg --card-edit
- SSH wizard guiding users through gpg-agent SSH setup
- Key import and generation
- SSH public key export
- System diagnostics with platform-appropriate fix suggestions

### Active
- [ ] Key selection UI for import operation
- [ ] Help screen (`?` key)
- [ ] Touch policy configuration
- [ ] Multiple YubiKey support
- [ ] Backup/restore workflows
- [ ] Attestation support

### Out of Scope
- GUI (non-TUI) interface — terminal-first, always
- FIDO2/WebAuthn operations — handled better by browser/ykman
- Key material backup to cloud — security boundary

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Use gpg CLI instead of openpgp-card crate directly | Avoids card lock contention; gpg handles session state | Validated |
| Log to temp dir instead of /tmp | /tmp doesn't exist on Windows | Validated |
| cfg! macros for platform hints | Compile-time, zero runtime overhead | Validated |
| Security: no sensitive values in logs | Serial numbers, PINs, key material never logged | Validated |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-03-24 after initialization*
