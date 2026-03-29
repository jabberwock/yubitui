---
gsd_state_version: 1.0
milestone: v1.1
milestone_name: Accessible to New Users
status: Phase complete — ready for verification
stopped_at: Completed 12-04-PLAN.md — refresh wired in dashboard/keys/piv
last_updated: "2026-03-29T18:47:18.658Z"
progress:
  total_phases: 9
  completed_phases: 6
  total_plans: 34
  completed_plans: 28
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-26)

**Core value:** Zero-friction YubiKey management — detect problems automatically, guide users through fixes
**Current focus:** Phase 12 — yubikey-slot-delete-workflow

## Current Position

Phase: 12 (yubikey-slot-delete-workflow) — EXECUTING
Plan: 3 of 3

## Performance Metrics

**Velocity (v1.0 baseline):**

- Total plans completed (v1.0): 18
- v1.1 plans completed: 0

*Updated after each plan completion*

## Accumulated Context

### Decisions

- [v1.0]: Native PC/SC via pcsc crate — zero ykman dependency; all card reads direct APDUs
- [v1.0]: Kill scdaemon before exclusive card access; 50ms sleep debt unpaid (Phase 6 pays it)
- [v1.1 roadmap]: Infrastructure split MUST precede new screens — all four research files agree
- [v1.1 roadmap]: Phase 9 (FIDO2) carries research-phase flag — CTAP2 credential management is MEDIUM confidence; spike during planning
- [Phase 06-tech-debt-infrastructure]: src/model/ is the model layer (zero ratatui imports), src/tui/ is the TUI layer; boundary enforced by CI grep lint
- [Phase 06-tech-debt-infrastructure]: AppState struct in model/app_state.rs holds Tauri-serializable state; pin_state/key_state/ssh_state/dashboard_state remain on App (TUI-specific)
- [Phase 06-tech-debt-infrastructure]: firmware=None in DeviceInfo now returns Model::Unknown (not YubiKeyNeo); openpgp_version is OpenPGP spec version, never hardware firmware fallback
- [Phase 06-tech-debt-infrastructure]: Mock fixture is hardcoded Rust struct — no file I/O, deterministic for CI; --mock flag passes through App::new() and guards all detect_all() call sites
- [Phase 06-tech-debt-infrastructure]: 50ms sleep after kill_scdaemon() is now present at all card APDU entry points (detection.rs and factory_reset gaps fixed)
- [Phase 07-03]: wait_for_text retry loop (0.3s poll) replaces fixed sleep+assert — eliminates CI timing races in E2E tests
- [Phase 07-03]: E2E smoke test pattern: start_session -> wait_for_text -> menu nav -> assert content -> Esc back -> cleanup -> echo PASS
- [Phase 07-01]: ClickAction placed in src/model/click_region.rs referencing tui action enums; cross-layer reference valid within single Rust crate; From<Rect> for Region in tui/mod.rs as sole Rect conversion boundary; EnableMouseCapture wrapped in if-let-Err for Windows ConPTY graceful degradation
- [Phase 07-02]: PivTuiState/DiagnosticsTuiState created as TUI-layer structs — model::piv::PivState is card hardware data; no DiagnosticsState existed; followed SshState/KeyState pattern
- [Phase 07-02]: render_context_menu returns Rect so dashboard registers per-item click regions without recomputing centered_area geometry
- [Phase 07-02]: std::mem::take(&mut click_regions) in render() to resolve borrow checker conflict; render() signature changed to &mut self
- [Phase 07-mouse-support-e2e-test-harness]: dashboard::render() decoupled to &AppState — enables test isolation without constructing full App
- [Phase 07-mouse-support-e2e-test-harness]: ssh::render() had unused _app: &App parameter — removed entirely (simpler, no data needed)
- [Phase 08]: textual-rs not on crates.io — use git dep (jabberwock/textual-rs); update to registry version when published
- [Phase 08]: ratatui 0.30 upgrade had zero breaking changes in yubitui codebase — cargo check passed immediately after dep bump
- [Phase 08-02]: app.rs is now thin pub fn run() — old App struct and crossterm event loop fully deleted; HelpScreen uses compose() with Label widgets for display-only content
- [Phase 08-02]: Theme names verified against actual textual-rs builtin_themes(): use gruvbox-dark and catppuccin-mocha (not gruvbox/catppuccin)
- [Phase 08-03]: SshWizardScreen retains 6 sub-screens as Reactive<SshState>.screen — no push_screen_deferred; keeps SshState serializable (D-04)
- [Phase 08-03]: DiagnosticsScreen uses full-width layout (no sidebar) — 4 diagnostic items flow naturally as a sequential list
- [Phase 08]: Legacy ratatui shims kept in widgets for unmigrated keys.rs/dashboard.rs; removed in 08-05/08-06
- [Phase 08]: PinManagementScreen uses push_screen_deferred+ModalScreen for all wizard sub-screens (change/admin/reset/unblock)
- [Phase 08]: textual-rs App runner handles 'q' quit and Ctrl+T theme globally — on_action does not re-implement these
- [Phase 08]: KeyState.pin_input removed — pushed PinInputWidget screen replaces inline state in textual-rs model
- [Phase 08-06]: Snapshot dimensions 80x24 over 120x40 — standard terminal width produces realistic snapshots
- [Phase 08-06]: Pilot navigation tests: pilot.press() + settle() + snapshot captures full screen-push rendering
- [Phase 09-oath-totp-screen]: OathScreen countdown bar computed from chrono::Utc::now() on each compose() call — no timer thread needed since textual-rs re-renders on key events
- [Phase 09-02]: OathScreen countdown computed per-render from chrono::Utc::now() — no background timer thread; textual-rs re-renders on key events
- [Phase 09-02]: HOTP with no code shows '[press Enter]' placeholder — full HOTP generation (card APDU) wired in Plan 03
- [Phase 09-03]: Used on_event() with downcast_ref KeyEvent for character-level input in AddAccountScreen wizard
- [Phase 09-03]: DeleteConfirmScreen delegates compose/key_bindings to inner ConfirmScreen; overrides on_action to call delete_credential()
- [Phase 09-oath-totp-screen]: nav_7 follows nav_1..nav_6 pattern; '[7] OATH / Authenticator' button label matches Yubico Authenticator branding
- [Phase 10-fido2-screen]: Fido2State.firmware_version stored as Option<String> (formatted) — model layer converts from packed u32 so TUI just displays the string
- [Phase 10-fido2-screen]: Fido2State.credentials: None=locked, Some([])=no creds, Some(creds)=populated — three-state distinction drives TUI rendering
- [Phase 10-fido2-screen]: PinAuthScreen pops parent Fido2Screen on success and pushes new Fido2Screen with credentials — cleanest way to propagate credential list without cross-screen state mutation
- [Phase 10-fido2-screen]: reset_fido2() uses raw CTAPHID frames via hidapi — ctap-hid-fido2 does not expose authenticatorReset (0x07)
- [Phase 10-fido2-screen]: ResetGuidanceScreen stores own_id via on_mount() Cell<Option<WidgetId>> for matching worker source_id in on_event()
- [Phase 10-fido2-screen]: Dashboard nav_8 reads yk.fido2.clone() via yubikey_state().and_then() — same pattern as nav_7 oath
- [Phase 11]: OTP slot status is Occupied/Empty only — credential type is write-only on hardware; screen includes hardware limitation note
- [Phase 11]: nav_7 used for OTP Slots in current worktree state — phases 9/10 (oath/fido2) not yet executed; OTP becomes nav_7
- [Phase 11-02]: GlossaryScreen uses same Widget pattern as HelpScreen — no new widget types needed
- [Phase 11-02]: Help popups use existing ModalScreen+PopupScreen pattern — each screen gets a contextual SCREEN_HELP_TEXT const
- [Phase 12]: OpenPGP slot delete uses PUT DATA attribute-change trick (RSA4096 then RSA2048) — no DELETE KEY APDU exists; requires Admin PIN auth
- [Phase 12]: PIV cert delete is PUT DATA with empty 0x53 value; PIV key delete is MOVE KEY INS=0xF6 P1=0xFF (firmware 5.7+ ONLY)
- [Phase 12]: PIV management key auth is 3DES challenge-response via `des` crate TdesEde3; new dependency added to Cargo.toml
- [Phase 12-01]: OpenPGP slot delete uses PUT DATA attribute-change trick: RSA4096 then RSA2048 — no DELETE KEY APDU exists on OpenPGP card spec
- [Phase 12-01]: Admin PIN VERIFY SW 0x63Cx: retry count from low nibble; 0x6983 = blocked — clear user messages returned from delete_openpgp_key()
- [Phase 12-01]: Two-step delete TUI flow: PinThenDeleteScreen -> DeleteKeyScreen (mirrors fido2 PinAuthScreen -> DeleteCredentialScreen pattern)
- [Phase 12]: des 0.9.0-rc.3 with cipher 0.5 required — cipher 0.4 incompatible with des pre-release
- [Phase 12]: DeletePivConfirmScreen pushes PivScreen(None) after delete — full YubiKeyState not available in modal context; user refreshes with R
- [Phase 12]: KeysScreen compose() must include 'd Delete Key Slot' label to match KEYS_BINDINGS show=true declaration
- [Phase 12]: Refresh uses pop+push-fresh-screen pattern (same as delete confirm screens) rather than in-place mutation

### Roadmap Evolution

- Phase 12 added: yubikey slot delete workflow — individual OpenPGP/PIV slot deletion without factory reset
- Phase 13 added: UI polish — consistent card/panel layout, visual hierarchy, and design quality across all screens to match PIN Management standard

### Pending Todos

None.

### Blockers/Concerns

- Phase 9 (FIDO2): ctap-hid-fido2 credential management API needs prototyping before full plan scope is locked — flag for plan-phase to trigger research spike

## Session Continuity

Last session: 2026-03-29T18:47:18.656Z
Stopped at: Completed 12-04-PLAN.md — refresh wired in dashboard/keys/piv
Resume file: None
