---
phase: 02-ux-menus-wizards-fixes
plan: "03"
subsystem: key-attributes
tags: [key-attributes, ssh-pubkey-popup, ykman, tui-popup, openpgp]
dependency_graph:
  requires: [02-01, 02-02]
  provides: [key-attribute-display, ssh-pubkey-popup]
  affects: [src/ui/keys.rs, src/yubikey/key_operations.rs, src/app.rs]
tech_stack:
  added: []
  patterns: [popup overlay on main screen background, ykman openpgp info parsing, gpg --export-ssh-key non-interactive]
key_files:
  created: []
  modified:
    - src/yubikey/key_operations.rs
    - src/ui/keys.rs
    - src/app.rs
decisions:
  - "show_context_menu and menu_selected_index fields kept with #[allow(dead_code)] — reserved for Plan 02-04 context menu integration"
  - "Single-match clippy lint fixed by converting match to if expression for KeyAttributes | SshPubkeyPopup arm"
  - "get_ssh_public_key_text() uses gpg --card-status then gpg --export-ssh-key with -- flag separator for security"
metrics:
  duration_minutes: 4
  completed_date: "2026-03-24T19:40:00Z"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 3
---

# Phase 2 Plan 03: Key Attributes and SSH Pubkey Popup Summary

**One-liner:** KeyAttributes display per slot via ykman openpgp info parsing, plus in-TUI SSH public key viewer popup with copy instructions — user presses A or S on Key Management screen.

## What Was Built

### Task 1: get_key_attributes(), get_ssh_public_key_text(), and UI extension

Added to `src/yubikey/key_operations.rs`:

- **`KeyAttributes` struct:** `signature`, `encryption`, `authentication` fields each holding `Option<SlotInfo>`
- **`SlotInfo` struct:** `algorithm` (e.g., "ed25519", "RSA2048") and `fingerprint` fields
- **`get_key_attributes()`:** Calls `ykman openpgp info` via `find_ykman()`, parses output with `parse_ykman_openpgp_info()` and `save_slot()` helpers
- **`get_ssh_public_key_text()`:** Calls `gpg --card-status` to find auth key fingerprint, then `gpg --export-ssh-key -- <fp>` to return key as string (no terminal output, safe for TUI)

Extended `src/ui/keys.rs`:

- **`KeyScreen::KeyAttributes`** — new variant for read-only algorithm display
- **`KeyScreen::SshPubkeyPopup`** — new variant for in-TUI SSH key viewer
- **`KeyState` extended** with `key_attributes: Option<KeyAttributes>`, `ssh_pubkey: Option<String>`, `show_context_menu: bool`, `menu_selected_index: usize`
- **`render_key_attributes()`** — displays algorithm + fingerprint per slot in green; empty slots in DarkGray; if no ykman data shows Yellow warning
- **`render_ssh_pubkey_popup()`** — renders main screen as background, overlays popup with key text + copy instructions for authorized_keys, GitHub, GitLab
- **`render_main()` updated** — added `[A] View key attributes` (Blue) and `[S] Show SSH public key` (White) items; constraint increased from 12 to 14

### Task 2: Event wiring in app.rs

Updated `src/app.rs` Keys handling:

- **`KeyCode::Char('a')`** in `KeyScreen::Main`: transitions to `KeyScreen::KeyAttributes`, calls `get_key_attributes()`, caches result or sets message on error
- **`KeyCode::Char('s')`** in `KeyScreen::Main`: transitions to `KeyScreen::SshPubkeyPopup`, calls `get_ssh_public_key_text()`, caches result or sets message on error
- **Explicit `KeyScreen::KeyAttributes | KeyScreen::SshPubkeyPopup` arm** before the existing catch-all: handles only `Esc` (returns to Main, clears message); Enter is intentionally a no-op on these read-only screens

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Clippy compliance] show_context_menu and menu_selected_index trigger dead_code under -D warnings**
- **Found during:** Task 2 clippy check
- **Issue:** Fields added per plan spec but not yet used in the UI; `-D warnings` flags them as errors
- **Fix:** Added `#[allow(dead_code)]` with comment noting they are reserved for Plan 02-04 context menu integration
- **Files modified:** `src/ui/keys.rs`
- **Commit:** 8f48b8c0

**2. [Rule 1 - Clippy compliance] Single-match pattern triggers clippy::single_match under -D warnings**
- **Found during:** Task 2 clippy check
- **Issue:** `KeyScreen::KeyAttributes | KeyScreen::SshPubkeyPopup => match key.code { KeyCode::Esc => ... _ => {} }` triggers `single_match`
- **Fix:** Converted to `if key.code == KeyCode::Esc { ... }` block
- **Files modified:** `src/app.rs`
- **Commit:** 8f48b8c0

## Known Stubs

None. All features are fully implemented:
- `get_key_attributes()` calls ykman — returns error message if ykman not installed
- `get_ssh_public_key_text()` calls gpg — returns error message if no auth key on card
- Both A and S screens dismiss cleanly with Esc

## Verification Results

- `cargo check`: PASSED
- `cargo clippy -- -D warnings`: PASSED
- `cargo test`: PASSED (0 regressions)
- `grep -n "KeyAttributes\|SshPubkeyPopup" src/ui/keys.rs`: 5 matches (variants, field, dispatch, render functions)
- `grep -n "get_key_attributes\|get_ssh_public_key_text" src/yubikey/key_operations.rs`: 2 matches (both functions)
- `grep -n "KeyAttributes\|SshPubkeyPopup" src/app.rs`: 3 matches (event handlers and arm)

## Self-Check: PASSED
