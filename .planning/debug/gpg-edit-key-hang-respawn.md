---
status: awaiting_human_verify
trigger: "gpg --edit-key process hangs during key import; TUI freezes; after kill -9, YubiKey requests touch, user touches it, then a NEW gpg process spawns and keeps running."
created: 2026-03-25T01:00:00Z
updated: 2026-03-25T02:00:00Z
---

## Current Focus

hypothesis: CONFIRMED — Root cause of BUG 1 found from debug log files at /tmp/yubitui-keytocard-*.log.
test: Read actual log output from the test run. /tmp/yubitui-keytocard-A123FF7475998643-slot1.log had 1.2M lines of infinite loop. /tmp/yubitui-keytocard-A123FF7475998643-slot2.log showed the same pattern.
expecting: Import completes with touch policy enabled. gpg process exits cleanly after save.
next_action: human verification — re-run key import with touch policy enabled

## Symptoms

expected: gpg --edit-key completes key-to-card transfer, TUI shows success, process exits cleanly
actual:
1. User enters admin PIN
2. gpg process (PID 3425) spawns and hangs — TUI is frozen
3. User kills PID 3425
4. YubiKey starts flashing (requesting touch)
5. User touches YubiKey, it stops flashing
6. Seconds later YubiKey starts flashing AGAIN
7. NEW gpg process (PID 3605) appears and keeps running indefinitely
errors: no error shown — TUI just hangs
reproduction: attempt key import via the TUI (key to card operation)
started: after the previous fix (removed save\n from upfront writes, now written in SC_OP_SUCCESS handler)

## Eliminated

- hypothesis: SC_OP_SUCCESS event missed or different token emitted
  evidence: Log shows SC_OP_SUCCESS was NEVER reached in either slot. The session never got past the keyedit.prompt loop to the point where a card operation even started.
  timestamp: 2026-03-25T02:00:00Z

- hypothesis: save command causes a confirmation prompt or needs a second quit
  evidence: save was never sent. The session got stuck in infinite keyedit.prompt loop before any card operation.
  timestamp: 2026-03-25T02:00:00Z

- hypothesis: Retry loop in app.rs causing re-spawn
  evidence: execute_key_import is called once on Submit; during KeyImportRunning all keys are ignored; poll_import_task uses try_recv (non-blocking). No retry in app.rs.
  timestamp: 2026-03-25T01:05:00Z

- hypothesis: Stderr pipe buffer deadlock causing hang
  evidence: stderr reader thread continuously drains the pipe into an unbounded mpsc channel; stdout is Stdio::null() (no pipe). No pipe buffer deadlock possible.
  timestamp: 2026-03-25T01:05:00Z

- hypothesis: gpg emits additional prompts after SC_OP_SUCCESS that are missed
  evidence: The hang occurs BEFORE SC_OP_SUCCESS (gpg waiting for touch). The break-after-SC_OP_SUCCESS path is not reached during the hang.
  timestamp: 2026-03-25T01:05:00Z

## Evidence

- timestamp: 2026-03-25T02:00:00Z
  checked: /tmp/yubitui-keytocard-A123FF7475998643-slot1.log (1,231,364 lines) and slot2.log (2,561 lines)
  found: |
    Both logs show the same pattern: gpg issues GET_LINE keyedit.prompt (the main edit-key command prompt),
    our UNEXPECTED GET_LINE handler sends an empty line, gpg echoes GOT_IT and re-issues keyedit.prompt.
    This is an infinite loop — 1.2M lines in slot 1.
    Slot 1 sequence: KEY_CONSIDERED → GET_LINE keyedit.prompt (consumed pre-buffered "keytocard") →
    GET_BOOL keyedit.keytocard.use_primary (we sent "y") → GET_LINE keyedit.prompt (nothing buffered) → LOOP.
    Slot 2 sequence: KEY_CONSIDERED → GET_LINE keyedit.prompt (consumed pre-buffered "key 1") →
    GET_LINE keyedit.prompt (consumed pre-buffered "keytocard") → CARDCTRL 3 (card removed) →
    GET_LINE cardedit.genkeys.storekeytype → GET_LINE keyedit.prompt → LOOP.
    SC_OP_SUCCESS was NEVER seen. PIN was NEVER requested. No card operation ever started.
  implication: |
    The pre-buffering approach is fundamentally broken. gpg processes stdin commands in response to
    GET_LINE keyedit.prompt prompts, but our pre-buffered writes happen before the event loop starts.
    The commands are queued in the pipe and consumed by the first N prompts, but there's no guarantee
    the command count matches the prompt count. After pre-buffered commands run out, keyedit.prompt
    hits the UNEXPECTED GET_LINE handler which sends empty → infinite loop.
    FIX NEEDED: Handle GET_LINE keyedit.prompt reactively using a state machine:
    state 0 (need to select key): send "key N" (if subkey_idx > 0, else skip), advance state
    state 1 (need to send keytocard): send "keytocard", advance state
    state 2 (keytocard issued, waiting for SC_OP_SUCCESS): if we get keyedit.prompt again after
      SC_OP_SUCCESS + save, we send "quit" to exit.
    Remove all pre-buffering of commands from before the event loop.

