---
phase: 09-oath-totp-screen
plan: "04"
subsystem: tui-navigation
tags: [oath, dashboard, navigation, snapshot-tests, pilot]
dependency_graph:
  requires: [09-02, 09-03]
  provides: [dashboard-oath-nav, oath-pilot-tests]
  affects: [src/tui/dashboard.rs, src/tui/oath.rs]
tech_stack:
  added: []
  patterns: [textual-rs-pilot-tests, insta-snapshots, push_screen_deferred]
key_files:
  created:
    - src/tui/snapshots/yubitui__tui__oath__tests__oath_default_state.snap
    - src/tui/snapshots/yubitui__tui__oath__tests__oath_navigate_down.snap
    - src/tui/snapshots/yubitui__tui__oath__tests__oath_no_credentials.snap
    - src/tui/snapshots/yubitui__tui__oath__tests__oath_password_protected.snap
  modified:
    - src/tui/dashboard.rs
    - src/tui/oath.rs
    - Cargo.toml
decisions:
  - "nav_7 uses same keybinding pattern as nav_1 through nav_6; nav_1 description updated to '1-7 Navigate'"
  - "Dashboard button label '[7] OATH / Authenticator' matches Yubico Authenticator branding"
  - "Cargo.toml textual-rs path fixed to ../../../../textual-rs/crates/textual-rs for worktree depth"
metrics:
  duration: "~15 minutes"
  completed: "2026-03-27"
  tasks_completed: 2
  tasks_total: 3
  files_modified: 5
---

# Phase 09 Plan 04: Dashboard Nav Wiring + Pilot Tests Summary

**One-liner:** Dashboard nav_7 key wires OathScreen via push_screen_deferred; 4 Pilot snapshot tests cover default, empty, password-protected, and Down-nav states.

## Tasks Completed

### Task 1: Wire OATH into dashboard navigation (commit: 4759b4c)

Added `nav_7` navigation to `src/tui/dashboard.rs`:
- New `KeyBinding { key: KeyCode::Char('7'), action: "nav_7" }` in `DASHBOARD_BINDINGS`
- Updated nav_1 description from `"1-6 Navigate"` to `"1-7 Navigate"`
- New `Button::new("[7] OATH / Authenticator")` in `compose()`
- Button press handler: `"[7] OATH / Authenticator" => "nav_7"` in `on_event()`
- `on_action("nav_7")` extracts `yk.oath.clone()` and calls `push_screen_deferred(OathScreen::new(...))`
- Fixed `Cargo.toml` textual-rs path for worktree (4 levels deep vs 1 for main repo)

Verification: `cargo check` passes with 0 errors.

### Task 2: Add Pilot snapshot tests for OATH screen (commit: 817ecfb)

Added 4 new tests to the `#[cfg(test)]` module in `src/tui/oath.rs`:

| Test | What it covers |
|------|----------------|
| `oath_default_state` | OathScreen with `mock_yubikey_states()` credentials |
| `oath_no_credentials` | Empty credential list |
| `oath_password_protected` | password_required=true renders password message |
| `oath_navigate_down` | Down key press moves selection; snapshot after move |

All 10 oath tests pass (`cargo test tui::oath::tests`). All 3 dashboard tests still pass.
4 new insta snapshot files accepted.

## Task 3: Human Verification (PENDING)

Task 3 is a `checkpoint:human-verify` — awaiting user to run `cargo run -- --mock` and verify:
1. Dashboard shows "[7] OATH / Authenticator" button
2. Pressing '7' opens OATH screen with 3 mock credentials
3. Countdown bar, Down-arrow navigation, Add ('a') and Delete ('d') wizards work
4. Visual style matches other screens

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Worktree missing oath.rs (09-01 through 09-03 work)**
- **Found during:** Initial setup
- **Issue:** Worktree branch was behind local main — lacked all phase 09-01 through 09-03 commits including `src/tui/oath.rs`
- **Fix:** `git merge local-main/main` to fast-forward worktree branch
- **Files modified:** Many (merged 28 files from 09-01 through 09-03 work)

**2. [Rule 3 - Blocking] textual-rs path wrong for worktree depth**
- **Found during:** Task 1 cargo check
- **Issue:** `Cargo.toml` had `../textual-rs/crates/textual-rs` which resolves correctly from main repo but not from worktree (which is 4 levels deep)
- **Fix:** Updated path to `../../../../textual-rs/crates/textual-rs`
- **Files modified:** `Cargo.toml`
- **Commit:** 4759b4c (included in Task 1 commit)

**3. [Rule 1 - Bug] `let app` must be `let mut app` in Pilot tests**
- **Found during:** Task 2 first compile
- **Issue:** Plan template used `let app` but `app.pilot()` requires `&mut self`
- **Fix:** Changed to `let mut app` and structured pilot usage consistently (some tests use `app.pilot().settle().await` inline, navigate_down uses `let mut pilot = app.pilot()`)
- **Files modified:** `src/tui/oath.rs`

## Known Stubs

None — all 4 tests render live OathScreen with real data paths; no hardcoded placeholder values.

## Self-Check: PENDING (awaiting Task 3 checkpoint)
