---
phase: 13-ui-polish
plan: "04"
subsystem: tui
tags: [ui-polish, datatable, markdown, otp, help, glossary]
dependency_graph:
  requires: []
  provides: [polished-otp-screen, polished-help-screen, polished-glossary-screen]
  affects: [src/tui/otp.rs, src/tui/help.rs, src/tui/glossary.rs]
tech_stack:
  added: []
  patterns: [DataTable-for-tabular-data, Markdown-for-long-text, Button-for-actions]
key_files:
  created: []
  modified:
    - src/tui/otp.rs
    - src/tui/help.rs
    - src/tui/glossary.rs
    - src/tui/snapshots/yubitui__tui__otp__tests__otp_default_state.snap
    - src/tui/snapshots/yubitui__tui__otp__tests__otp_no_yubikey.snap
    - src/tui/snapshots/yubitui__tui__help__tests__help_screen.snap
    - src/tui/snapshots/yubitui__tui__glossary__tests__glossary_screen.snap
decisions:
  - "Used DataTable::new(columns) + add_row() API (plan showed DataTable::new(cols, rows) which doesn't exist — adapted to actual API)"
  - "Used ColumnDef::new(label).with_width(n) API (plan showed ColumnDef::new(label, n) — adapted)"
  - "Markdown::new(&str) confirmed correct signature"
metrics:
  duration_seconds: 123
  completed_date: "2026-03-29"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 7
---

# Phase 13 Plan 04: OTP / Help / Glossary Polish Summary

OtpScreen upgraded to DataTable with [OK]/[EMPTY] slot badges and a Refresh button; HelpScreen and GlossaryScreen replaced ~25 Label calls each with a single Markdown widget rendering formatted headings and tables.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Polish OtpScreen — DataTable for slots, Buttons for actions | 655d5f33 | src/tui/otp.rs, 2 snapshots |
| 2 | Polish HelpScreen and GlossaryScreen — Markdown widget | 655d5f33 | src/tui/help.rs, src/tui/glossary.rs, 2 snapshots |

## What Was Built

### OtpScreen
- Slot display replaced with a 3-column `DataTable` (Status / Slot / Configuration)
- Slot 1 and Slot 2 rows show `[OK]` or `[EMPTY]` status badges with touch-policy detail
- Hardware write-only note kept as `Label` below the table (important user message)
- `Button::new("Refresh (R)")` added in both the key-present and no-key branches
- Layout: Header → heading Label → DataTable → note Labels → spacer → Refresh Button → Footer

### HelpScreen
- ~25 individual `Label::new()` calls replaced with a single `Markdown::new(HELP_MARKDOWN)` call
- Content includes H1/H2 headings and Markdown tables for all keybinding groups
- `compose()` reduced to 3 lines: Header → Markdown → Footer
- `key_bindings()` and `on_action()` unchanged

### GlossaryScreen
- ~25 individual `Label::new()` calls replaced with a single `Markdown::new(GLOSSARY_MARKDOWN)` call
- Content includes H1 title and H2 section per protocol with body text
- `compose()` reduced to 3 lines: Header → Markdown → Footer
- `key_bindings()` and `on_action()` unchanged

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] DataTable API mismatch**
- **Found during:** Task 1
- **Issue:** Plan showed `DataTable::new(columns, rows)` and `ColumnDef::new("label", width)` — neither constructor exists in textual-rs 0.3.11
- **Fix:** Used actual API: `DataTable::new(columns)` + `.add_row(vec![...])`, and `ColumnDef::new("label").with_width(n)`
- **Files modified:** src/tui/otp.rs
- **Commit:** 655d5f33

**2. [Rule 1 - Bug] Snapshot tests needed update**
- **Found during:** Task 1 & 2
- **Issue:** Insta snapshot tests failed because rendering changed (expected behavior)
- **Fix:** Ran `INSTA_UPDATE=always cargo test` to accept the new snapshots
- **Files modified:** 4 snapshot files
- **Commit:** 655d5f33

## Verification

```
cargo check: Finished dev profile — 0 errors
cargo test (4 tests): otp_default_state OK, otp_no_yubikey OK, help_screen OK, glossary_screen OK
```

## Known Stubs

None — all widget data is wired to live model state (`OtpState`) or static content constants. No placeholder text or empty data sources.

## Self-Check: PASSED

Files created/modified:
- FOUND: src/tui/otp.rs
- FOUND: src/tui/help.rs
- FOUND: src/tui/glossary.rs
- FOUND: src/tui/snapshots/yubitui__tui__otp__tests__otp_default_state.snap
- FOUND: src/tui/snapshots/yubitui__tui__otp__tests__otp_no_yubikey.snap
- FOUND: src/tui/snapshots/yubitui__tui__help__tests__help_screen.snap
- FOUND: src/tui/snapshots/yubitui__tui__glossary__tests__glossary_screen.snap

Commits:
- FOUND: 655d5f33
