---
phase: 04-programmatic-subprocess-control
plan: "04"
subsystem: yubikey-subprocess, tui-ssh, tui-keys
tags: [no-terminal-escape, stdio-piped, ssh-test-tui, touch-policy-programmatic, deprecated-removal]
dependency_graph:
  requires: [04-02, 04-03]
  provides: [complete-no-escape-guarantee, tui-ssh-test-connection, programmatic-touch-policy]
  affects: []
tech_stack:
  added: []
  patterns: [all-operations-piped-io, tui-text-input-for-connection-params, Result-String-over-Child]
key_files:
  created: []
  modified:
    - src/yubikey/touch_policy.rs
    - src/yubikey/ssh_operations.rs
    - src/yubikey/pin_operations.rs
    - src/yubikey/key_operations.rs
    - src/ui/ssh.rs
    - src/app.rs
key_decisions:
  - "set_touch_policy returns Result<String> (not Result<Child>) — callers no longer call child.wait() and the escape-to-terminal dance is eliminated"
  - "test_ssh_connection uses BatchMode=yes + ConnectTimeout=10 + Stdio::piped — non-interactive, never hangs, all output captured for TUI display"
  - "SSH ExportKey in execute_ssh_operation navigates to Screen::Keys + KeyScreen::SshPubkeyPopup (existing TUI popup) rather than escaping terminal"
  - "TestConnection screen gains test_conn_user/test_conn_host/test_conn_focused fields in SshState for TUI-native text input"
  - "Deprecated interactive functions (change_user_pin, generate_key_on_card, import_key_to_card, etc.) fully removed — no longer #[allow(dead_code)] stubs"
  - "export_ssh_public_key removed; export_ssh_key_to_file updated to call get_ssh_public_key_text directly"
  - "execute_key_operation ExportSSH routes to get_ssh_public_key_text + SshPubkeyPopup, replacing old export_ssh_public_key call"
requirements-completed: [NO-ESCAPE-01, IN-TUI-FEEDBACK-01]
duration: 35min
completed: "2026-03-25"
---

# Phase 04 Plan 04: Terminal Escape Audit and Cleanup Summary

**Complete elimination of Stdio::inherit and LeaveAlternateScreen from all operation functions — zero terminal escapes, TUI-native SSH test connection input, and removal of all deprecated interactive functions.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-03-25T18:53:00Z
- **Completed:** 2026-03-25T19:28:43Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- `set_touch_policy` converted from `Result<Child>` (Stdio::inherit) to `Result<String>` (Stdio::piped) — ykman runs non-interactively, no terminal escape
- `test_ssh_connection` converted to use Stdio::piped + BatchMode=yes + ConnectTimeout=10 — never hangs, never escapes terminal
- SSH ExportKey and TestConnection in `execute_ssh_operation` no longer drop out of TUI — ExportKey navigates to existing SshPubkeyPopup; TestConnection collects username/hostname via in-TUI text fields
- All deprecated interactive functions removed: `change_user_pin`, `change_admin_pin`, `set_reset_code`, `unblock_user_pin`, `execute_gpg_card_edit` from pin_operations.rs; `import_key_to_card`, `generate_key_on_card`, `reset_key_slot`, `export_ssh_public_key` from key_operations.rs
- Full audit confirmed: zero `Stdio::inherit` in `src/yubikey/`, `LeaveAlternateScreen` appears only in `App::run()` startup/shutdown (not in any operation function)
- All 57 unit tests pass; clippy clean

## Task Commits

1. **Task 1: Fix remaining escape sites (SSH, touch policy, view status)** - `d284c49` (feat)
2. **Task 2: Full audit — remove deprecated functions and verify zero escapes** - `2add36e` (feat)

## Files Created/Modified

- `src/yubikey/touch_policy.rs` — `set_touch_policy` now returns `Result<String>` with `Stdio::null/piped`, not `Result<Child>` with `Stdio::inherit`
- `src/yubikey/ssh_operations.rs` — `test_ssh_connection` uses piped IO + BatchMode/ConnectTimeout; `add_to_remote_authorized_keys` stdout/stderr now piped; `export_ssh_key_to_file` uses `get_ssh_public_key_text`
- `src/yubikey/pin_operations.rs` — deprecated interactive PIN functions and `execute_gpg_card_edit` removed entirely
- `src/yubikey/key_operations.rs` — `import_key_to_card`, `generate_key_on_card`, `reset_key_slot`, `export_ssh_public_key` removed entirely
- `src/ui/ssh.rs` — `SshState` gains `test_conn_user`, `test_conn_host`, `test_conn_focused` fields; `render_test_connection` shows two labeled text input boxes with Tab-to-switch focus
- `src/app.rs` — `execute_ssh_operation` ExportKey routes to `SshPubkeyPopup`, TestConnection uses TUI fields; `execute_key_operation` ExportSSH uses `get_ssh_public_key_text` + SshPubkeyPopup; `execute_touch_policy_set` uses new `Result<String>` API; TestConnection key handler handles Char/Backspace/Tab input

## Decisions Made

- `set_touch_policy` returns `Result<String>` (not `Result<Child>`) — this is a cleaner API boundary: callers get the outcome immediately without managing child lifetime; the `--force` flag means ykman doesn't need interactive PIN input for this operation
- SSH TestConnection uses TUI text input fields rather than a separate `TestConnectionInput` screen variant — fewer screen states, same result; Tab cycles between username/hostname
- SSH ExportKey in the SSH wizard (`execute_ssh_operation`) navigates to `Screen::Keys + KeyScreen::SshPubkeyPopup` (reuses existing popup), not a new SSH-specific popup
- `execute_key_operation` ExportSSH also routes to `SshPubkeyPopup` — both SSH wizard and Keys screen use the same popup path for key display

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

One minor compile error: used `ui::Screen::KeyManagement` (doesn't exist) instead of `Screen::Keys`. Fixed immediately before commit.

## Known Stubs

None — all paths are fully wired. SSH export shows real key data from `get_ssh_public_key_text()`. SSH test connection runs real `ssh` command with user-entered params.

## Next Phase Readiness

Phase 04 is complete. The NO-ESCAPE-01 requirement is fully satisfied:
- Zero `Stdio::inherit` in `src/yubikey/`
- `LeaveAlternateScreen` and `disable_raw_mode` appear only in `App::run()` startup/shutdown
- All operation functions use piped IO and return structured results for TUI display
- 57 unit tests pass; clippy clean

---
*Phase: 04-programmatic-subprocess-control*
*Completed: 2026-03-25*
