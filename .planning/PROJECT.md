# yubitui

## What This Is

A terminal user interface (TUI) for YubiKey management, written in Rust. Provides comprehensive YubiKey operations through a keyboard-driven interface: diagnostics, PIN management, key import/generation, SSH setup wizard, OATH/TOTP live codes, FIDO2 passkey management, OTP slot view, PIV certificates, touch policy, attestation, and new-user onboarding — all via native PC/SC APDUs without requiring ykman or gpg --card-status. Built on textual-rs component model with Pilot-based snapshot testing.

## Why It Exists

Managing YubiKeys currently requires juggling multiple CLI tools (`gpg`, `ykman`, `gpgconf`, `ssh-add`) with cryptic commands. yubitui puts everything in one place with guided workflows and real-time status.

## Who It's For

Developers and security-conscious users who use YubiKeys for SSH authentication and GPG signing — primarily on Linux and macOS, with Windows support required.

## Core Value

Zero-friction YubiKey management: detect problems automatically, guide users through fixes, expose all operations without requiring memorization of CLI incantations.

## Platform Requirement

**Must be cross-platform: Linux, macOS, Windows. No exceptions.**
All diagnostics, hints, file paths, and operations must be platform-aware.

## Current State (as of 2026-03-29 — v1.1 shipped)

**v1.1 shipped.** All 8 phases complete (phases 6–13), 34 plans executed.

### Shipped in v1.0 (2026-03-26)

- YubiKey detection via native PC/SC reader enumeration (no ykman)
- Dashboard with live status and context menu
- Full diagnostics screen (gpg-agent, pcscd, scdaemon, SSH agent)
- PIN management: change user/admin PIN, set reset code, unblock
- Key operations: view, import, generate (7-step wizard), export SSH pubkey
- SSH wizard: enable SSH support, configure shell rc, restart agent
- Touch policy (view and set per slot), Attestation (PEM popup)
- Multi-key: Tab cycling between connected YubiKeys
- PIV certificates screen (9a/9c/9d/9e slot occupancy)
- CI: 3-OS matrix (Linux/macOS/Windows) + tag-triggered release builds
- Native PC/SC protocol: zero ykman dependency

### Shipped in v1.1 (2026-03-29)

- **textual-rs migration**: All 7 screens rebuilt as textual-rs components — Header/Footer, Button widgets, DataTable, ProgressBar, Markdown. Pilot-based snapshot tests replace tmux harness.
- **Model/View separation**: `src/model/` has zero ratatui imports; `src/tui/` renders only. CI lint enforces boundary. All model types `serde::Serialize`.
- **Mouse support**: Region-based click dispatch (reverse iteration for popup-first), scroll on all list screens, Windows ConPTY graceful degradation.
- **OATH/TOTP screen**: Live TOTP codes with countdown ProgressBar, Add Account wizard, Delete confirmation, OATH password prompt on SW 0x6982.
- **FIDO2 screen**: PIN set/change, resident credential list, per-credential delete, factory reset with 10s timing guidance, Windows admin privilege notice.
- **OTP slots screen**: Slot 1/2 status (Occupied/Empty/type), hardware write-only note, Refresh button.
- **Education system**: Per-screen `?` help panels on all 8 screens, protocol Glossary (PIV/FIDO/FIDO2/OpenPGP/SSH/TOTP/OTP), Markdown rendering via textual-rs.
- **Onboarding flow**: Factory-default detection heuristic (no FIDO2 PIN + no OATH credentials + no PIV certs), OnboardingScreen guides initial setup.
- **Slot delete workflows**: OpenPGP slot delete via Admin PIN + RSA attribute-change trick (PUT DATA RSA4096→RSA2048). PIV cert delete (PUT DATA empty 0x53), PIV key delete (MOVE KEY INS=0xF6, firmware 5.7+ only). Management key 3DES auth via `des` crate.
- **UI polish**: All screens use DataTable for tabular data, Button widgets for actions, consistent `[OK]`/`[SET]`/`[EMPTY]`/`[BLOCKED]` status badges, Header→data→spacer→Buttons→Footer layout everywhere.
- **160 tests** — Pilot snapshot tests for all screens, no hardware required.

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
- ✓ Full mouse support (click navigation + scroll — region-based dispatch) — v1.1
- ✓ Model/View architectural separation (no ratatui in business logic) — v1.1
- ✓ Snapshot test suite (160 Pilot tests, no hardware required) — v1.1
- ✓ Feature parity with Yubico Authenticator (OATH/TOTP, FIDO2, OTP slots) — v1.1
- ✓ In-TUI protocol education (per-screen help + Glossary) — v1.1
- ✓ New user onboarding flow (factory-default detection) — v1.1
- ✓ Individual slot delete (OpenPGP + PIV cert + PIV key) — v1.1
- ✓ UI polish (DataTable, Button, status badges, consistent layout) — v1.1

