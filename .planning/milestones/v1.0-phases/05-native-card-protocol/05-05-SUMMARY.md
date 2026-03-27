---
phase: 05-native-card-protocol
plan: 05
subsystem: ui
tags: [ratatui, touch-policy, ssh-wizard, key-attributes]

# Dependency graph
requires:
  - phase: 05-native-card-protocol
    provides: YubiKeyState with touch_policies field from PC/SC native reads

provides:
  - render_key_attributes displays touch policies per slot when YubiKeyState.touch_policies is populated
  - SSH Wizard shows accurate status on initial open via key '5' and dashboard menu

affects: [ui, ssh-wizard, key-attributes]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Pass YubiKeyState through render dispatch chain to sub-renderers needing it
    - Call status refresh functions on screen entry, not only on action execute

key-files:
  created: []
  modified:
    - src/ui/keys.rs
    - src/app.rs

key-decisions:
  - "render_key_attributes now receives &Option<YubiKeyState> — same pattern as render_ssh_pubkey_popup; no new imports needed"
  - "refresh_ssh_status() called at both entry points (Char('5') and dashboard menu arm) without resetting SshState fields — preserves sub-screen state"

patterns-established:
  - "Pattern: screen-entry refresh — call status refresh at navigation time so first render is accurate"

requirements-completed: [NATIVE-PCSC-01]

# Metrics
duration: 8min
completed: 2026-03-26
---

# Phase 5 Plan 05: Touch Policy Display + SSH Status Fix Summary

**Touch policy per-slot display added to [K] Key Attributes screen; SSH Wizard status refreshed on entry via Char('5') and dashboard menu**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-03-26T05:47:22Z
- **Completed:** 2026-03-26T05:55:00Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- render_key_attributes updated to accept YubiKeyState and render Touch Policies section (Signature/Encryption/Authentication/Attestation) matching the identical layout in render_main
- SSH Wizard refresh_ssh_status() now called at both entry points so initial render shows real gpg-agent.conf / SSH_AUTH_SOCK / agent status
- Section absent (no crash) when touch_policies is None (YubiKey 4 or no card)

## Task Commits

Each task was committed atomically:

1. **Task 1: Pass YubiKeyState to render_key_attributes, display touch policies** - `616b2fc` (feat)
2. **Task 2: Call refresh_ssh_status on every Screen::SshWizard entry** - `844f30f` (fix)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `src/ui/keys.rs` - Updated render() dispatch and render_key_attributes signature; added touch policy section
- `src/app.rs` - Added refresh_ssh_status() calls in Char('5') arm and dashboard menu Enter arm

## Decisions Made

- render_key_attributes follows same parameter-passing pattern as render_ssh_pubkey_popup — no new import needed since YubiKeyState is already in scope at the top of keys.rs
- refresh_ssh_status() is called after setting current_screen, before returning from the key handler — matches the existing pattern used in the SshWizard '5' keybind inside the SshWizard handler

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - both changes were straightforward and the existing code structure made them easy to apply.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All gap closure plans for Phase 05 are now complete
- Touch policies visible in both Key Management main screen and Key Attributes screen
- SSH Wizard ready for UAT — accurate status on first open

---
*Phase: 05-native-card-protocol*
*Completed: 2026-03-26*
