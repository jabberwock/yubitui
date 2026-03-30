---
status: diagnosed
trigger: "YubiKey is detected and data loads on dashboard screen, but subsequent screens (KeysScreen, PivScreen, etc.) cannot access card data"
created: 2026-03-29T00:00:00Z
updated: 2026-03-29T00:00:00Z
---

## Current Focus

hypothesis: confirmed — see Resolution
test: N/A (research-only mode)
expecting: N/A
next_action: return diagnosis to caller

## Symptoms

expected: KeysScreen, PivScreen, OathScreen etc. display card data after navigating from Dashboard
actual: Only the first screen (dashboard) recognized the YubiKey; subsequent screens show "No YubiKey" or stale/empty data
errors: none reported (silent failure)
reproduction: launch app, navigate from dashboard to any sub-screen (Keys, PIV, OATH, etc.)
started: unknown; relates to Phase 12 architecture

## Eliminated

- hypothesis: exclusive PC/SC session left open by dashboard blocking sub-screens
  evidence: commit c51fe519 already drops the exclusive card connection before PIV/OTP detection in detect_all_yubikey_states(); by the time the TUI is running, no PC/SC handle is held
  timestamp: 2026-03-29

- hypothesis: scdaemon not restarted after detection, blocking subsequent access
  evidence: detect_all_yubikey_states() explicitly restarts scdaemon at line 195; individual get_piv_state() calls also kill+restart scdaemon; this is not the bottleneck
  timestamp: 2026-03-29

## Evidence

- timestamp: 2026-03-29
  checked: src/app.rs lines 15-25
  found: YubiKeyState::detect_all() is called ONCE at startup; result is stored in AppState.yubikey_states
  implication: all card data is a frozen snapshot taken at startup; no re-detection ever happens

- timestamp: 2026-03-29
  checked: src/tui/dashboard.rs on_action() lines 280-321
  found: when navigating to a sub-screen (e.g. nav_1 = Keys), dashboard passes `self.app_state.yubikey_state().cloned()` — a clone of the YubiKeyState from startup
  implication: sub-screens receive the snapshot YubiKeyState; if card data was collected at startup, it should appear

- timestamp: 2026-03-29
  checked: src/model/detection.rs lines 179-180
  found: YubiKeyState.oath = None and YubiKeyState.fido2 = None are INTENTIONALLY not populated at startup ("OATH detection is expensive — only fetched on-demand")
  implication: OathScreen and Fido2Screen always receive None; those screens will always show empty/no-data regardless of card state

- timestamp: 2026-03-29
  checked: src/tui/dashboard.rs nav_7 / nav_8 actions (lines 309-317)
  found: OathScreen is passed `yk.oath.clone()` which is always None; Fido2Screen is passed `yk.fido2.clone()` which is always None
  implication: OATH and FIDO2 screens structurally cannot show data — the on-demand fetch is not implemented

- timestamp: 2026-03-29
  checked: src/tui/keys.rs on_action "refresh" (line 572-574)
  found: "refresh" action is a no-op — comment says "Refresh is an app-level side effect — no-op in widget scope."
  implication: pressing R on KeysScreen does not re-read card; state is frozen forever at startup

- timestamp: 2026-03-29
  checked: src/tui/dashboard.rs on_action "refresh" (lines 323-327)
  found: "refresh" on DashboardScreen is also a no-op with same comment
  implication: there is no implemented refresh path anywhere in the TUI

- timestamp: 2026-03-29
  checked: src/tui/piv.rs on_action "refresh" (line 303-306)
  found: PivScreen "refresh" calls ctx.pop_screen_deferred() — it pops itself and returns to dashboard; it does NOT re-fetch PIV data
  implication: PivScreen cannot update its PIV data without a full app restart

- timestamp: 2026-03-29
  checked: src/tui/piv.rs DeletePivConfirmScreen on_action "confirm" lines 558-566
  found: after successful delete, code calls get_piv_state().ok() to get fresh PIV state, then discards it ("let _ = fresh_piv_state") and pushes PivScreen::new(None), acknowledging "limited context here"
  implication: after delete, PivScreen always shows None yubikey_state, displaying "PIV data unavailable"

- timestamp: 2026-03-29
  checked: src/model/piv.rs get_piv_state() lines 39-41
  found: get_piv_state() calls kill_scdaemon() and sleeps 50ms every time it is called
  implication: even when PIV state IS re-fetched (e.g. after delete), it's functional — but the result is thrown away

## Resolution

root_cause: |
  There are TWO root causes operating at different layers:

  1. PRIMARY — Frozen state snapshot with no refresh path:
     AppState is populated once in app.rs:18 via YubiKeyState::detect_all(), then cloned into
     every sub-screen at navigation time via dashboard.rs on_action(). The "refresh" action is
     a no-op stub in every screen (dashboard.rs:323, keys.rs:572, piv.rs:303 pops without
     re-fetching). There is no mechanism to re-run detect_all_yubikey_states() and update
     AppState while the TUI is running. If the YubiKey was detected correctly at startup, the
     data IS passed to sub-screens — but it is a frozen clone from app startup with no ability
     to update.

  2. SECONDARY — OATH and FIDO2 are intentionally None and the on-demand fetch is unimplemented:
     YubiKeyState.oath and .fido2 are always None after detection (detection.rs:182-183 comments
     confirm "only fetched on-demand"). Dashboard nav_7/nav_8 pass yk.oath.clone() and
     yk.fido2.clone() to OathScreen/Fido2Screen — these are always None. The on-demand fetch
     is never triggered anywhere. OathScreen and Fido2Screen will ALWAYS show empty regardless
     of what is on the card.

  3. TERTIARY — Post-operation refresh discards freshly fetched data:
     In piv.rs DeletePivConfirmScreen::on_action "confirm" (line 558-566), fresh PIV state IS
     fetched (get_piv_state().ok()) but immediately discarded ("let _ = fresh_piv_state") and
     PivScreen::new(None) is pushed instead, so the PIV screen always shows "PIV data
     unavailable" after a delete operation.

fix: |
  Not applicable (research-only mode). Suggested approaches:

  1. For the frozen-state problem: app.rs needs to hold AppState behind an Arc<Mutex<>> or
     equivalent shared handle, and detect_all_yubikey_states() needs to be callable from
     within the TUI event loop. The "refresh" stubs need to trigger actual re-detection and
     propagate the updated AppState to the current screen. Alternatively, each screen that
     navigates back to dashboard should cause DashboardScreen to re-initialize with fresh state.

  2. For OATH/FIDO2: on-demand fetch must be triggered on OathScreen/Fido2Screen mount
     (on_mount hook or first compose()), running the fetch in a background thread and updating
     screen state via a channel or reactive variable.

  3. For post-delete PIV: DeletePivConfirmScreen should pass the freshly fetched PivState into
     a new YubiKeyState (or at minimum a new PivScreen with the fresh data) rather than
     discarding it and pushing PivScreen::new(None).

verification: N/A — research-only
files_changed: []
