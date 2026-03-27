---
phase: 08-textual-rs-migration
plan: 01
subsystem: tui-infrastructure
tags: [dependencies, cleanup, ratatui, textual-rs, click-region-removal, e2e-removal]
dependency_graph:
  requires: []
  provides: [textual-rs-dependency, ratatui-0.30, click-region-free-codebase, no-tmux-e2e]
  affects: [all-tui-screens, app.rs, model/app_state.rs]
tech_stack:
  added:
    - "ratatui 0.30.0 (upgraded from 0.29) with crossterm_0_29 feature"
    - "crossterm 0.29.0 (upgraded from 0.28)"
    - "textual-rs git dep (jabberwock/textual-rs, v0.1.0 on master, v0.2.0 in changelog)"
  patterns:
    - "render() signatures simplified: click_regions parameter removed from all 7 screens"
key_files:
  modified:
    - Cargo.toml
    - Cargo.lock
    - src/model/app_state.rs
    - src/model/mod.rs
    - src/tui/mod.rs
    - src/tui/dashboard.rs
    - src/tui/diagnostics.rs
    - src/tui/help.rs
    - src/tui/keys.rs
    - src/tui/pin.rs
    - src/tui/piv.rs
    - src/tui/ssh.rs
    - src/app.rs
  deleted:
    - src/model/click_region.rs
    - tests/e2e/dashboard_smoke.sh
    - tests/e2e/diagnostics_smoke.sh
    - tests/e2e/helpers.sh
    - tests/e2e/keys_smoke.sh
    - tests/e2e/pin_smoke.sh
    - tests/e2e/piv_smoke.sh
    - tests/e2e/run_all.sh
    - tests/e2e/ssh_smoke.sh
decisions:
  - "Used git dep for textual-rs (not on crates.io) instead of crates.io version string"
  - "ratatui 0.30 had no breaking changes in yubitui codebase — cargo check passed immediately"
  - "MSRV bumped from 1.75 to 1.88 per textual-rs requirement"
metrics:
  duration: "~10 minutes"
  completed_date: "2026-03-27"
  tasks_completed: 3
  tasks_total: 3
  files_changed: 21
---

# Phase 8 Plan 01: Dependency Upgrade + ClickRegion Removal + E2E Deletion Summary

Upgraded to ratatui 0.30 + crossterm 0.29 + added textual-rs git dep, deleted ~290 lines of ClickRegion infrastructure across 12 files, and retired the 8-file tmux E2E harness.

## What Was Done

### Task 1: Cargo.toml Dependency Upgrade

- `ratatui = "0.29"` replaced with `ratatui = { version = "0.30.0", features = ["crossterm_0_29"] }`
- `crossterm = { version = "0.28", ... }` replaced with `crossterm = { version = "0.29.0", features = ["event-stream"] }`
- `textual-rs` added as git dependency (see deviation below)
- MSRV updated from `1.75` to `1.88`
- `cargo check` passed immediately — ratatui 0.30 had no breaking changes in this codebase

### Task 2: Delete ClickRegion Infrastructure

Deleted `src/model/click_region.rs` entirely (ClickRegion, ClickAction, Region types).

Removed from all affected files:
- `click_regions: Vec<ClickRegion>` field from `AppState`
- `pub mod click_region` from `model/mod.rs`
- `From<Rect> for Region` impl from `tui/mod.rs`
- `click_regions` parameter from all 7 screen `render()` functions
- All `click_regions.push(...)` call blocks (hit-region math)
- `execute_click_action()` and `handle_mouse_click()` from `app.rs`
- All test call sites updated to match new signatures

Net deletion: ~290 lines of click region math.

### Task 3: Delete tmux E2E Harness

Deleted `tests/e2e/` directory and all 8 shell scripts. No CI references to e2e found in `.github/workflows/`.

## Verification Results

```
cargo check   -> Finished (0 errors, 0 warnings relevant to changes)
cargo test    -> test result: ok. 109 passed; 0 failed
grep ClickRegion src/ -> no matches
tests/e2e/    -> does not exist
click_region.rs -> does not exist
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] textual-rs not published to crates.io**

- **Found during:** Task 1
- **Issue:** The plan specified `textual-rs = "0.2"` as a crates.io dependency. The crate `textual-rs` does not exist on crates.io. The GitHub repo `jabberwock/textual-rs` exists with the code, but has not been published to the registry. The Cargo.toml on master shows version `0.1.0`, though the CHANGELOG lists `0.2.0` as released 2026-03-26 (version number not yet bumped in the manifest).
- **Fix:** Used git dependency: `textual-rs = { git = "https://github.com/jabberwock/textual-rs", package = "textual-rs" }` — resolves correctly and the library compiles.
- **Files modified:** Cargo.toml
- **Commit:** 7802e33
- **Impact:** Cargo.lock now pins a specific git commit hash. Downstream plans that use textual-rs APIs will work correctly. When the crate is published to crates.io, the dependency line can be simplified to `textual-rs = "0.2"`.

**2. [Rule 1 - Bug] render_main in keys.rs still used old 5-argument signature**

- **Found during:** Task 2 (cargo test run)
- **Issue:** `render_ssh_pubkey_popup` in keys.rs called `render_main(frame, area, yubikey_state, state, &mut _dummy_regions)` with the old click_regions argument after the signature was simplified.
- **Fix:** Removed the `&mut _dummy_regions` argument and the dummy variable.
- **Files modified:** src/tui/keys.rs
- **Commit:** e7437c8

**3. [Rule 1 - Bug] Unused `popup_area` variable warning in dashboard.rs**

- **Found during:** Task 2 (cargo test run)
- **Issue:** After removing click_regions registration from the context menu block, `popup_area` was no longer used — `render_context_menu` returns a Rect that was only needed to register click targets.
- **Fix:** Renamed to `_popup_area` to suppress the warning.
- **Files modified:** src/tui/dashboard.rs
- **Commit:** e7437c8

## Known Stubs

None. This plan performed infrastructure cleanup only — no UI rendering logic was added or changed. All 109 existing tests pass unchanged.

## Self-Check: PASSED

| Check | Result |
|-------|--------|
| Cargo.toml exists | FOUND |
| src/model/click_region.rs deleted | FOUND (deleted) |
| tests/e2e/ deleted | FOUND (deleted) |
| Commit 7802e33 (dep upgrade) | FOUND |
| Commit e7437c8 (ClickRegion removal) | FOUND |
| Commit f3ed4d2 (E2E deletion) | FOUND |
| cargo check | Passes (0 errors) |
| cargo test | 109 passed, 0 failed |
| ClickRegion grep in src/ | No matches |
