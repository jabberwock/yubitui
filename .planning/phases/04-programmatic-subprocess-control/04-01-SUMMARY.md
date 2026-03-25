---
phase: 04-programmatic-subprocess-control
plan: "01"
subsystem: yubikey-parser, ui-widgets
tags: [gpg, status-fd, pin-input, progress, tui, widgets]
dependency_graph:
  requires: []
  provides: [gpg_status_parser, pin_input_widget, progress_popup_widget]
  affects: [04-02, 04-03, 04-04]
tech_stack:
  added: []
  patterns: [fixture-based-unit-tests, dead_code-for-future-use, //!-module-docs]
key_files:
  created:
    - src/yubikey/gpg_status.rs
    - src/ui/widgets/pin_input.rs
    - src/ui/widgets/progress.rs
  modified:
    - src/yubikey/mod.rs
    - src/ui/widgets/mod.rs
    - src/diagnostics/pcscd.rs
decisions:
  - "#[allow(dead_code)] applied to pub items in gpg_status and pin_input/progress — these are consumed by Plans 02-04, not yet wired"
  - "centered_area helper duplicated in pin_input.rs and progress.rs (as in popup.rs) rather than extracting to shared util — avoids changing existing popup.rs"
  - "Module-level docs use //! (inner doc comment) per clippy needless_doc_comment rule"
metrics:
  duration_seconds: 211
  completed_date: "2026-03-25"
  tasks_completed: 2
  files_created: 3
  files_modified: 3
---

# Phase 04 Plan 01: Foundational Modules Summary

Three new source files implementing GPG status-fd parsing, TUI PIN input widget, and spinner progress popup — the building blocks for non-interactive gpg subprocess control.

## What Was Built

**Task 1: GPG status-fd parser (`src/yubikey/gpg_status.rs`)**

Parses `[GNUPG:] TOKEN [args...]` lines from gpg's `--status-fd` output into a typed `GpgStatus` enum with 13 variants. Translates tokens to human-readable messages for TUI display. 21 unit tests covering all token variants and all message translations — no hardware required (fixture strings).

**Task 2: PIN input widget and progress popup (`src/ui/widgets/`)**

- `pin_input.rs`: `PinInputField`, `PinInputState`, `PinInputAction` — multi-field masked input form with Tab navigation, backspace, Enter submit, Esc cancel, error message display. Render function `render_pin_input` shows ● characters for each typed character, cursor block on active field, yellow highlight on active field.
- `progress.rs`: `render_progress_popup` — centered popup showing spinner (`| / - \`) + status string. Caller increments `tick` each render frame to animate.
- Both registered in `src/ui/widgets/mod.rs`.

## Verification

- `cargo test gpg_status` — 21 tests, all pass
- `cargo test` — 57 tests total (36 pre-existing + 21 new), all pass
- `cargo clippy -- -D warnings` — clean
- `cargo build` — succeeds

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Pre-existing pcscd.rs clippy errors blocked `cargo clippy -- -D warnings`**
- **Found during:** Task 1 verification
- **Issue:** Two `.args(&[...])` calls in `src/diagnostics/pcscd.rs` triggered `needless_borrows_for_generic_args` clippy error, causing `-D warnings` to fail for all code
- **Fix:** Changed `&["print", ...]` to `["print", ...]` and `&["-x", ...]` to `["-x", ...]` — trivial one-line fixes
- **Files modified:** `src/diagnostics/pcscd.rs`
- **Commit:** 5f3c1c2

**2. [Rule 2 - Missing] Module-level doc comment style**
- **Found during:** Task 1 verification
- **Issue:** Clippy flagged `/// ...` at file top as `needless_doc_comment` — module-level docs require `//!`
- **Fix:** Changed `///` to `//!` in `gpg_status.rs`
- **Files modified:** `src/yubikey/gpg_status.rs`
- **Commit:** 5f3c1c2

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1    | 5f3c1c2 | feat(04-01): GPG status-fd parser with 21 unit tests |
| 2    | 8007e98 | feat(04-01): PIN input widget and progress popup widget |

## Known Stubs

None — these are pure logic/widget modules with no data flow that could be stubbed.

## Self-Check: PASSED
