# yubitui

## What This Is

A terminal user interface (TUI) for YubiKey management, written in Rust on the textual-rs component framework. Provides comprehensive YubiKey operations through a keyboard-driven interface: OATH/TOTP credentials, FIDO2 passkeys and PIN management, OTP slot inspection, PIV certificates, OpenPGP key management, diagnostics, PIN management, SSH setup, touch policy, attestation, and targeted slot deletion — all via native PC/SC APDUs without requiring ykman or gpg --card-status. New users are guided by an onboarding checklist when a factory-default YubiKey is detected.

## Why It Exists

Managing YubiKeys currently requires juggling multiple CLI tools (`gpg`, `ykman`, `gpgconf`, `ssh-add`) with cryptic commands. yubitui puts everything in one place with guided workflows and real-time status.

## Who It's For

Developers and security-conscious users who use YubiKeys for SSH authentication, GPG signing, TOTP codes, and FIDO2 — primarily on Linux and macOS, with Windows support required.

## Core Value

Zero-friction YubiKey management: detect problems automatically, guide users through fixes, expose all operations without requiring memorization of CLI incantations.

## Platform Requirement

**Must be cross-platform: Linux, macOS, Windows. No exceptions.**
All diagnostics, hints, file paths, and operations must be platform-aware.

## Current State (as of 2026-03-29 — v1.1 shipped)

**v1.1 shipped.** All 9 v1.1 phases complete, 35 plans executed.

### Shipped in v1.1 (Phases 6–13)

- Model/View architectural split: `src/model/` zero ratatui, `src/tui/` all rendering, CI lint boundary enforced
- Full textual-rs migration: all screens as components with rule-of-thirds layout, Footer keybindings, Button widgets, configurable themes
- OATH/TOTP screen: live codes, countdown timer, add/delete wizard, password-protected vault
- FIDO2 screen: PIN management, resident credential list/delete, factory reset via CTAPHID
- OTP Slots screen: slot occupancy display (read-only; write deferred)
- Per-screen `?` help panels and protocol glossary
- Factory-default detection and onboarding checklist for new users
- OpenPGP individual key slot deletion (Admin PIN + RSA attribute trick)
- PIV cert/key deletion (3DES management key auth, firmware 5.7+ gate for key delete)
- DataTable, Button, ProgressBar, Markdown widgets on every screen; consistent status badges
- Post-delete and on-demand refresh on all screens (R key)
- 161 unit/snapshot tests — all hardware paths mockable

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
- ✓ Full mouse support (click navigation + scroll — region-based dispatch) — v1.1 Phase 07
- ✓ Model/View architectural separation (no ratatui in business logic) — v1.1 Phase 06
- ✓ E2E test suite (15 insta snapshot tests + Pilot integration tests) — v1.1 Phase 07
- ✓ textual-rs component migration — all screens rebuilt — v1.1 Phase 08
- ✓ OATH/TOTP credential management with live codes — v1.1 Phase 09
- ✓ FIDO2 PIN management and resident credential management — v1.1 Phase 10
- ✓ OTP slot read-only inspection — v1.1 Phase 11
- ✓ Per-screen help panels and protocol glossary — v1.1 Phase 11
- ✓ New user onboarding checklist (factory-default detection) — v1.1 Phase 11
- ✓ OpenPGP individual key slot deletion — v1.1 Phase 12
- ✓ PIV cert/key deletion with management key auth — v1.1 Phase 12
- ✓ Consistent DataTable/Button/Badge UI across all screens — v1.1 Phase 13

### Active

- [ ] Outcome-oriented provisioning wizards (SSH+touch, initial YubiKey setup) — backlog 999.1
- [ ] OTP slot write (configure Yubico OTP, static password, HMAC-SHA1) — high risk, deferred
- [ ] PIV cert view (decoded X.509) — deferred
- [ ] PIV management key change — deferred
- [ ] OATH application password set/change — deferred
- [ ] OATH URI import (otpauth://) — deferred
- [ ] Backup/restore workflows — deferred to v2

### Out of Scope

- GUI (non-TUI) interface — terminal-first until Tauri milestone
- FIDO2 via PC/SC — requires HID FIDO transport (0xF1D0), not CCID
- Key material backup to cloud — security boundary
- FIDO2/WebAuthn browser operations — out of scope for TUI

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Native PC/SC via pcsc crate (no ykman) | Eliminates binary dependency; works on clean systems | ✓ Validated — core architecture of v1.0 |
| gpg remains for keyring operations only | gpg handles GPG keyring; card reads all native | ✓ Validated — clean separation |
| Kill scdaemon before exclusive card access | Avoids SW 0x6B00 contention on shared card channel | ✓ Validated — required on all platforms |
| T=0 GET RESPONSE chaining in get_data() | Multi-part card responses on YubiKey 5.4.x | ✓ Validated — fingerprint reads work |
| --pinentry-mode loopback for PIN ops | Non-interactive gpg PIN input without terminal escape | ✓ Validated — all PIN ops in-TUI |
| textual-rs via git dep (jabberwock/textual-rs) | Not on crates.io yet; switch to registry when published | ✓ Working — upgrade to registry version when available |
| src/model/ zero ratatui imports, CI lint enforced | Enables Tauri GUI layer without code changes | ✓ Validated — boundary holds in v1.1 |
| AppState with serde::Serialize on all model types | Downstream Tauri layer can consume without code changes | ✓ Validated — all model types serialize |
| OpenPGP slot delete via RSA attribute trick | No DELETE KEY APDU in OpenPGP card spec | ✓ Validated — destroys key material correctly |
| PIV key delete MOVE KEY INS=0xF6, firmware 5.7+ only | Older firmware has no key delete | ✓ Validated — firmware gate shows clear message |
| 3DES challenge-response for PIV management key auth | des 0.9.0-rc.3 with cipher 0.5 required | ✓ Validated — auth works |
| Reset FIDO2 via raw CTAPHID frames (hidapi) | ctap-hid-fido2 doesn't expose authenticatorReset (0x07) | ✓ Validated |
| Factory-default detection: no FIDO2 PIN + 0 OATH creds + default PIV mgmt key | Heuristic with no extra PC/SC calls | ✓ Validated — detects new YubiKeys reliably |
| PinInputWidget fields as direct children (not Vertical-wrapped) | Vertical{height:1fr} collapses to 0 in screen-stack | ✓ Fixed in f2bc499 |

## Context

**Stack:** Rust, textual-rs (git dep), pcsc crate, hidapi, des 0.9.0-rc.3, chrono, GitHub Actions
**LOC:** ~14,000 Rust (161 tests)
**Shipped:** v1.0 on 2026-03-26, v1.1 on 2026-03-29
**CI:** Linux/macOS/Windows matrix, clippy -D warnings enforced, tag-triggered releases
**Next:** v1.2 — provisioning wizards, OTP write, PIV improvements (plan via `/gsd:new-milestone`)

## Tauri Future

Business logic is in `src/model/` with zero ratatui/textual-rs imports. All model types implement `serde::Serialize`. A Tauri GUI layer can consume `src/model/` without modification when that milestone arrives.

## Evolution

This document evolves at phase transitions and milestone boundaries.

---
*Last updated: 2026-03-29 after v1.1 milestone — Accessible to New Users*
