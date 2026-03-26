# Requirements: yubitui v1.1

**Defined:** 2026-03-26
**Core Value:** Zero-friction YubiKey management — detect problems automatically, guide users through fixes, expose all operations without requiring memorization of CLI incantations.

## v1.1 Requirements

### Infrastructure & Architecture

- [ ] **INFRA-01**: Developer can run app with `--mock` flag to get fixture YubiKeyState without hardware (enables E2E testing in CI)
- [ ] **INFRA-02**: Card connection is reliable — 50ms sleep after scdaemon kill before any new APDU operation (pay v1.0 tech debt)
- [x] **INFRA-03**: App state is split into `src/model/` (zero ratatui imports, all state `Clone + Debug`) and `src/tui/` (all ratatui rendering) with no cross-contamination
- [x] **INFRA-04**: Business logic modules (`src/yubikey/`, `src/model/`) have zero ratatui imports — enforced by CI lint
- [ ] **INFRA-05**: Each screen has its own typed action enum and `handle_key()` function — no 1600-line match arm in app.rs
- [x] **INFRA-06**: Model types implement `serde::Serialize` — Tauri-serializable without code changes when GUI is added

### Mouse & E2E Testing

- [ ] **MOUSE-01**: User can click any navigation item, menu entry, or button to activate it (mouse click-to-navigate works on all screens)
- [ ] **MOUSE-02**: User can scroll lists with the mouse wheel
- [ ] **MOUSE-03**: Mouse click regions use a `ClickRegionMap` rebuilt each frame — coordinates are always accurate after terminal resize
- [ ] **MOUSE-04**: On Windows (ConPTY), mouse events degrade gracefully to keyboard-only with no crash or error message
- [ ] **TEST-01**: E2E test harness exists under `tests/e2e/` using tmux `send-keys`/`capture-pane` — runs without YubiKey hardware using `--mock` flag
- [ ] **TEST-02**: All existing screens have at least one tmux E2E smoke test covering navigation and key interactions
- [ ] **TEST-03**: New screens (OATH, FIDO2, OTP) each have tmux E2E tests written before or alongside implementation (TDD)
- [ ] **TEST-04**: Ratatui TestBackend + insta snapshot tests cover rendering of each screen's key states

### OATH / TOTP

- [ ] **OATH-01**: User can view a list of all OATH credentials stored on the YubiKey with their current TOTP/HOTP codes
- [ ] **OATH-02**: User can see a countdown timer showing seconds remaining in the current 30s TOTP window
- [ ] **OATH-03**: User can add a new OATH account by manually entering an issuer, account name, and secret (Base32)
- [ ] **OATH-04**: User can delete an OATH account with an irreversibility confirmation dialog
- [ ] **OATH-05**: User is prompted for an OATH password when the key returns SW 0x6982 (password-protected OATH applet) before any credential operation
- [ ] **OATH-06**: TOTP codes are generated using the current system time as the epoch challenge — the YubiKey's on-card CALCULATE APDU receives the correct 8-byte big-endian timestep

### FIDO2

- [ ] **FIDO-01**: User can view a FIDO2 info screen showing: firmware version, supported algorithms, PIN status (set/not set), PIN retry count
- [ ] **FIDO-02**: User can set a FIDO2 PIN when none is configured
- [ ] **FIDO-03**: User can change an existing FIDO2 PIN
- [ ] **FIDO-04**: User can view a list of resident FIDO2 credentials (passkeys) stored on the YubiKey
- [ ] **FIDO-05**: User can delete a specific resident FIDO2 credential with a confirmation dialog
- [ ] **FIDO-06**: User can reset the FIDO2 applet (with prominent warning about credential loss and 10s timing window requirement)
- [ ] **FIDO-07**: On Windows, user sees a clear message when FIDO2 operations require administrator privileges

### OTP Slots

- [ ] **OTP-01**: User can view OTP slot status screen showing slot 1 and slot 2 occupancy and configured type (Yubico OTP, static password, HMAC-SHA1, empty)

### Education & Onboarding

