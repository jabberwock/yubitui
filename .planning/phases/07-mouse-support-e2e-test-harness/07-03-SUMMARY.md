---
phase: 07-mouse-support-e2e-test-harness
plan: "03"
subsystem: testing
tags: [tmux, bash, e2e, smoke-tests, mock-mode, wait_for_text]

# Dependency graph
requires:
  - phase: 07-mouse-support-e2e-test-harness
    provides: "--mock flag and mock data fixture in model/mock.rs"

provides:
  - "tests/e2e/ directory with 6 screen smoke tests"
  - "helpers.sh with wait_for_text retry polling function"
  - "run_all.sh driver that aggregates pass/fail results"
  - "TEST-03 E2E harness pattern for future phases"

affects:
  - "07-04 (CI workflow that will run tests/e2e/run_all.sh)"
  - "All future test plans needing E2E coverage"

# Tech tracking
tech-stack:
  added: ["tmux (via send-keys/capture-pane for headless TUI testing)"]
  patterns:
    - "wait_for_text: polls tmux capture-pane every 0.3s with configurable timeout instead of fixed sleeps"
    - "Session name includes $$ (PID) to prevent collision in parallel runs"
    - "run_all.sh: single binary build shared across all test scripts via BINARY env var"
    - "All tests use --mock mode: no hardware or external services required"

key-files:
  created:
    - "tests/e2e/helpers.sh"
    - "tests/e2e/run_all.sh"
    - "tests/e2e/dashboard_smoke.sh"
    - "tests/e2e/diagnostics_smoke.sh"
    - "tests/e2e/keys_smoke.sh"
    - "tests/e2e/pin_smoke.sh"
    - "tests/e2e/ssh_smoke.sh"
    - "tests/e2e/piv_smoke.sh"
  modified: []

key-decisions:
  - "wait_for_text retry loop (0.3s poll) replaces fixed sleep+assert pattern — eliminates CI timing races"
  - "Menu navigation uses index-based Down arrow sequences matching execute_dashboard_action match arms (0=Diagnostics, 1=Keys, 2=PIN, 3=SSH, 4=PIV)"
  - "Back navigation via Esc key confirmed from each screen's handle_key function"
  - "Dashboard text assertion uses 'Navigation' block title (always visible) rather than 'Dashboard' which only appears in title bar"

patterns-established:
  - "E2E pattern: start_session -> wait_for_text startup -> navigate -> wait_for_text content -> navigate back -> cleanup -> echo PASS"
  - "All smoke scripts source helpers.sh, call build_if_needed, use unique SESSION with $$"

requirements-completed: [TEST-01, TEST-02, TEST-03]

# Metrics
duration: 3min
completed: 2026-03-27
---

# Phase 07 Plan 03: E2E Test Harness Summary

**tmux-based E2E harness with 6 screen smoke tests using wait_for_text retry polling — all pass against --mock mode without hardware**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-27T03:18:59Z
- **Completed:** 2026-03-27T03:22:00Z
- **Tasks:** 2
- **Files modified:** 8 created

## Accomplishments
- helpers.sh: shared functions including wait_for_text that polls tmux capture-pane every 0.3s with configurable timeout (no brittle fixed sleeps)
- 6 screen smoke tests: dashboard, diagnostics, keys, pin, ssh, piv — each navigates to its screen, asserts content, navigates back
- run_all.sh: builds binary once, runs all *_smoke.sh scripts, exits non-zero on any failure
- All 6 tests pass: `bash tests/e2e/run_all.sh` reports "6 passed, 0 failed"

## Task Commits

Each task was committed atomically:

1. **Task 1: E2E harness driver, helpers with wait_for_text, and dashboard smoke test** - `aaca63d` (feat)
2. **Task 2: Smoke tests for remaining 5 screens** - `9f0f47e` (feat)

## Files Created/Modified
- `tests/e2e/helpers.sh` - Shared functions: build_if_needed, start_session, capture, wait_for_text, assert_contains, cleanup
- `tests/e2e/run_all.sh` - Driver: builds binary, loops over *_smoke.sh scripts, aggregates pass/fail
- `tests/e2e/dashboard_smoke.sh` - Dashboard: asserts YubiKey info + context menu opens/closes
- `tests/e2e/diagnostics_smoke.sh` - Diagnostics: menu index 0, asserts "System Diagnostics"
- `tests/e2e/keys_smoke.sh` - Keys: menu index 1, asserts "Key Management"
- `tests/e2e/pin_smoke.sh` - PIN: menu index 2, asserts "PIN Management"
- `tests/e2e/ssh_smoke.sh` - SSH: menu index 3, asserts "SSH"
- `tests/e2e/piv_smoke.sh` - PIV: menu index 4, asserts "PIV"

## Decisions Made
- wait_for_text polls every 0.3s rather than fixed sleep — eliminates timing races on slower CI runners
- Confirmed menu indices (0=Diagnostics, 1=Keys, 2=PIN, 3=SSH, 4=PIV) from app.rs execute_dashboard_action match
- Back navigation verified via Esc key confirmed in each screen's handle_key() function
- Dashboard "back" assertion uses "Navigation" (block title always present) not "Dashboard" (appears only in title bar text, may be truncated)

## Deviations from Plan

None — plan executed exactly as written. All navigation keys and assertions confirmed against actual source code before writing tests.

## Issues Encountered
None.

## User Setup Required
None — no external service configuration required. Tests run against --mock mode.

## Next Phase Readiness
- E2E harness is complete; TEST-03 pattern established for future phases
- tests/e2e/run_all.sh is ready to be added to CI in plan 07-04
- Each smoke test is independently runnable: `bash tests/e2e/keys_smoke.sh`

---
*Phase: 07-mouse-support-e2e-test-harness*
*Completed: 2026-03-27*