### Active

- [ ] PIV cert view (X.509 decode via x509-parser) — v2 candidate
- [ ] PIV management key change — v2 candidate
- [ ] OATH application password set/change — v2 candidate
- [ ] OATH import via otpauth:// URI — v2 candidate
- [ ] Provisioning wizards — outcome-oriented multi-step flows (backlog 999.1)
- [ ] FIDO2 fingerprint management (Bio series only) — v2 candidate

### Out of Scope

- GUI (non-TUI) interface — terminal-first; Tauri GUI is v2 possibility when model layer is stable
- OTP slot write — underdocumented HID frame protocol and access code complexity deferred to v2
- Backup/restore workflows — deferred to v2
- FIDO2 fingerprint management — Bio series only, niche hardware
- Application enable/disable toggle — enterprise niche
- Reactive ratatui rendering engine — future milestone

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
| textual-rs component model replacing raw ratatui | Enables consistent layout, Pilot tests, widget reuse | ✓ Validated — 160 tests, 7 screens clean |
| Pilot snapshot tests replacing tmux E2E harness | Faster, deterministic, no process spawn overhead | ✓ Validated — all coverage in cargo test |
| src/model/ zero ratatui boundary enforced by CI lint | Enables Tauri GUI without rearchitecting | ✓ Validated — boundary clean in v1.1 |
| DataTable::new(columns) + add_row() API | textual-rs actual API differs from docs at planning time | ✓ Adapted — all screens use correct API |
| OATH countdown computed per-render from chrono::Utc::now() | No background timer thread needed | ✓ Validated — textual-rs re-renders on key events |
| OpenPGP slot delete via RSA attribute-change trick | No DELETE KEY APDU in OpenPGP card spec | ✓ Validated — PUT DATA RSA4096→RSA2048 destroys key |
| PIV key delete via MOVE KEY INS=0xF6 (firmware 5.7+ only) | Standard PIV doesn't have key delete | ✓ Validated — firmware gate UX is clear |
| des 0.9.0-rc.3 with cipher 0.5 (not cipher 0.4) | Pre-release cipher version incompatibility | ✓ Resolved — correct dep pinned |
| Factory-default heuristic uses model data only | Avoids double scdaemon kill at startup (Pitfall 5) | ✓ Validated — PIV management key auth deferred to v2 |

## Context

**Stack:** Rust, ratatui 0.30, textual-rs 0.3.11, pcsc crate, GitHub Actions
**LOC:** ~15,732 Rust (160 tests, 8 screens, all cross-platform)
**Shipped:** v1.0 on 2026-03-26, v1.1 on 2026-03-29 (3-day sprint, ~200 commits)
**CI:** Linux/macOS/Windows matrix, clippy -D warnings enforced, tag-triggered releases
**Testing:** 160 Pilot snapshot tests (no hardware required), all in `cargo test`

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-03-29 after v1.1 milestone — Accessible to New Users*
