---
phase: 05-native-card-protocol
plan: "04"
subsystem: ui
tags: [rust, ratatui, gpg, yubikey, key-management, error-handling]

# Dependency graph
requires:
  - phase: 05-03
    provides: key import workflow with programmatic gpg session and ScOpFailure reporting

provides:
  - ScOpFailure(6) mapped to "Wrong Admin PIN" in status_to_message
  - CardCtrl(3) surfaced as message during import failure
  - ViewStatus routes to KeyOperationResult screen on success
  - ExportSSH error routes to SshPubkeyPopup with None ssh_pubkey
  - v/k/e/s/a navigation arms clear stale key_state.message

affects:
  - 05-05
  - 05-06

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Explicit ScOpFailure variant arms before catch-all for specific error codes"
    - "CardCtrl(3) handled as distinct case in import session to surface card-removal"
    - "Navigation arms clear stale message before screen transition"

key-files:
  created: []
  modified:
    - src/yubikey/gpg_status.rs
    - src/yubikey/key_operations.rs
    - src/app.rs

key-decisions:
  - "[05-04]: ExportSSH Err arm sets ssh_pubkey=None and routes to SshPubkeyPopup — renderer already handles None with 'No authentication key found on card.'"
  - "[05-04]: ViewStatus Err still routes to Main (error in status bar); only Ok routes to KeyOperationResult for full status display"
  - "[05-04]: CardCtrl(3) gets its own arm in run_keytocard_session; other CardCtrl codes remain in catch-all (scdaemon noise)"

patterns-established:
  - "Pattern: Match specific failure codes before catch-all arms in GpgStatus match blocks"

requirements-completed: [NATIVE-PCSC-01]

# Metrics
duration: 3min
completed: 2026-03-26
---

# Phase 5 Plan 4: Key Import Error Reporting and Navigation Bug Fixes Summary

**Key import distinguishes Wrong Admin PIN from card-removed; [V] routes to result screen; [E] error shows popup; stale messages cleared on navigation**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-03-26T05:50:30Z
- **Completed:** 2026-03-26T05:53:31Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- `ScOpFailure(6)` now returns "Wrong Admin PIN" from `status_to_message` (was falling through to generic "Smartcard operation failed")
- `CardCtrl(3)` during import is explicitly matched and pushed to messages (card-removed signal is now visible)
- `[V] ViewStatus` success path routes to `KeyScreen::KeyOperationResult` so card status text is visible; error path still routes to Main
- `[E] ExportSSH` error path sets `ssh_pubkey = None` and routes to `SshPubkeyPopup` (renderer shows "No authentication key found on card.")
- Navigation keys `v/k/e/s/a` each clear `key_state.message` before changing screen, preventing stale error bleed-through
- 2 new unit tests added for `ScOpFailure(6)` and `ScOpFailure(99)` message coverage; all 87 tests pass

## Task Commits

1. **Task 1: Fix ScOpFailure(6) mapping and CardCtrl(3) surfacing** - `6739cbe` (fix)
2. **Task 2: Fix [V] ViewStatus route, [E] ExportSSH error route, stale message clear** - `f3de3df` (fix)

## Files Created/Modified

- `src/yubikey/gpg_status.rs` - Added `ScOpFailure(6)` arm returning "Wrong Admin PIN"; 2 new unit tests
- `src/yubikey/key_operations.rs` - Added explicit `CardCtrl(3)` arm in `run_keytocard_session`
- `src/app.rs` - Fixed ViewStatus/ExportSSH routing; added `message = None` clears in v/k/e/s/a nav arms

## Decisions Made

- ExportSSH Err arm sets `ssh_pubkey = None` and routes to `SshPubkeyPopup` — the renderer already handles `None` with "No authentication key found on card." so no new UI code needed
- ViewStatus Err still routes to Main (error stays in status bar); only success routes to KeyOperationResult
- CardCtrl(3) gets its own explicit arm in the import session match; other CardCtrl codes remain in the catch-all (they are scdaemon connection noise)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Key import error reporting is now unambiguous: wrong admin PIN and card disconnect are distinct messages
- Navigation UX is clean: no stale errors bleed through when switching between key operations
- Ready for Plan 05-05 (fingerprint detection fix / model display improvement)

---
*Phase: 05-native-card-protocol*
*Completed: 2026-03-26*

## Self-Check: PASSED

- SUMMARY.md exists at expected path
- Commit 6739cbe (Task 1) confirmed in git log
- Commit f3de3df (Task 2) confirmed in git log
