# yubitui

## What This Is

A terminal user interface (TUI) for YubiKey management, written in Rust. Provides comprehensive YubiKey operations through a keyboard-driven interface: diagnostics, PIN management, key import/generation, SSH setup wizard, touch policy, attestation, and PIV certificates — all via native PC/SC APDUs without requiring ykman or gpg --card-status.

## Why It Exists

Managing YubiKeys currently requires juggling multiple CLI tools (`gpg`, `ykman`, `gpgconf`, `ssh-add`) with cryptic commands. yubitui puts everything in one place with guided workflows and real-time status.

## Who It's For

Developers and security-conscious users who use YubiKeys for SSH authentication and GPG signing — primarily on Linux and macOS, with Windows support required.

## Core Value

Zero-friction YubiKey management: detect problems automatically, guide users through fixes, expose all operations without requiring memorization of CLI incantations.

## Platform Requirement

**Must be cross-platform: Linux, macOS, Windows. No exceptions.**
All diagnostics, hints, file paths, and operations must be platform-aware.

## Current Milestone: v1.1 Accessible to New Users

**Goal:** Make yubitui approachable for new users — working mouse support, feature parity with Yubico Authenticator, in-TUI education explaining every protocol, and a clean Model/View architecture ready for TUI library swap and Tauri GUI.

**Target features:**
- Full mouse support (click navigation, button interaction, scroll)
- UI/data architectural separation (Model/View split — no ratatui in business logic; Tauri-ready)
- Tmux-based E2E test suite (TDD; features verified before user sees them)
- Feature parity with Yubico Authenticator (TOTP/HOTP, FIDO/FIDO2, OTP slots, PIV improvements)
- In-TUI conceptual explanations: PIV, FIDO, FIDO2, OpenPGP/PGP, SSH, TOTP, HOTP/OTP
- New user onboarding flow — guided first-time setup

## Current State (as of 2026-03-26 — v1.0 shipped)

**v1.0 shipped.** All 5 phases complete, 21 plans executed.

### Shipped in v1.0

- YubiKey detection via native PC/SC reader enumeration (no ykman)
- Dashboard with live status and context menu (m/Enter opens popup, arrow/mouse nav)
- Full diagnostics screen (gpg-agent, pcscd, scdaemon, SSH agent)
- PIN management: change user/admin PIN, set reset code, unblock (all programmatic, no terminal escape)
- PIN unblock wizard: 4-branch decision tree (reset code / admin PIN / factory reset)
- Key operations: view card status, import key to card, generate on-device (7-step wizard), export SSH public key
- Key attribute display: algorithm type per slot (SIG/ENC/AUT) via native PC/SC GET DATA
- SSH pubkey popup: view/copy SSH public key without leaving TUI
- SSH wizard: enable SSH support, configure shell rc, restart agent, export key, test connection
- Mouse support: scroll navigation in list screens, click to close menus
- gnupg_home fix: uses gpgconf as authoritative source with Windows/GPG4Win fallback
- Reusable popup widget system: render_popup, render_confirm_dialog, render_context_menu
- CLI flags: `--check`, `--list`, `--debug`
- Security hardening: no flag injection, no shell injection, no sensitive values in logs
- 87 unit tests — all parser functions tested with fixture data (no hardware required)
- Touch policy: view per slot, set with IRREVERSIBLE warning, native PC/SC PUT DATA
- Attestation: verify on-device key generation, PEM popup via native ATTEST APDU (0xFB)
- Multi-key: Tab cycling between connected YubiKeys, dashboard shows Key X/Y indicator
- CI: 3-OS matrix (Linux/macOS/Windows) with clippy; tag-triggered release binary builds
- **Native PC/SC protocol**: card.rs module, BER-TLV parser, T=0 GET RESPONSE chaining, zero ykman dependency
- **PIV screen**: Screen::Piv via key '6' and dashboard menu, renders 9a/9c/9d/9e slot occupancy

## Requirements

### Validated

