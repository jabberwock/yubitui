# Phase 4: Programmatic Subprocess Control - Context

**Gathered:** 2026-03-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Replace all `Stdio::inherit()` subprocess calls that escape the TUI. The two offenders are `execute_pin_operation()` and `execute_key_operation()` in `src/app.rs`, both of which call `LeaveAlternateScreen` + `disable_raw_mode` before spawning gpg interactively, then restore the TUI after. Every gpg and ykman operation must stay inside the TUI — no terminal handoff, no leaving the alternate screen.

Out of scope: replacing gpg/ykman with native Rust PC/SC crates (Phase 5).

</domain>

<decisions>
## Implementation Decisions

### Mechanism for non-interactive gpg
- **D-01:** Use `--pinentry-mode loopback` + `--status-fd 1` on all gpg card operations. gpg sends `[GNUPG:]` status tokens on the status fd; the app reads them to drive UI state (know when a PIN is needed, when the operation succeeded, when it failed with what error).
- **D-02:** Pin/passphrase collection via `--command-fd 0` (stdin): when gpg emits `NEED_PASSPHRASE` or `GET_HIDDEN`, the app reads the next TUI-collected PIN and writes it to gpg's stdin.
- **D-03:** Remove `LeaveAlternateScreen` / `disable_raw_mode` / `EnterAlternateScreen` / `enable_raw_mode` from both `execute_pin_operation()` and `execute_key_operation()`. These calls exist solely to support `Stdio::inherit()` — they become unnecessary.

### PIN input widget
- **D-04:** Build a TUI PIN input widget with masked display using dots (●●●●●●). The widget supports: character append (printable keys), backspace, Enter to submit, Esc to cancel.
- **D-05:** PIN change operations (change user PIN, change admin PIN, set reset code, unblock) show all required fields on a single screen — current PIN → new PIN → confirm new PIN — with the active field highlighted. Tab or Enter moves focus between fields. Submit is only enabled when all fields are non-empty.
- **D-06:** The PIN input widget lives in `src/ui/widgets/` alongside the existing popup widget (`src/ui/widgets/popup.rs`). It is reusable across PIN operations and key operations.

### Progress and status feedback
- **D-07:** While an operation runs, show a spinner + a current step status line (e.g. "Verifying current PIN...", "Setting new PIN...", "Generating key..."). Use the existing popup widget for the container.
- **D-08:** Translate gpg `[GNUPG:]` status tokens to human-readable messages. Examples: `ERROR change_passwd 67108949` → "Incorrect PIN (N attempts remaining)"; `KEY_CREATED P <fp>` → "Key generated successfully"; `CARDCTRL 3` → "Card removed — reinsert and retry". Do not show raw gpg output to the user.

### Key generation wizard
- **D-09:** Full wizard in TUI, replacing the escaped `gpg --card-edit admin generate quit` flow. Wizard screens: (1) algorithm selection (ed25519 / cv25519 pair recommended, rsa2048, rsa4096), (2) expiry (none / 1yr / 2yr / custom date), (3) identity (name + email fields), (4) backup copy prompt (yes/no, with file path input if yes), (5) confirm + Admin PIN entry, (6) result screen.
- **D-10:** Backup copy: if user selects yes, gpg creates an off-card backup during generation. Show a file path field (default: `~/yubikey-backup-<date>.gpg`). Backup is optional and defaults to no.
- **D-11:** gpg batch key generation (`gpg --batch --gen-key`) with a generated parameter file is the recommended non-interactive mechanism. The parameter file sets `Key-Type`, `Subkey-Type`, `Name-Real`, `Name-Email`, `Expire-Date`, `%no-protection` (since the card PIN is the protection). Research the exact batch format.

