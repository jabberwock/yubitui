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

Phase 3 complete — all Milestone 1 phases shipped. v1.0 feature-complete.

### Done
- YubiKey detection via `gpg --card-status` and PC/SC
- Dashboard with live status and context menu (m/Enter opens popup, arrow/mouse nav)
- Full diagnostics screen (gpg-agent, pcscd, scdaemon, SSH agent)
- PIN management: change user/admin PIN, set reset code, unblock
- PIN unblock wizard: 4-branch decision tree (reset code / admin PIN / factory reset)
- Key operations: view card status, import key to card, generate on-device, export SSH public key
- Key attribute display: algorithm type per slot (SIG/ENC/AUT) via ykman
- SSH pubkey popup: view/copy SSH public key without leaving TUI
- SSH wizard: enable SSH support, configure shell rc, restart agent, export key, test connection
- Mouse support: scroll navigation in list screens, click to close menus
- gnupg_home fix: uses gpgconf as authoritative source with Windows/GPG4Win fallback
- Reusable popup widget system: render_popup, render_confirm_dialog, render_context_menu
- CLI flags: `--check`, `--list`, `--debug`
- Security hardening: no flag injection, no shell injection, no sensitive values in logs
- **[Phase 3]** 36 unit tests — all parser functions pub and tested with fixture data
- **[Phase 3]** Touch policy: view per slot, set with IRREVERSIBLE warning, `ykman openpgp keys set-touch`
- **[Phase 3]** Attestation: verify on-device key generation, PEM popup via `ykman openpgp keys attest`
- **[Phase 3]** Multi-key: Tab cycling between connected YubiKeys, dashboard shows Key X/Y indicator
- **[Phase 3]** CI: 3-OS matrix (Linux/macOS/Windows) with clippy; tag-triggered release binary builds

### Known Gaps
- 02-04 dashboard context menu (visual polish plan was deferred)
- Human UAT needed for hardware-dependent features (touch policy set, attestation, multi-key Tab)

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
- [ ] Backup/restore workflows
- [ ] 02-04 dashboard context menu and visual polish (deferred)

### Validated (Phase 3)
- Touch policy configuration (view and set per slot) — Validated in Phase 3
- Multiple YubiKey support (detect and Tab-switch) — Validated in Phase 3
- Attestation support (on-device key verification) — Validated in Phase 3
- Unit tests for all parsers — Validated in Phase 3 (36 tests)
- CI 3-OS matrix + release builds — Validated in Phase 3

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
*Last updated: 2026-03-24 after Phase 3 completion (Milestone 1 complete)*
