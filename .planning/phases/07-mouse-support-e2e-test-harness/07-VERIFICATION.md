---
phase: 07-mouse-support-e2e-test-harness
verified: 2026-03-26T08:00:00Z
status: passed
score: 12/12 must-haves verified
re_verification: false
---

# Phase 7: Mouse Support + E2E Test Harness Verification Report

**Phase Goal:** Add full mouse support (click navigation + scroll) across all screens and establish the E2E test harness with smoke tests and insta snapshot tests for all 7 screens.
**Verified:** 2026-03-26
**Status:** PASSED
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | ClickRegion and Region types exist in src/model/ with zero ratatui imports | VERIFIED | src/model/click_region.rs has Region, ClickAction, ClickRegion; grep ratatui returns 0 hits |
| 2 | All per-screen action enums derive Clone | VERIFIED | All 7 action enums have `#[derive(Clone, Debug)]` confirmed across dashboard/keys/pin/piv/ssh/diagnostics/help |
| 3 | AppState has a click_regions field with serde(skip) | VERIFIED | Line 28-29 of app_state.rs: `#[serde(skip)]` + `pub click_regions: Vec<ClickRegion>` |
| 4 | EnableMouseCapture failure on Windows does not crash the app | VERIFIED | app.rs line 75: `if let Err(e) = execute!(stdout, EnableMouseCapture)` with tracing::debug log |
| 5 | User can click any navigation item, menu entry, or button on any screen | VERIFIED | All 7 render functions accept `click_regions: &mut Vec<ClickRegion>` and push regions; execute_click_action dispatches all variants |
| 6 | User can scroll any list with mouse wheel on Keys, PIV, SSH, Diagnostics | VERIFIED | handle_scroll() has explicit match arms for Screen::Keys/Piv/SshWizard/Diagnostics; scroll_offset fields confirmed in piv_tui_state, ssh_state, diagnostics_tui_state |
| 7 | After terminal resize, click targets remain accurate | VERIFIED | Regions registered each frame in render (regions cleared and rebuilt per frame via std::mem::take pattern) |
| 8 | Mouse click dispatch uses ClickRegionMap lookup with .iter().rev() | VERIFIED | app.rs line 163: `self.state.click_regions.iter().rev().find(|r| r.region.contains(col, row))` |
| 9 | Popup/modal clicks captured before background elements | VERIFIED | .iter().rev() ensures last-pushed (popup) regions win; dashboard pushes menu items after nav items |
| 10 | E2E test harness runs against --mock without hardware | VERIFIED | run_all.sh 6 passed 0 failed; helpers.sh uses `$BINARY --mock` |
| 11 | All 6 existing screen smoke tests pass | VERIFIED | bash tests/e2e/run_all.sh: 6 passed, 0 failed |
| 12 | insta snapshot tests exist for all 7 screens, cargo test passes | VERIFIED | 15 assert_snapshot! calls across all 7 screen files; 15 .snap files in src/tui/snapshots/; cargo test: 109 passed, 0 failed |

