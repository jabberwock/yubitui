---
gsd_state_version: 1.0
milestone: v1.1
milestone_name: Accessible to New Users
status: Phase complete — ready for verification
stopped_at: Completed 07-04-PLAN.md (insta snapshot tests for all 7 screens)
last_updated: "2026-03-27T03:45:07.959Z"
progress:
  total_phases: 5
  completed_phases: 2
  total_plans: 7
  completed_plans: 7
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-26)

**Core value:** Zero-friction YubiKey management — detect problems automatically, guide users through fixes
**Current focus:** Phase 07 — mouse-support-e2e-test-harness

## Current Position

Phase: 07 (mouse-support-e2e-test-harness) — EXECUTING
Plan: 4 of 4 (07-01, 07-03 complete)

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

### Pending Todos

None.

### Blockers/Concerns

- Phase 9 (FIDO2): ctap-hid-fido2 credential management API needs prototyping before full plan scope is locked — flag for plan-phase to trigger research spike

## Session Continuity

Last session: 2026-03-27T03:45:07.957Z
Stopped at: Completed 07-04-PLAN.md (insta snapshot tests for all 7 screens)
Resume file: None
