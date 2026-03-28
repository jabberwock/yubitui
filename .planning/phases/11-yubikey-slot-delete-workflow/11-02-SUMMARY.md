---
phase: 11-yubikey-slot-delete-workflow
plan: 02
subsystem: tui-education
tags: [tui, help, glossary, education, keybinding]
dependency_graph:
  requires: [11-01]
  provides: [src/tui/glossary.rs]
  affects: [src/tui/mod.rs, src/tui/dashboard.rs, src/tui/fido2.rs, src/tui/oath.rs, src/tui/piv.rs, src/tui/keys.rs, src/tui/pin.rs, src/tui/ssh.rs, src/tui/diagnostics.rs]
tech_stack:
  added: []
  patterns: [textual-rs-widget-pattern, modal-popup-pattern, insta-snapshot-tests]
key_files:
  created:
    - src/tui/glossary.rs
    - src/tui/snapshots/yubitui__tui__glossary__tests__glossary_screen.snap
  modified:
    - src/tui/mod.rs
    - src/tui/dashboard.rs
    - src/tui/fido2.rs
    - src/tui/oath.rs
    - src/tui/piv.rs
    - src/tui/keys.rs
    - src/tui/pin.rs
    - src/tui/ssh.rs
    - src/tui/diagnostics.rs
decisions:
  - "GlossaryScreen uses same Widget pattern as HelpScreen (compose + key_bindings + on_action) — no new widget types"
  - "Help popups use existing ModalScreen+PopupScreen pattern — no new infrastructure"
  - "? on GlossaryScreen closes the glossary (same as HelpScreen) — prevents infinite recursion"
metrics:
  duration: "~7 minutes"
  completed: "2026-03-28"
  tasks_completed: 2
  files_modified: 9
  files_created: 2
---

# Phase 11 Plan 02: Per-Screen Help Panels + Protocol Glossary Summary

GlossaryScreen with 8 protocol explanations (PIV/FIDO2/FIDO U2F/OpenPGP/SSH/TOTP/HOTP/OTP) + ? keybinding on all 7 non-dashboard screens pushing contextual ModalScreen+PopupScreen help overlays.

## What Was Built

### Task 1: GlossaryScreen widget + dashboard ? keybinding

- **`src/tui/glossary.rs`** — `GlossaryScreen` widget following HelpScreen pattern: `compose()` with Header + 8 protocol explanation blocks (Label sequences) + Footer; `key_bindings()` with Esc and ? both mapped to "back"; `on_action()` with `ctx.pop_screen_deferred()`; snapshot test `glossary_screen`
- **`src/tui/mod.rs`** — Added `pub mod glossary;` after `pub mod fido2;`
- **`src/tui/dashboard.rs`** — Added `?` → `"glossary"` to `DASHBOARD_BINDINGS` static; added `"glossary"` arm in `on_action()` pushing `GlossaryScreen::new()`; `nav_6` → HelpScreen unchanged

### Task 2: Add ? help keybinding to all 7 non-dashboard screens

For each of 7 screens, applied 3-step pattern: (A) help text const, (B) ? KeyBinding in key_bindings array, (C) "help" arm in on_action pushing ModalScreen+PopupScreen:

- **`src/tui/fido2.rs`** — `FIDO2_HELP_TEXT`: passkeys, PIN, credential management; popup "FIDO2 Help"
- **`src/tui/oath.rs`** — `OATH_HELP_TEXT`: TOTP/HOTP, hardware storage, live codes; popup "OATH Help"
- **`src/tui/piv.rs`** — `PIV_HELP_TEXT`: X.509 slots 9a/9c/9d/9e with use cases; popup "PIV Help"
- **`src/tui/keys.rs`** — `KEYS_HELP_TEXT`: SIG/ENC/AUT slots, SSH via gpg-agent; popup "OpenPGP Keys Help"
- **`src/tui/pin.rs`** — `PIN_HELP_TEXT`: User/Admin PINs, defaults, unblock procedure; popup "PIN Management Help"
- **`src/tui/ssh.rs`** — `SSH_HELP_TEXT`: gpg-agent SSH auth, wizard steps; popup "SSH Wizard Help"
- **`src/tui/diagnostics.rs`** — `DIAGNOSTICS_HELP_TEXT`: PC/SC, GPG, gpg-agent checks; popup "Diagnostics Help"

HelpScreen and GlossaryScreen excluded from ? binding (they close on ? instead).

## Verification

- `cargo check` passes (90 pre-existing warnings, 0 errors)
- `cargo test` passes — 144 tests, 0 failures (snapshot tests use blank-space pattern, pass without update)
- `?` on dashboard pushes GlossaryScreen (8 protocols)
- `?` on fido2/oath/piv/keys/pin/ssh/diagnostics pushes per-screen help popup
- `6` on dashboard still pushes HelpScreen (nav_6 unchanged)
- HelpScreen and GlossaryScreen do NOT have `?` help action (only Esc to close)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Merged local-main/main into worktree before starting**
- **Found during:** Initialization
- **Issue:** This worktree (agent-a4192512) branched from commit `0641761` (phase 08), predating the 11-01 OTP changes (fido2.rs, oath.rs, otp.rs, mod.rs, dashboard.rs nav_7/8/9). Without merging, plan 02 would have modified stale files.
- **Fix:** `git fetch local-main && git merge local-main/main --no-edit --no-verify` — fast-forward to `5856c0c`
- **Commit:** (merge, no separate commit — fast-forward)

## Known Stubs

None. All 7 help texts are real educational content. GlossaryScreen renders all 8 protocol explanations. No placeholder text flows to UI rendering.

## Self-Check: PASSED