**Score:** 12/12 truths verified

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/model/click_region.rs` | Region, ClickRegion, ClickAction types | VERIFIED | All 3 types present, zero ratatui imports |
| `src/model/app_state.rs` | click_regions field on AppState | VERIFIED | Field at line 29, serde(skip) at line 28, yubikey_state/yubikey_count methods at lines 47/51 |
| `src/tui/mod.rs` | From<Rect> for Region conversion | VERIFIED | Line 10: `impl From<ratatui::layout::Rect> for crate::model::click_region::Region` |
| `src/app.rs` | execute_click_action and handle_scroll | VERIFIED | fn execute_click_action at line 181, fn handle_scroll at line 203 |
| `src/tui/dashboard.rs` | Click region registration | VERIFIED | 4 click_regions.push calls; render sig uses &AppState not &App |
| `src/tui/keys.rs` | Click region registration | VERIFIED | 2 click_regions.push calls |
| `src/tui/pin.rs` | Click region registration | VERIFIED | 1 click_regions.push call (back button) |
| `src/tui/piv.rs` | Click region registration + PivTuiState | VERIFIED | 1 push, PivTuiState.scroll_offset at line 11 |
| `src/tui/ssh.rs` | Click region registration + SshState.scroll_offset | VERIFIED | 2 pushes, scroll_offset at line 113 |
| `src/tui/diagnostics.rs` | Click region registration + DiagnosticsTuiState | VERIFIED | 1 push, scroll_offset at line 11 |
| `src/tui/help.rs` | Click region registration | VERIFIED | 1 push (whole-area close) |
| `tests/e2e/helpers.sh` | wait_for_text with retry loop | VERIFIED | while loop polling every 0.3s, executable (-rwxr-xr-x) |
| `tests/e2e/run_all.sh` | Aggregates results, exits non-zero on failure | VERIFIED | PASS/FAIL counters, loop over *_smoke.sh, exit based on FAIL count |
| `tests/e2e/dashboard_smoke.sh` | Dashboard smoke test | VERIFIED | Uses wait_for_text, echoes PASS: dashboard_smoke |
| `tests/e2e/diagnostics_smoke.sh` | Diagnostics smoke test | VERIFIED | Uses wait_for_text (3 calls), echoes PASS: diagnostics_smoke |
| `tests/e2e/keys_smoke.sh` | Keys smoke test | VERIFIED | Uses wait_for_text (3 calls), echoes PASS: keys_smoke |
| `tests/e2e/pin_smoke.sh` | PIN smoke test | VERIFIED | Uses wait_for_text (3 calls), echoes PASS: pin_smoke |
| `tests/e2e/piv_smoke.sh` | PIV smoke test | VERIFIED | Uses wait_for_text (3 calls), echoes PASS: piv_smoke |
| `tests/e2e/ssh_smoke.sh` | SSH smoke test | VERIFIED | Uses wait_for_text (3 calls), echoes PASS: ssh_smoke |
| `Cargo.toml` | insta dev-dependency | VERIFIED | Line 59: `insta = "1.47"` under [dev-dependencies] |
| `src/tui/snapshots/` | 15 .snap files for all screens | VERIFIED | Exactly 15 .snap files covering all 7 screens, 0 pending .snap.new files |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/tui/dashboard.rs render()` | `src/model/app_state.rs click_regions` | `&mut Vec<ClickRegion>` param | WIRED | Line 95: param accepted; line 238/250/254/287: push calls confirmed |
| `src/app.rs handle_mouse_event()` | `src/model/click_region.rs Region::contains` | `click_regions.iter().rev() + region.contains(col, row)` | WIRED | Line 163-164: iter().rev().find(|r| r.region.contains(col, row)) |
| `src/tui/dashboard.rs tests` | `src/model/mock.rs` | `mock_yubikey_states()` | WIRED | Line 303: `use crate::model::{mock::mock_yubikey_states, AppState}` |
| `tests/e2e/*.sh` | `cargo run -- --mock` | `tmux send-keys/capture-pane` | WIRED | helpers.sh line 21: `$BINARY --mock`; all 6 tests pass against mock binary |
| `src/app.rs execute_click_action()` | per-screen executor functions | match on ClickAction variants | WIRED | Line 181: dispatches Dashboard/Keys/Pin/Piv/Ssh/Diagnostics/Help variants |

---

## Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|-------------------|--------|
| `src/tui/dashboard.rs` | yubikey_states | app_state.yubikey_state() / mock_yubikey_states() | Yes — mock fixture or live AppState | FLOWING |
| `src/tui/keys.rs` | key_state.available_keys | KeyState from App | Yes — populated by gpg key loading | FLOWING |
| Snapshot tests (all 7 screens) | TestBackend buffer | mock_yubikey_states() or Diagnostics::default() | Yes — real TUI render output | FLOWING |

---

## Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| cargo test passes all snapshot assertions | `cargo test` | 109 passed, 0 failed | PASS |
| E2E run_all.sh exits 0 | `bash tests/e2e/run_all.sh` | 6 passed, 0 failed | PASS |
| No pending snapshot files | `find . -name "*.snap.new"` | 0 results | PASS |
| Zero ratatui imports in model layer | `grep -r "ratatui" src/model/` | 0 results | PASS |
| Reverse iteration in mouse dispatch | `grep "iter().rev()" src/app.rs` | Line 163 confirmed | PASS |
| scroll_offset in piv/ssh/diagnostics | `grep "scroll_offset" src/tui/piv.rs ...` | All 3 files confirmed | PASS |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| MOUSE-01 | 07-02 | User can click any nav item, menu entry, or button | SATISFIED | All 7 render functions push click regions; execute_click_action dispatches to all screen executors; E2E tests navigate via key press which mirrors click dispatch |
| MOUSE-02 | 07-02 | User can scroll lists with mouse wheel | SATISFIED | handle_scroll() with explicit arms for Keys/Piv/SshWizard/Diagnostics; scroll_offset fields in PivTuiState, SshState, DiagnosticsTuiState |
| MOUSE-03 | 07-01 | ClickRegionMap rebuilt each frame — accurate after resize | SATISFIED | std::mem::take pattern + clear-and-rebuild on every render call; Region type in model with Rect conversion only in tui layer |
| MOUSE-04 | 07-01 | Windows ConPTY graceful degradation | SATISFIED | app.rs: if-let-Err on EnableMouseCapture + let _ on DisableMouseCapture |
| TEST-01 | 07-03 | E2E harness under tests/e2e/ using tmux, works with --mock | SATISFIED | 8 files in tests/e2e/; helpers.sh uses $BINARY --mock; all pass without hardware |
| TEST-02 | 07-03 | All existing screens have at least one tmux E2E smoke test | SATISFIED | 6 smoke tests: dashboard, diagnostics, keys, pin, piv, ssh — all pass |
| TEST-03 | 07-03 | TDD pattern established for new screens | SATISFIED | E2E harness pattern documented and working; run_all.sh ready for CI integration |
| TEST-04 | 07-04 | Ratatui TestBackend + insta snapshot tests for each screen | SATISFIED | 15 assert_snapshot! calls across all 7 screens; 15 .snap files committed; 0 pending |

All 8 requirements satisfied. No orphaned requirements detected.

---

## Anti-Patterns Found

None detected. Scanned src/model/click_region.rs, src/app.rs, and all src/tui/*.rs files. No TODO/FIXME/PLACEHOLDER comments, no empty return null/return []/return {} implementations flowing to rendering. The SSH wizard uses SshAction::NavigateTo(Screen::Dashboard) as a back-button click region and SshAction dispatched from wizard step rows — this is functionally correct, not a stub.

---

## Human Verification Required

### 1. Mouse click navigation — visual confirmation

**Test:** Run `cargo run -- --mock`, click a nav item in the dashboard sidebar (e.g., "Key Management")
**Expected:** TUI navigates to the Keys screen; clicking the back button area returns to Dashboard
**Why human:** Pixel-accurate click targeting requires a live terminal; automated tests verify region registration and dispatch logic but not the final rendered pixel positions at real terminal dimensions

### 2. Popup click-through prevention

**Test:** Run `cargo run -- --mock`, open the context menu (Enter), click a menu item
**Expected:** Menu item activates (navigates to that screen); clicking outside the menu area does NOT activate a background nav item
**Why human:** Requires visual inspection of z-ordering behavior in a live terminal

### 3. Mouse scroll on list screens

**Test:** Run `cargo run -- --mock`, navigate to Keys screen, scroll mouse wheel up/down
**Expected:** Key list selection moves up/down in response to scroll events
**Why human:** Requires a mouse-capable terminal; scroll behavior confirmation needs live interaction

---

## Summary

Phase 7 goal is fully achieved. All infrastructure and behavior are in place:

**Mouse support (MOUSE-01 through MOUSE-04):** The ClickRegion type system exists in the model layer with zero ratatui coupling. All 7 screen render functions register click regions each frame. handle_mouse_event uses reverse-iteration dispatch (.iter().rev()) ensuring popups capture clicks before background elements. handle_scroll() dispatches to explicit match arms for all four list screens. Windows ConPTY graceful degradation is implemented.

**E2E test harness (TEST-01 through TEST-04):** 6 tmux smoke tests pass against --mock mode without hardware. wait_for_text retry logic eliminates timing brittleness. 15 insta snapshot tests cover all 7 screens with mock fixture and no-yubikey state variants. cargo test passes 109/109. Dashboard and SSH render functions are decoupled from &App, enabling testable rendering without a full App construction.

Three items are flagged for human verification — all involve live terminal interaction that cannot be automated without running hardware.

---

_Verified: 2026-03-26_
_Verifier: Claude (gsd-verifier)_