### Import key flow
- **D-12:** `gpg --edit-key -- <keyid>` with `--pinentry-mode loopback --status-fd 1 --command-fd 0`. The `keytocard` command sequence is automated: select each subkey by capability (S → sig slot, E → enc slot, A → aut slot) using gpg's `key N` commands, then `keytocard`, then `save`.
- **D-13:** Auto-map subkeys by capability flags — no subkey picker shown to the user. The operation maps: signing-capable subkey → SIG slot, encryption-capable → ENC slot, authentication-capable → AUT slot. If a capability has no matching subkey, that slot is skipped.
- **D-14:** After import, show a result popup listing which slots were filled (e.g. "SIG ✓  ENC ✓  AUT —") then return to the key menu. Same popup widget as other operations.

### Audit of remaining inherit() calls
- **D-15:** After fixing PIN and key operations, audit ALL remaining `Stdio::inherit()` and `LeaveAlternateScreen` calls in the codebase. Any found that aren't covered by the above must be converted or documented as intentional.

### Claude's Discretion
- Exact gpg status token → human message mapping table (implement the common ones; fall back to "Operation failed — check gpg is installed and card is inserted" for unmapped codes)
- Spinner animation style (braille dots, pipe chars, etc.) — standard terminal spinner is fine
- Exact ratatui widget structure for the PIN input field and the keygen wizard screens

</decisions>

<specifics>
## Specific Ideas

- The user's core frustration: after invoking a PIN operation, the TUI disappears and the user is dumped at a raw gpg prompt with no indication of what to type or what will happen next. The fix isn't just technical — the UX must actively tell the user what's happening at each step.
- Phase 5 (native PC/SC) is explicitly deferred. Phase 4 stays with gpg/ykman CLIs, just non-interactively.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

No external specs — requirements are fully captured in decisions above.

### Key source files (read before planning)
- `src/app.rs` §`execute_pin_operation` (line 635) — current LeaveAlternateScreen + gpg escape pattern to replace
- `src/app.rs` §`execute_key_operation` (line 677) — same pattern for key operations
- `src/yubikey/pin_operations.rs` — `execute_gpg_card_edit` is the primary target; `find_ykman` is fine as-is
- `src/yubikey/key_operations.rs` — `import_key_to_card`, `generate_key_on_card`, `reset_key_slot` are targets
- `src/ui/widgets/popup.rs` — existing popup widget to reuse/extend for PIN input and progress display
- `src/ui/pin.rs` — existing PIN screen states (`PinScreen` enum, `PinState` struct) — new fields needed for input values
- `src/ui/keys.rs` — key screen states — wizard screens need adding

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/ui/widgets/popup.rs` — `render_popup`, `render_confirm_dialog`, `render_context_menu`: reuse for PIN input overlays and progress popups
- `PinState` in `src/ui/pin.rs` — already has `screen: PinScreen` and `message: Option<String>`; add PIN input buffer fields here
- `find_ykman()` in `src/yubikey/pin_operations.rs` — already handles PATH + Windows fallback; keep unchanged

### Established Patterns
- Operation result as `Option<String>` stored on state structs, rendered as popup — keep this pattern for results
- `[GNUPG:]` status-fd parsing follows the same fixture-based testable pattern established in Phase 3 (parser functions separated from command invocation, tested with string fixtures)
- `-- <arg>` flag separator already used in `gpg --export-ssh-key -- <fp>` — apply same pattern everywhere

### Integration Points
- `execute_pin_operation()` and `execute_key_operation()` in `app.rs` are the entry points — these are where `LeaveAlternateScreen` is called and where the new non-interactive subprocess logic connects
- The TUI event loop in `app.rs` handles key events; adding PIN input field editing means routing keystrokes to the active input widget when a PIN screen is active
- `crossterm::terminal` calls (`disable_raw_mode`, `LeaveAlternateScreen`) used for the escape — removing them restores the TUI-stays-alive invariant

</code_context>

<deferred>
## Deferred Ideas

- Replace gpg/ykman CLI with native Rust PC/SC + openpgp-card crates — Phase 5
- Subkey picker UI — deferred (auto-map by capability is sufficient for Phase 4)

</deferred>

---

*Phase: 04-programmatic-subprocess-control*
*Context gathered: 2026-03-25*
