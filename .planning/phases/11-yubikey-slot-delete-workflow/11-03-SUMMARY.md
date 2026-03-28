---
phase: 11-yubikey-slot-delete-workflow
plan: 03
subsystem: tui-education
tags: [tui, onboarding, factory-default, otp, keybinding, textual-rs]
dependency_graph:
  requires:
    - phase: 11-01
      provides: OTP screen (OtpScreen widget, nav 9)
    - phase: 11-02
      provides: per-screen ? help panels, GlossaryScreen
  provides:
    - src/model/onboarding.rs (is_factory_default heuristic)
    - src/tui/onboarding.rs (OnboardingScreen widget)
    - Dashboard startup wiring for factory-default detection
    - OTP screen ? help panel
    - ctx.quit() wired on dashboard via q key
    - q → back on all sub-screens
  affects: [src/tui/dashboard.rs, src/tui/onboarding.rs, all sub-screens]
tech_stack:
  added: []
  patterns: [textual-rs-widget-pattern, factory-default-heuristic, ctx-quit-pattern]
key_files:
  created:
    - src/model/onboarding.rs
    - src/tui/onboarding.rs
  modified:
    - src/model/mod.rs
    - src/tui/mod.rs
    - src/tui/dashboard.rs
    - src/tui/otp.rs
    - src/tui/diagnostics.rs
    - src/tui/fido2.rs
    - src/tui/glossary.rs
    - src/tui/help.rs
    - src/tui/keys.rs
    - src/tui/oath.rs
    - src/tui/onboarding.rs
    - src/tui/pin.rs
    - src/tui/piv.rs
    - src/tui/ssh.rs
key_decisions:
  - "Factory-default heuristic uses model data only (no extra PC/SC calls at startup): fido2.pin_is_set==false && oath.credentials.is_empty() && piv.slots.is_empty()"
  - "PIV management key AUTHENTICATE check replaced by slot-empty heuristic per research Pitfall 5 (double scdaemon kill at startup) — deferred to v2"
  - "ctx.quit() is the quit mechanism in textual-rs 0.3.5 (global q was removed in 0.3.3, ctx.quit() added in 0.3.5)"
  - "q → back on sub-screens is hidden (show: false) — Esc remains the visible binding"
  - "q → dismiss on onboarding (not quit) — onboarding is a first-screen, not a sub-screen"
patterns_established:
  - "ctx.quit() pattern: dashboard binds q → 'quit' action → ctx.quit() in on_action"
  - "Sub-screen q binding: hidden KeyBinding with action='back', show=false, no description"
requirements_completed: [EDU-03, EDU-04]
duration: "~45 min (including textual-rs 0.3.3→0.3.5 upgrade cycle)"
completed: "2026-03-28"
---

# Phase 11 Plan 03: Onboarding Screen + Factory-Default Detection + q-key Navigation Summary

**OnboardingScreen for factory-default YubiKeys, ctx.quit() on dashboard, q→back on all sub-screens, OTP ? help panel — textual-rs 0.3.5 upgrade resolved the quit API**

## Performance

- **Duration:** ~45 min
- **Completed:** 2026-03-28
- **Tasks:** 1 (+ follow-on q-key wiring after textual-rs upgrade)
- **Files modified:** 14

## Accomplishments

