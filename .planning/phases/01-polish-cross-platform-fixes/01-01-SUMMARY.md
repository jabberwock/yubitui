---
phase: 01-polish-cross-platform-fixes
plan: "01"
subsystem: ui/keys
tags: [ui, key-picker, ratatui, interactive]
dependency_graph:
  requires: []
  provides: [KEY-PICKER]
  affects: [src/ui/keys.rs, src/app.rs]
tech_stack:
  added: []
  patterns: [ratatui List/ListItem with per-item styles, stateful selection index]
key_files:
  created: []
  modified:
    - src/ui/keys.rs
    - src/app.rs
decisions:
  - "Use a separate layout with 4 chunks for ImportKey screen instead of the shared render_operation_screen helper, to allow ratatui List widget for key listing"
  - "Bounds-clamp selected_key_index at import time rather than preventing out-of-bounds index at UI level"
metrics:
  duration: "2 minutes"
  completed: "2026-03-24T18:20:00Z"
  tasks_completed: 2
  files_modified: 2
---

# Phase 1 Plan 01: Interactive Key Picker Summary

**One-liner:** Interactive GPG key picker with arrow-key navigation and yellow-bold highlight using ratatui List widget, replacing hardcoded first-key import.

## What Was Built

The ImportKey screen in yubitui previously always imported `available_keys[0]` regardless of how many keys existed in the user's GPG keyring. Users with multiple GPG keys had no way to choose which one to import.

This plan replaces the hardcoded behavior with a full interactive key picker:

1. `KeyState` gains a `selected_key_index: usize` field (default 0)
2. Entering the ImportKey screen resets the selection to 0 and loads available keys
3. Up/Down arrow keys navigate the list (with bounds enforcement)
4. The import operation uses the selected index (with a bounds-safety clamp)
5. The ImportKey screen renders a ratatui `List` widget with per-item styles:
   - Selected item: `"> [N] key_id"` in yellow bold
   - Other items: `"  [N] key_id"` in white
   - Empty state: red message with `gpg --import <file>` hint
   - Hint bar: `"Use Up/Down to select, Enter to import, Esc to cancel"` in dark gray

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Add selected_key_index to KeyState and wire up arrow-key navigation | 841a1a9c | src/ui/keys.rs, src/app.rs |
| 2 | Render key list with visual highlight in ImportKey screen | 236af64b | src/ui/keys.rs, src/app.rs |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed clippy lint: push_str with single char**
- **Found during:** Task 2 (cargo clippy -- -D warnings)
- **Issue:** `hint_text.push_str("\n")` triggers `clippy::single_char_add_str`
- **Fix:** Changed to `hint_text.push('\n')`
- **Files modified:** src/ui/keys.rs
- **Commit:** 236af64b

**2. [Rule 3 - Blocking] Applied cargo fmt to fix pre-existing formatting issues in modified files**
- **Found during:** Task 2 (cargo fmt -- --check)
- **Issue:** Pre-existing formatting issues in src/app.rs and src/ui/keys.rs (trailing whitespace, line length) caused `cargo fmt -- --check` to fail
- **Fix:** Ran `cargo fmt` to auto-format all modified files; also reformatted other files touched by rustfmt (not changed by this plan)
- **Files modified:** src/ui/keys.rs, src/app.rs (plus other files formatted by rustfmt as side effect)
- **Commit:** 236af64b

## Known Stubs

None — the feature is fully wired: keys load from `list_gpg_keys()`, selection state is maintained, and `import_key_to_card()` is called with the selected key ID.

## Verification Results

- `cargo check`: PASSED
- `cargo clippy -- -D warnings`: PASSED
- `cargo fmt -- --check`: PASSED
- `grep "available_keys[0]" src/app.rs`: No matches (hardcoded index removed)
- `grep "selected_key_index" src/ui/keys.rs src/app.rs`: Usage confirmed in both files
