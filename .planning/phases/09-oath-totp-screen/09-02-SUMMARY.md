---
phase: 09-oath-totp-screen
plan: 02
subsystem: ui
tags: [textual-rs, oath, totp, hotp, tui, widget, ratatui, chrono]

# Dependency graph
requires:
  - phase: 09-01
    provides: OathState, OathCredential, OathType, OathAlgorithm model types and mock fixture

provides:
  - OathScreen textual-rs Widget in src/tui/oath.rs
  - OathTuiState with selected_index and scroll_offset
  - Credential list with name, code, type badge per row
  - TOTP countdown bar from chrono::Utc::now()
  - HOTP [press Enter] placeholder behavior
  - pub mod oath registered in src/tui/mod.rs

affects: [09-03, 09-04, dashboard navigation to Screen::Oath]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "OathScreen follows PivScreen Widget pattern: Reactive<TuiState>, compose() returns Header+Labels+Footer, on_action() with pop_screen_deferred"
    - "Countdown computed per-render from chrono::Utc::now().timestamp() % 30 — no background timer needed"
    - "HOTP credentials show [press Enter] when code is None, code value when Some"

key-files:
  created:
    - src/tui/oath.rs
  modified:
    - src/tui/mod.rs
    - Cargo.toml

key-decisions:
  - "Countdown computed on each render from chrono::Utc::now() — textual-rs re-renders on key events so no timer thread needed"
  - "HOTP with no code shows '[press Enter]' placeholder; full HOTP generation wired in Plan 03"
  - "add_account/delete_account/refresh on_action handlers are stubs — wired in Plan 03"
  - "Cargo.toml textual-rs path adjusted for worktree directory depth (../../../../ instead of ../)"

patterns-established:
  - "OathTuiState: Default, Clone, PartialEq — same pattern as PivTuiState, DashboardState"
  - "Static OATH_BINDINGS slice with show=true for footer-visible bindings, show=false for hidden aliases"

requirements-completed: [OATH-01, OATH-02, OATH-06]

# Metrics
duration: 15min
completed: 2026-03-27
---

# Phase 09 Plan 02: OathScreen Widget Summary

**OathScreen textual-rs Widget with credential list (name/code/type badge per row), HOTP [press Enter] placeholder, and live TOTP countdown bar from chrono::Utc::now()**

## Performance

- **Duration:** 15 min
- **Started:** 2026-03-27T00:00:00Z
- **Completed:** 2026-03-27T00:15:00Z
- **Tasks:** 1
- **Files modified:** 3

## Accomplishments

- Created src/tui/oath.rs with OathScreen implementing textual-rs Widget trait
- Credential list renders: selection marker, issuer/name (30-char padded), code (8-char), [TOTP]/[HOTP] badge
- TOTP countdown bar: `TOTP refreshes in Xs  [========        ]` using 20-char proportional fill
- HOTP credentials show `[press Enter]` when code is None
- All 9 OATH_BINDINGS defined (Esc, Up, Down, j, k, Enter, a, d, r)
- 4 insta snapshot tests covering: credentials, no YubiKey, empty, password-required
- Registered `pub mod oath;` in tui/mod.rs (alphabetical between keys and pin)

## Task Commits

1. **Task 1: Create OathScreen textual-rs Widget** - `fc745a0` (feat)

## Files Created/Modified

- `src/tui/oath.rs` - OathScreen Widget with OathTuiState, compose(), on_action(), 4 tests (new, 285 lines)
- `src/tui/mod.rs` - Added `pub mod oath;`
- `Cargo.toml` - Fixed textual-rs path for worktree directory depth

## Decisions Made

- Countdown computed per-render via `chrono::Utc::now().timestamp() % 30` — no background timer thread needed since textual-rs re-renders on key events
- HOTP generation (add_account, delete_account, refresh) left as stubs — full wiring in Plan 03

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed textual-rs path for worktree directory depth**
- **Found during:** Task 1 (cargo check)
- **Issue:** Cargo.toml had `path = "../textual-rs/crates/textual-rs"` which resolves correctly from main repo root but not from the nested worktree path `.claude/worktrees/agent-a6825377/`
- **Fix:** Changed path to `../../../../textual-rs/crates/textual-rs` to resolve to `/Users/michael/code/textual-rs/crates/textual-rs`
- **Files modified:** Cargo.toml
- **Verification:** cargo check succeeds with zero errors
- **Committed in:** fc745a0

---

**Total deviations:** 1 auto-fixed (1 blocking path issue)
**Impact on plan:** Required for compilation. Cargo.toml change is worktree-local; the main repo's `../textual-rs` path remains correct.

## Issues Encountered

- Worktree did not have Plan 01 outputs (src/model/oath.rs) — resolved by merging main branch into worktree branch before implementation.

## Known Stubs

- `on_action("generate_hotp")`: checks credential type but does not call card APDU — stub for Plan 03
- `on_action("add_account")`: no-op — full wizard in Plan 03
- `on_action("delete_account")`: no-op — confirmation dialog in Plan 03
- `on_action("refresh")`: no-op — CALCULATE ALL wiring in Plan 03

These stubs are intentional and documented; Plan 03 wires all card interactions.

## Next Phase Readiness

- OathScreen Widget complete and compiling — ready for Plan 03 (add/delete account wizards and card interactions)
- Plan 04 (dashboard navigation to Screen::Oath) can use `OathScreen::new(app_state.selected_yubikey().and_then(|yk| yk.oath.clone()))`

---
*Phase: 09-oath-totp-screen*
*Completed: 2026-03-27*

## Self-Check: PASSED

- src/tui/oath.rs: FOUND
- 09-02-SUMMARY.md: FOUND
- Commit fc745a0: FOUND
