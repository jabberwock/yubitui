---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: Phases 1-3 are already implemented. Starting from Phase 1 to close remaining gaps.
status: Executing Phase 05
last_updated: "2026-03-26T03:23:34.057Z"
progress:
  total_phases: 5
  completed_phases: 4
  total_plans: 18
  completed_plans: 17
---

# Project State

## Current Phase

**Phase 5** — Native Card Protocol

## Status

active

## Current Plan

Phase 5 — Plan 01 (05-01) [complete]

## Progress

[████████░░] 88%

- Phase 1: complete (all 3 plans complete)
- Phase 2: complete (all 4 plans complete)
- Phase 3: complete (all 4 plans complete)
- Phase 4: complete (all 4 plans complete)
- Phase 5: in progress (1/3 plans complete)

## Completed Plans

- 01-01: Interactive key picker — arrow-key navigation replaces hardcoded available_keys[0] (2026-03-24)
- 01-02: Help screen — ? key opens keybinding reference overlay from any screen (2026-03-24)
- 01-03: README roadmap sync — checkboxes corrected, log path platform-aware (2026-03-24)
- 02-01: Foundation infrastructure — popup widgets, mouse capture, gpgconf-authoritative gnupg path (2026-03-24)
- 02-02: PIN unblock wizard — 4-branch decision tree, ykman factory reset with double confirmation (2026-03-24)
- 02-03: Key attributes display and SSH pubkey popup — ykman openpgp info parsing, in-TUI SSH key viewer (2026-03-24)
- 03-01: 20 unit tests across 5 parser modules, all parser functions pub, safe fingerprint slice in keys.rs (2026-03-24)
- 03-02: Touch policy and attestation backend — TouchPolicy enum, parse/set functions, attestation cert fetch, 12 unit tests (2026-03-24)
- 03-03: Multi-key detection, touch policy UI + set flow, attestation popup — Vec<YubiKeyState> with Tab cycling (2026-03-24)
- 03-04: CI 3-OS matrix and release workflow — GitHub Actions on Linux/macOS/Windows with clippy and tag-triggered binary releases (2026-03-24)
- 04-01: Foundational modules — GPG status-fd parser (21 tests), PIN input widget, progress popup (2026-03-25)
- 04-02: In-TUI PIN operations — programmatic gpg PIN functions with --pinentry-mode loopback, TUI PIN input, no terminal escape (2026-03-25)
- 04-03: Key generation wizard (7-step TUI) + programmatic import via --command-fd auto-mapping subkeys by capability (2026-03-25)
- 04-04: Terminal escape audit and cleanup — zero Stdio::inherit in yubikey modules, TUI SSH test connection input, deprecated functions removed (2026-03-25)
- 05-01: PC/SC APDU primitives module (card.rs) + native card reads for detection, PIN status, OpenPGP state, key attributes — no gpg/ykman subprocess for card reads (2026-03-25)

## Decisions