- `src/model/onboarding.rs` — `is_factory_default()` using PIV slot-empty + no FIDO2 PIN + zero OATH creds heuristic (no extra PC/SC calls)
- `src/tui/onboarding.rs` — `OnboardingScreen` with 4-item [x]/[ ] checklist (FIDO2 PIN, OATH, PIV, OpenPGP), Esc/Enter/q dismiss to dashboard
- Dashboard wiring: `show_onboarding` field computed in `DashboardScreen::new()`, pushed via `on_mount` when factory-default key detected
- OTP screen `?` help panel added (deferred from Plan 02 since the file didn't exist then)
- `ctx.quit()` wired on dashboard: `q` → `"quit"` → `ctx.quit()` (textual-rs 0.3.5 API)
- `q → back` (hidden binding) added to all 11 sub-screens: diagnostics, fido2, glossary, help, oath, onboarding (→dismiss), otp, pin (3 binding arrays), piv, ssh, keys (5 binding arrays)

## Task Commits

1. **Onboarding + factory-default + OTP help** — `b38e83d3` (feat(11-03))
2. **textual-rs 0.3.3 bump** — `f11378b1` (chore)
3. **Remove dead q→back (global q fired first)** — `f17495f9` (fix)
4. **Revert to 0.3.2** — `861136e2` (fix — 0.3.3 dropped global q with no replacement)
5. **Bump to 0.3.5 (ctx.quit() added)** — `07c4f7a1` (chore)
6. **Wire ctx.quit() + q→back (this session)** — pending commit

## Files Created/Modified

- `src/model/onboarding.rs` — `is_factory_default(yk: &YubiKeyState) -> bool` with unit tests
- `src/tui/onboarding.rs` — `OnboardingScreen` widget, 4-item checklist, dismiss → push DashboardScreen
- `src/model/mod.rs` — added `pub mod onboarding;`
- `src/tui/mod.rs` — added `pub mod onboarding;`
- `src/tui/dashboard.rs` — `show_onboarding` field, `on_mount` onboarding push, `q` → `ctx.quit()`, updated stale comment
- `src/tui/otp.rs` — `OTP_HELP_TEXT` const, `?` → `"help"` binding, popup push, `q` hidden back binding
- All sub-screens (diagnostics, fido2, glossary, help, keys ×5, oath, pin ×3, piv, ssh) — hidden `q` → `back` binding

## Decisions Made

- **PIV heuristic over AUTHENTICATE**: Management key AUTHENTICATE check skipped at startup to avoid double scdaemon kill (research Pitfall 5). Slot-empty check is a good-enough proxy for factory-default detection. Full check deferred to v2.
- **None fields → false**: If fido2/oath/piv are `None` (device not fully probed), that condition returns false — conservative behavior, avoids false-positive onboarding.
- **textual-rs 0.3.3 revert**: 0.3.3 removed global `q` quit but didn't ship `ctx.quit()` yet — reverted to 0.3.2 to preserve quit. Upgraded to 0.3.5 once `ctx.quit()` was available.
- **q hidden on sub-screens**: `show: false` so footer doesn't show both "Esc Back" and "q Back". Esc remains the canonical visible binding.
- **q → dismiss on onboarding**: Onboarding is a first-screen that pushes dashboard — q should dismiss (same as Esc), not quit the app.

## Deviations from Plan

### Auto-fixed Issues

**1. textual-rs quit API — q-key wiring deferred then unblocked**
- **Found during:** Task 1 execution
- **Issue:** Plan assumed global `q` quit existed. Upgrading to 0.3.3 broke quit (global q removed, no ctx.quit() yet). Reverted to 0.3.2.
- **Fix:** Waited for textual-rs 0.3.5 which added `ctx.quit()`. Then wired dashboard `q` → `ctx.quit()` and sub-screen `q` → `back` as planned.
- **Files modified:** Cargo.toml (textual-rs version), src/tui/dashboard.rs + all sub-screens
- **Verification:** 150/150 tests pass

---

**Total deviations:** 1 (textual-rs API gap — resolved by upgrading to 0.3.5)
**Impact on plan:** No scope creep. All q-key wiring now complete.

## Issues Encountered

- textual-rs 0.3.3 removed global q without a replacement → reverted to 0.3.2 and filed internally. 0.3.5 shipped ctx.quit() which unblocked the full implementation.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Phase 11 complete: OTP screen, per-screen help, glossary, onboarding, q-key navigation all wired
- All sub-screens have consistent keyboard behavior: Esc = back (visible), q = back (hidden), ? = help
- Dashboard: q = quit (visible), ? = glossary
- Ready for Phase 12 (delete/reset workflows) or any new phase

---
*Phase: 11-yubikey-slot-delete-workflow*
*Completed: 2026-03-28*
