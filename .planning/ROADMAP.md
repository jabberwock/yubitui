# yubitui Roadmap

## Milestone 1: Production-Ready v1.0

Phases 1-3 are already implemented. Starting from Phase 1 to close remaining gaps.

---

## Phase 1: Polish & Cross-Platform Fixes

**Goal:** Fix known rough edges so the app works correctly on all platforms.

**Scope:**
- Key selection UI: replace hardcoded `available_keys[0]` with an interactive picker
- Wire up `?` help screen with keybinding reference
- Audit all hardcoded paths and platform-specific strings for Windows/Linux/macOS correctness
- Update README roadmap checkboxes to reflect actual implementation state

**Done when:** App runs correctly on Windows with accurate diagnostics, keys can be selected for import, help screen is accessible.

**Plans:** 3 plans

Plans:
- [x] 01-01-PLAN.md — Interactive key selection UI for import
- [x] 01-02-PLAN.md — Help screen with keybinding reference
- [x] 01-03-PLAN.md — Update README roadmap checkboxes

**Requirements:** [KEY-PICKER, HELP-SCREEN, README-SYNC]

---

## Phase 2: UX — Menus, Wizards & Bug Fixes

**Goal:** Make yubitui genuinely accessible to non-experts through guided wizards, polished UI, and fixed diagnostics.

**Scope:**
- Dropdown/context menus throughout the TUI
- Mouse support
- PIN unblock wizard: 4-branch decision tree (reset code -> admin PIN -> factory reset -> abort)
- SSH enable wizard: guide through gpg-agent.conf edit, agent restart, SSH_AUTH_SOCK setup
- Fix SSH detection false negative on Windows (wrong gnupg conf path)
- Key attribute display (read-only via ykman openpgp info)
- authorized_keys management (display and copy SSH public key)

**Done when:** A non-expert can unblock their PIN, enable SSH, and understand their key status without reading documentation.

**Plans:** 4 plans

Plans:
- [x] 02-01-PLAN.md — Popup widget, mouse support, gnupg path fix
- [x] 02-02-PLAN.md — PIN unblock wizard (4-branch decision tree)
- [x] 02-03-PLAN.md — Key attribute display and SSH pubkey popup
- [ ] 02-04-PLAN.md — Dashboard context menu and visual verification

**Requirements:** [MENU-01, MOUSE-01, PIN-WIZARD-01, SSH-FIX-01, KEY-ATTR-01, AUTHKEYS-01]

---

## Phase 3: Advanced YubiKey Features

**Goal:** Power-user features and release readiness.

**Scope:**
- Touch policy configuration (view and set touch policy per slot)
- Multiple YubiKey support (detect and switch between connected keys)
- Attestation support (verify key was generated on-device)
- Unit tests for all parsers (card status, PIN counter, PIV info)
- CI passes on Linux, macOS, Windows
- Release binary builds via GitHub Actions

**Done when:** `cargo test` passes with meaningful coverage, CI matrix is green, touch policy and attestation work.

**Plans:** 4/4 plans complete

Plans:
- [x] 03-01-PLAN.md — Parser unit tests (20 tests) + fingerprint slice safety fix
- [x] 03-02-PLAN.md — Touch policy and attestation backend modules with tests
- [x] 03-03-PLAN.md — Multi-key detection, App struct evolution, touch/attestation UI integration
- [x] 03-04-PLAN.md — CI 3-OS matrix + release binary workflow

---

---

## Phase 4: Programmatic Subprocess Control

**Goal:** Eliminate all interactive subprocess escapes. Every gpg and ykman operation stays inside the TUI — no terminal handoff, no "base menu with no indicator of next steps."

**Scope:**
- Replace `gpg --card-edit` (interactive) with `--command-fd 0 --status-fd 1 --passphrase-fd 0` for all PIN operations (change user/admin PIN, unblock, set reset code)
- Replace `gpg --card-edit` key generation flow with non-interactive equivalent
- Surface gpg status output (progress, errors, confirmations) as in-TUI feedback
- Audit all subprocess invocations: identify any remaining cases where control leaves the TUI

**Done when:** No operation causes the terminal to hand off to an external interactive process. All user feedback during operations is rendered inside the TUI.

**Plans:** 3/4 plans executed

Plans:
- [x] 04-01-PLAN.md — GPG status-fd parser, PIN input widget, progress popup (foundation)
- [x] 04-02-PLAN.md — Non-interactive PIN operations with in-TUI input and feedback
- [x] 04-03-PLAN.md — Key generation wizard and non-interactive import with auto-map
- [x] 04-04-PLAN.md — Audit and fix all remaining escape sites, remove deprecated functions

**Requirements:** [NO-ESCAPE-01, IN-TUI-FEEDBACK-01]

---

## Phase 5: Native Card Protocol (No External CLI Deps)

**Goal:** Replace all ykman CLI calls and gpg --card-status reads with direct PC/SC raw APDUs via the pcsc crate. gpg remains for keyring operations only.

**Scope:**
- Create PC/SC primitives module (card.rs) with connect, GET DATA, TLV parsing, error mapping
- Replace device detection (ykman list --serials) with PC/SC reader enumeration
- Replace card state reads (gpg --card-status) with GET DATA APDUs (0xC4, 0x6E, 0x65, 0x5F50, 0x5E)
- Replace touch policy get/set (ykman openpgp) with GET DATA/PUT DATA on DOs 0xD6-0xD9
- Replace PIV detection (ykman piv info) with native PIV AID SELECT + GET DATA
- Replace attestation (ykman openpgp keys attest) with native ATTEST APDU (0xFB)
- Remove find_ykman(), unused crates (openpgp-card, card-backend-pcsc, yubikey)

**Done when:** `cargo test` passes; app works on a clean system with only pcscd/PC/SC installed; no ykman binary required.

**Plans:** 3 plans

Plans:
- [ ] 05-01-PLAN.md — PC/SC primitives (card.rs) + replace detection, PIN, OpenPGP, key attribute reads
- [ ] 05-02-PLAN.md — Native touch policy, PIV detection, attestation via PC/SC APDUs
- [ ] 05-03-PLAN.md — Cleanup: remove find_ykman, unused crates, grep audit, human verify

**Requirements:** [NATIVE-PCSC-01, NO-GPG-BIN-01, NO-YKMAN-BIN-01]

---

## Backlog

- FIDO2/WebAuthn status display (read-only, no management)
- Configurable refresh interval
- Export key directly to clipboard
- Migrate UI layer to reactive ratatui framework (user is building a Textual-style reactive rendering engine on top of ratatui — app.rs God struct needs componentization, wizard screens and popup widgets map well, src/yubikey/ unaffected)