- timestamp: 2026-03-25T01:05:00Z
  checked: event_loop in app.rs (lines 88-95)
  found: event_loop calls terminal.draw, poll_import_task (try_recv, non-blocking), handle_events (100ms poll). TUI renders continuously. "Frozen" means KeyImportRunning screen ignores all input — there is no cancel, no status update beyond "Importing key to card...", no touch indicator.
  implication: TUI is not technically frozen but is unresponsive to user input by design. There is no way to know when/if touch is needed.

- timestamp: 2026-03-25T01:05:00Z
  checked: run_keytocard_session rx loop — what happens while gpg waits for touch
  found: The for-line-in-rx loop blocks synchronously. gpg emits GET_LINE keytocard.where (Rust sends slot), then GET_HIDDEN for PIN (Rust sends PIN), then sends keytocard to scdaemon. scdaemon contacts YubiKey. YubiKey with touch policy enabled starts flashing. gpg emits NOTHING while waiting for scdaemon's response. The rx loop blocks indefinitely — no timeout, no output until touch happens or times out. The hang is real: the background thread is stuck and the TUI shows "importing" with no touch prompt.
  implication: Primary UX bug. Touch hint message added to operation_status.

- timestamp: 2026-03-25T01:08:00Z
  checked: what happens after kill -9 on the gpg child
  found: child.wait() in Rust returns Ok(ExitStatus{signal}) — on Unix, a process killed by signal returns a non-success ExitStatus but wait() itself succeeds (does not return Err). The variable `success` is false. run_keytocard_session previously returned Ok(false). import_key_programmatic marked the slot as failed and CONTINUED the for loop to the next slot, spawning a new gpg process.
  implication: Root cause of the respawn. Fixed by detecting signal-kill in exit_status and returning Err instead of Ok(false).

- timestamp: 2026-03-25T01:08:00Z
  checked: scdaemon behavior when gpg is killed mid-operation
  found: scdaemon is a separate long-running daemon. When gpg dies, scdaemon loses its client but keeps its PC/SC connection. The pending touch-wait continues. When the user touches, scdaemon completes the card write (even without gpg). This explains why the YubiKey flashes AFTER kill. Then the new gpg process (for next slot) contacts scdaemon → second flash.
  implication: scdaemon behavior is correct. Our fix prevents the second gpg from spawning.

- timestamp: 2026-03-25T01:09:00Z
  checked: gpg status-fd tokens for touch-wait
  found: gpg does NOT emit a specific status token for "waiting for touch". The touch wait is entirely inside scdaemon. No TOUCH_REQUIRED or similar token exists in the gpg status-fd protocol.
  implication: Cannot detect touch-wait from status-fd. UX fix: update operation_status message to say "touch YubiKey if it is flashing" from the start of the import.

- timestamp: 2026-03-25T01:12:00Z
  checked: compile and test after fix
  found: cargo build succeeds, cargo test: 85 passed 0 failed.
  implication: Fix is syntactically correct and does not regress existing tests.

## Resolution

root_cause:
  BUG 1 (hang): The run_keytocard_session function pre-buffered "key N" and "keytocard" to stdin
  before starting the event loop. gpg --edit-key drives the interaction via GET_LINE keyedit.prompt
  (one command per prompt). The pre-buffered writes were consumed by the first N keyedit.prompt
  occurrences in whatever order gpg issued them. After the pre-buffered commands were exhausted,
  subsequent keyedit.prompt prompts hit the UNEXPECTED GET_LINE handler which sent empty lines.
  gpg responded to each empty line with GOT_IT and re-issued keyedit.prompt — creating an infinite
  loop. Confirmed by debug log: slot1.log has 1,231,364 lines of GET_LINE keyedit.prompt / empty /
  GOT_IT repeating. SC_OP_SUCCESS was never reached; no card operation ever started.
  BUG 2 (respawn): kill -9 on gpg causes child.wait() to return Ok(ExitStatus{killed by signal}).
  This is not a Rust Err — run_keytocard_session returned Ok(false). import_key_programmatic's
  for loop continued to the next slot and spawned a new gpg process for it.

fix:
  BUG 1: Replaced pre-buffering with a state machine in the GET_LINE keyedit.prompt handler.
    State SelectKey  → send "key N" (for subkey_idx > 0)
    State SendKeytocard → send "keytocard"
    State WaitForResult → send "quit" if keyedit.prompt arrives (keytocard was rejected silently)
    State SendSave   → SC_OP_SUCCESS seen, next keyedit.prompt sends "save"
    State Done       → send "quit" if gpg returns to main prompt after save
    SC_OP_SUCCESS transitions to SendSave (does not write save immediately — waits for keyedit.prompt).
    Unknown GET_LINE prompts now send "quit" instead of empty, to exit gracefully.
  BUG 2: After child.wait(), check exit_status.signal() (via ExitStatusExt on Unix). If the process
    was killed by a signal, bail! with an informative error message.

verification: cargo build clean, 85 tests pass. Awaiting human confirmation.
files_changed: [src/yubikey/key_operations.rs, src/app.rs]