- [ ] **EDU-01**: User can press `?` on any screen to open a help panel explaining what the current screen does and what the protocol/feature is (PIV, FIDO2, OATH, OTP, OpenPGP, SSH — each screen has its own content)
- [ ] **EDU-02**: User can access a protocol glossary (accessible from the main menu or `?` from dashboard) explaining: PIV, FIDO, FIDO2, OpenPGP/PGP, SSH, TOTP, HOTP, OTP/Yubico OTP
- [ ] **EDU-03**: On first launch (or when device is in factory-default state), user sees an onboarding checklist that guides them through initial setup steps (FIDO2 PIN, OATH accounts, PIV/SSH if needed)
- [ ] **EDU-04**: Onboarding detects factory-default state heuristically: FIDO2 has no PIN set, OATH applet has 0 credentials, PIV uses default management key

## v2 Requirements (Deferred)

### PIV Improvements

- **PIV-01**: User can view decoded X.509 certificate details for occupied PIV slots (requires `x509-parser` crate)
- **PIV-02**: User can change the PIV Management Key

### OATH Advanced

- **OATH-07**: User can set/change an OATH application password (HMAC challenge/response auth)
- **OATH-08**: User can import OATH accounts via otpauth:// URI paste

### OTP Slot Write

- **OTP-02**: User can configure OTP slot 1 or 2 (Yubico OTP, static password, HMAC-SHA1) — deferred due to underdocumented HID frame protocol and access code complexity
- **OTP-03**: User can delete/wipe an OTP slot configuration

### FIDO2 Advanced

- **FIDO-08**: User can manage FIDO2 fingerprints (Bio series YubiKeys only)
- **FIDO-09**: User can enable/disable YubiKey applications (requires Management Key auth)

### Backup & Restore

- **BACK-01**: User can export a backup of non-key-material configuration (OATH accounts, PIV metadata)
- **BACK-02**: User can restore configuration from backup

## Out of Scope

| Feature | Reason |
|---------|--------|
| QR code scanning for OATH | No camera in a TUI — use manual URI/secret entry instead |
| Cloud backup of TOTP secrets | Defeats the security model of hardware-bound secrets |
| ykman subprocess calls | Core project constraint — native PC/SC APDUs only, always |
| OpenPGP key reset from TUI | Catastrophic and irreversible without the full gpg confirmation flow |
| FIDO2 via PC/SC | FIDO2 requires HID FIDO transport (0xF1D0), not CCID — routing through PC/SC is incorrect |
| ratatui 0.30 upgrade | Bumps MSRV to 1.86; separate concern, not a v1.1 feature requirement |
| GUI / non-TUI interface | Terminal-first always; Tauri GUI is future work, not v1.1 |
| OTP slot write (v1.1) | High-risk underdocumented HID frame protocol; deferred to v2 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| INFRA-01 | Phase 6 | Pending |
| INFRA-02 | Phase 6 | Pending |
| INFRA-03 | Phase 6 | Complete |
| INFRA-04 | Phase 6 | Complete |
| INFRA-05 | Phase 6 | Pending |
| INFRA-06 | Phase 6 | Complete |
| MOUSE-01 | Phase 7 | Pending |
| MOUSE-02 | Phase 7 | Pending |
| MOUSE-03 | Phase 7 | Pending |
| MOUSE-04 | Phase 7 | Pending |
| TEST-01 | Phase 7 | Pending |
| TEST-02 | Phase 7 | Pending |
| TEST-03 | Phase 7 | Pending |
| TEST-04 | Phase 7 | Pending |
| OATH-01 | Phase 8 | Pending |
| OATH-02 | Phase 8 | Pending |
| OATH-03 | Phase 8 | Pending |
| OATH-04 | Phase 8 | Pending |
| OATH-05 | Phase 8 | Pending |
| OATH-06 | Phase 8 | Pending |
| FIDO-01 | Phase 9 | Pending |
| FIDO-02 | Phase 9 | Pending |
| FIDO-03 | Phase 9 | Pending |
| FIDO-04 | Phase 9 | Pending |
| FIDO-05 | Phase 9 | Pending |
| FIDO-06 | Phase 9 | Pending |
| FIDO-07 | Phase 9 | Pending |
| OTP-01 | Phase 10 | Pending |
| EDU-01 | Phase 10 | Pending |
| EDU-02 | Phase 10 | Pending |
| EDU-03 | Phase 10 | Pending |
| EDU-04 | Phase 10 | Pending |

**Coverage:**
- v1.1 requirements: 34 total
- Mapped to phases: 34
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-26*
*Last updated: 2026-03-26 — traceability populated by roadmapper*
