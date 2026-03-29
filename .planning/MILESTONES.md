# yubitui Milestones

## v1.1 Accessible to New Users (Shipped: 2026-03-29)

**Phases completed:** 9 phases, 34 plans, 43 tasks

**Key accomplishments:**

- src/yubikey/ renamed to src/model/, src/ui/ to src/tui/, all model types derive serde::Serialize, AppState struct extracted, CI boundary lint added, and YubiKey NEO misidentification bug fixed with TDD tests
- 1. [Rule 3 - Blocking] Worktree behind main — 06-01 foundation not present
- Task 1: --mock flag and fixture
- Region/ClickRegion/ClickAction types in model layer with zero ratatui imports, Clone+Debug on all action enums, From<Rect> conversion boundary in tui layer, and Windows ConPTY graceful degradation
- Region-based click dispatch with reverse iteration (popup-first), scroll on all list screens, all 7 render functions emit click regions, old per-screen handle_mouse removed
- tmux-based E2E harness with 6 screen smoke tests using wait_for_text retry polling — all pass against --mock mode without hardware
- Insta snapshot tests for all 7 TUI screens with TestBackend rendering, mock fixture, no-yubikey state coverage, and dashboard/ssh decoupled from &App
- 1. [Rule 3 - Blocking] textual-rs not published to crates.io
- textual-rs App runner replaces crossterm event loop; theme/config persistence added; Help screen becomes first textual-rs Widget with Header/Footer and key bindings
- Diagnostics, PIV, and SSH screens migrated to textual-rs Widgets with Header, Footer, and visible keybindings — 4 of 7 screens now migrated
- PIN Management screen and its dependent widgets (pin_input, popup) fully migrated to textual-rs Widgets with wizard sub-screens as pushed modal screens
- Dashboard wired as root screen with 6 push_screen_deferred navigation buttons; Keys screen and all 7 sub-flows (KeyGenWizard, Import, Delete, TouchPolicy) migrated to textual-rs Widgets — all 7 screens now migrated
- 15 insta snapshot files accepted for all 7 textual-rs screens using TestApp Pilot tests — tmux E2E harness fully retired, all screen coverage in cargo test
- Status
- OathScreen textual-rs Widget with credential list (name/code/type badge per row), HOTP [press Enter] placeholder, and live TOTP countdown bar from chrono::Utc::now()
- AddAccountScreen
- Dashboard nav_7 key and "[7] OATH / Authenticator" button wire OathScreen via push_screen_deferred; 4 Pilot snapshot tests and human verification confirm all 6 OATH requirements satisfied
- 1. [Rule 1 - Bug] ctap-hid-fido2 type mismatches in research documentation
- Dashboard nav_8 wired to Fido2Screen via [8] FIDO2 / Security Key button, with fido2_from_mock snapshot test; 135 tests pass.
- 1. [Rule 3 - Blocking] Fixed textual-rs path dependency for worktree
- 1. [Rule 3 - Blocking] Merged local-main/main into worktree before starting
- OnboardingScreen for factory-default YubiKeys, ctx.quit() on dashboard, q→back on all sub-screens, OTP ? help panel — textual-rs 0.3.5 upgrade resolved the quit API
- OpenPGP individual key slot deletion via Admin PIN + RSA attribute-change trick (PUT DATA RSA4096 -> RSA2048), with two-step TUI flow (PIN collection -> confirmation) wired into KeysScreen
- 1. [Rule 3 - Blocking] cipher 0.4 vs cipher 0.5 version conflict
- All 160 cargo tests pass with updated snapshots showing 'd Delete Key Slot' in KeysScreen action list; PIV snapshot confirms 'D to delete' footer; human verification of live delete flows pending
- src/tui/dashboard.rs
- One-liner:
- 1. [Rule 1 - Bug] DataTable API mismatch — plan showed `DataTable::new(columns, rows)` but actual API is `DataTable::new(columns)` with `add_row(&mut self)`
- Slot summary — replaced 3 Labels with DataTable:
- Credential list
- 1. [Rule 1 - Bug] DataTable API mismatch

---

## v1.0 — Production-Ready

**Shipped:** 2026-03-26
**Phases:** 5 | **Plans:** 21 | **Commits:** 168
**LOC:** ~10,053 Rust | **Files:** 112 changed (+23,893 / -497)
**Timeline:** 2026-03-24 → 2026-03-26 (3 days)

### Delivered

Complete, self-contained YubiKey TUI with zero external CLI dependencies. All card reads go through native PC/SC APDUs (pcsc crate); gpg is used for keyring operations only; ykman binary is not required.

### Key Accomplishments

1. **Interactive key picker + help screen** — arrow-key navigation replaces hardcoded key selection; `?` opens keybinding reference from any screen
2. **PIN unblock wizard** — 4-branch decision tree (reset code → admin PIN → factory reset → abort) with double-confirmation destructive action
3. **SSH setup wizard** — guides non-experts through gpg-agent.conf, agent restart, SSH_AUTH_SOCK, and connection test without leaving the TUI
4. **Full programmatic GPG control** — `--command-fd`/`--status-fd`/`--pinentry-mode loopback` for all PIN ops, key import, and 7-step on-device key generation wizard; zero terminal escape
5. **Native PC/SC APDU protocol** — card.rs module replaces all ykman/gpg card reads with direct APDUs; T=0 GET RESPONSE chaining; BER-TLV parser
6. **PIV certificates screen** — Screen::Piv with native slot occupancy reads for 9a/9c/9d/9e; multi-YubiKey Tab-switching; touch policy per slot; attestation popup

### Known Gaps (Tech Debt)

- Phase 04 VERIFICATION.md missing (functional evidence confirmed by Phase 05 verifier cross-check)
- cargo fmt diffs in src/app.rs, src/ui/pin.rs, src/ui/widgets/popup.rs, src/utils/config.rs
- Char('t') nav arm doesn't clear stale message (cosmetic)
- No 50ms sleep after kill_scdaemon() on Linux (may cause Card Busy on slow teardown)

**Archive:** `.planning/milestones/v1.0-ROADMAP.md`
**Audit:** `.planning/milestones/v1.0-MILESTONE-AUDIT.md`