- README roadmap checkboxes corrected to reflect actual implementation state (Phase 1-3 items checked accurately)
- Log path note updated with platform-aware language covering Linux/macOS and Windows examples
- Consolidated redundant Phase 2 import lines into single 'Import keys to card (via GPG)' entry
- Global ? handler at top of handle_key_event before screen-specific blocks ensures uniform access from all screens
- previous_screen: Screen field stores return destination for modal overlay pattern
- Interactive key picker: use selected_key_index in KeyState, ratatui List widget with per-item styles for ImportKey screen
- [Phase 02-ux-menus-wizards-fixes]: Used Layout-based centering for popups since Rect::centered() does not exist in ratatui 0.29
- [Phase 02-ux-menus-wizards-fixes]: gpgconf --list-dirs homedir is now authoritative gnupg path source with Windows GPG4Win fallback
- [02-02]: UnblockUserPin kept with #[allow(dead_code)] for compat; UI routes through wizard variants
- [02-02]: factory_reset_openpgp uses ykman (not gpg) -- only ykman supports --force full OpenPGP app reset
- [02-03]: show_context_menu and menu_selected_index kept with #[allow(dead_code)] — reserved for Plan 02-04 context menu integration
- [02-03]: get_ssh_public_key_text() uses gpg --export-ssh-key with -- flag separator for security (defense-in-depth)
- [03-01]: Parser functions made pub to allow direct unit test calls; fixture strings used — no hardware required
- [03-01]: Safe fingerprint display uses .get(..16).unwrap_or(&str) instead of panic-prone [..16] slice
- [03-02]: touch_policy and attestation public API uses #[allow(dead_code)] — backend only until Plan 03-03 wires UI
- [03-02]: parse_attestation_result separated from get_attestation_cert for testability without YubiKey hardware
- [03-02]: VALID_ATTEST_SLOTS excludes "att" — attestation slot cannot self-attest per ykman behavior
- [03-04]: CI uses fail-fast: false so all OS results visible even when one fails
- [03-04]: libpcsclite-dev install is Linux-only conditional; macOS/Windows provide PCSC natively
- [03-04]: Release artifact names encode OS to prevent download collisions; Windows binary has .exe extension
- [03-04]: device-tests feature not enabled in any workflow — no YubiKey on CI runners
- [Phase 03-03]: App evolves from single yubikey_state: Option<YubiKeyState> to yubikey_states: Vec + selected_yubikey_idx; accessor yubikey_state() preserved for backward compat
- [Phase 03-03]: render() sites clone the selected state (.cloned()) rather than changing all render signatures
- [Phase 03-03]: 'a' key remapped to attestation; 'k' now opens key attributes (was 'a')
- [Phase 03-03]: detect_all_yubikey_states falls back to single detect_yubikey_state() -- gpg only sees one card
- [04-01]: #[allow(dead_code)] applied to pub items in gpg_status and pin_input/progress — these are consumed by Plans 02-04, not yet wired
- [04-01]: centered_area helper duplicated in pin_input.rs and progress.rs (as in popup.rs) rather than extracting to shared util — avoids changing existing popup.rs
- [04-02]: TODO comment instead of #[deprecated] on old interactive fns — clippy -D warnings treats deprecated-fn usage as error while app.rs still called them
- [04-02]: Background thread + mpsc channel for stderr reading in run_gpg_pin_operation — avoids single-thread deadlock from simultaneous stdin write + stderr read
- [04-02]: OperationRunning renders synchronously (blocking call) — progress popup shown but spinner does not animate during blocking gpg call; acceptable for v1
- [04-03]: GenerateKey KeyScreen variant removed after wizard added — no longer constructed; render_generate_key kept with #[allow(dead_code)] as fallback
- [04-03]: import_key_programmatic writes all edit-key commands upfront then answers GET_HIDDEN via mpsc channel — same pattern as run_gpg_pin_operation
- [04-03]: current_date_ymd() uses std::time + Gregorian day-of-epoch algorithm — avoids adding chrono to app logic (already in dev-deps)
- [04-04]: set_touch_policy returns Result<String> (not Result<Child>) — callers get outcome immediately without managing child lifetime; --force means no interactive PIN needed
- [04-04]: test_ssh_connection uses BatchMode=yes + ConnectTimeout=10 + Stdio::piped — non-interactive, never hangs, all output captured for TUI display
- [04-04]: SSH ExportKey routes to Screen::Keys + KeyScreen::SshPubkeyPopup (existing TUI popup) from execute_ssh_operation
- [04-04]: TestConnection screen gains TUI text input fields (test_conn_user/host/focused in SshState); Tab switches focus; Enter submits
- [04-04]: Deprecated interactive functions fully removed (not just #[allow(dead_code)]) — clean break, no legacy code
- [05-01]: card.rs is the single PC/SC APDU primitive module; kill_scdaemon before every native card operation
- [05-01]: serial_from_aid() reads BCD-encoded serial from AID select response bytes 10-13 (not ykman list --serials)
- [05-01]: tlv_find() BER-TLV walker used for DO 0x6E fingerprints and algorithm attributes; no flat-offset parsing
- [05-01]: apdu_error_message(sw, context) maps all SW codes to plain English; raw SW goes to tracing::debug! only
- [05-01]: detect_all_yubikey_states() builds full YubiKeyState from single card connection per reader

## Notes

- Cross-platform requirement is non-negotiable (Linux/macOS/Windows)
- Security rules: no sensitive values in logs, no shell injection, no hardcoded paths
- Always run `cargo clippy -- -D warnings` before committing

## Last Session

- Stopped at: Completed 05-01-PLAN.md (PC/SC primitives, native detection/pin/openpgp/key_operations)
- Date: 2026-03-25