- ✓ Cross-platform support (Linux, macOS, Windows) — v1.0
- ✓ YubiKey detection without holding card lock — v1.0
- ✓ PIN retry counter display and lock detection — v1.0
- ✓ Programmatic PIN management (no terminal escape) — v1.0
- ✓ SSH wizard guiding users through gpg-agent SSH setup — v1.0
- ✓ Key import and generation (7-step wizard) — v1.0
- ✓ SSH public key export — v1.0
- ✓ System diagnostics with platform-appropriate fix suggestions — v1.0
- ✓ Touch policy configuration (view and set per slot) — v1.0
- ✓ Multiple YubiKey support (detect and Tab-switch) — v1.0
- ✓ Attestation support (on-device key verification) — v1.0
- ✓ Unit tests for all parsers — v1.0 (87 tests)
- ✓ CI 3-OS matrix + release builds — v1.0
- ✓ Native PC/SC protocol (zero ykman/gpg-card dependency) — v1.0
- ✓ PIV certificates screen — v1.0

### Active

- ✓ Full mouse support (click navigation + scroll — region-based dispatch) — Phase 07
- ✓ Model/View architectural separation (no ratatui in business logic) — Phase 06
- ✓ Tmux-based E2E test suite (6 smoke tests + 15 insta snapshots) — Phase 07
- [ ] Feature parity with Yubico Authenticator (TOTP/HOTP, FIDO/FIDO2, OTP slots)
- [ ] In-TUI protocol education (PIV, FIDO, FIDO2, OpenPGP, SSH, TOTP, OTP/HOTP)
- [ ] New user onboarding flow
- [ ] Backup/restore workflows
- [ ] cargo fmt compliance (tech debt from v1.0)
- [ ] 50ms sleep after kill_scdaemon() for Linux Card Busy robustness

### Out of Scope

- GUI (non-TUI) interface — terminal-first, always
- FIDO2/WebAuthn operations — handled better by browser/ykman
- Key material backup to cloud — security boundary
- Reactive ratatui rendering engine (future milestone — app.rs componentization)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Native PC/SC via pcsc crate (no ykman) | Eliminates binary dependency; works on clean systems | ✓ Validated — core architecture of v1.0 |
| gpg remains for keyring operations only | gpg handles GPG keyring; card reads all native | ✓ Validated — clean separation |
| Kill scdaemon before exclusive card access | Avoids SW 0x6B00 contention on shared card channel | ✓ Validated — required on all platforms |
| T=0 GET RESPONSE chaining in get_data() | Multi-part card responses on YubiKey 5.4.x | ✓ Validated — fingerprint reads work |
| 0x71 outer TLV unwrap for GET_DEVICE_INFO | ykman does Tlv.unpack(0x71) first — we must too | ✓ Validated — fixes NEO misidentification |
| --pinentry-mode loopback for PIN ops | Non-interactive gpg PIN input without terminal escape | ✓ Validated — all PIN ops in-TUI |
| 7-step wizard for key generation | Complex multi-step flow needs guided UX | ✓ Validated — usable by non-experts |
| previous_screen field for modal overlays | Enables return navigation from help/attestation/etc. | ✓ Validated — uniform pattern |
| Vec<YubiKeyState> with selected index | Multi-key support without breaking single-key UX | ✓ Validated — Tab switching works |
| gpgconf --list-dirs homedir as authoritative path | Handles Windows GPG4Win, non-standard installs | ✓ Validated — SSH fix correct |
| Log to temp dir instead of /tmp | /tmp doesn't exist on Windows | ✓ Validated |
| Security: no sensitive values in logs | Serial numbers, PINs, key material never logged | ✓ Validated |

## Context

**Stack:** Rust, ratatui 0.29, pcsc crate, tokio (minimal), GitHub Actions
**LOC:** ~10,053 Rust (112 files, 87 tests)
**Shipped:** v1.0 on 2026-03-26 (3-day sprint, 168 commits)
**CI:** Linux/macOS/Windows matrix, clippy -D warnings enforced, tag-triggered releases

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-03-27 — Phase 07 complete (mouse support + E2E harness)*
