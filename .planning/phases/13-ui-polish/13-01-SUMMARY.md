---
phase: 13-ui-polish
plan: "01"
subsystem: tui
tags: [ui-polish, dashboard, diagnostics, button, datatable]
dependency_graph:
  requires: []
  provides: [polished-dashboard, polished-diagnostics]
  affects: [src/tui/dashboard.rs, src/tui/diagnostics.rs]
tech_stack:
  added: []
  patterns: [Button-nav, DataTable-status, bracket-badge-notation]
key_files:
  created: []
  modified:
    - src/tui/dashboard.rs
    - src/tui/diagnostics.rs
    - src/tui/snapshots/yubitui__tui__dashboard__tests__dashboard_default_populated.snap
    - src/tui/snapshots/yubitui__tui__dashboard__tests__dashboard_no_yubikey.snap
    - src/tui/snapshots/yubitui__tui__dashboard__tests__dashboard_context_menu_open.snap
    - src/tui/snapshots/yubitui__tui__diagnostics__tests__diagnostics_default.snap
decisions:
  - "DataTable::new(columns) API takes columns only; rows added via add_row(&mut self) on a local mut variable inside compose()"
  - "Snapshot tests updated immediately rather than deferring to plan 05 — insta accept used to accept new layouts"
metrics:
  duration: ~15 minutes
  completed: 2026-03-29T19:29:55Z
  tasks_completed: 2
  files_modified: 6
---

# Phase 13 Plan 01: Dashboard and Diagnostics UI Polish Summary

Dashboard navigation replaced with Button widgets and status badges normalized to [OK]/[BLOCKED]/[DANGER]/[SET]/[EMPTY]; Diagnostics compose() replaced with DataTable (Status/Component/Detail) plus a Run Diagnostics Button.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Polish DashboardScreen — Buttons for nav, consistent status badges | b70fd6e6 | src/tui/dashboard.rs + 3 snapshots |
| 2 | Polish DiagnosticsScreen — DataTable for status, consistent badges | 20627984 | src/tui/diagnostics.rs + 1 snapshot |

## What Was Built

### Dashboard (Task 1)

- All 9 navigation items converted from `Label::new("  [N] ...")` to `Button::new("[N] ...")` with visual border affordance
- PIN status badges changed from bare `LOW` to `[DANGER]`, `BLOCKED` to `[BLOCKED]`, `OK` to `[OK]`
- Key slot badges changed from `Set`/`Empty` to `[SET]`/`[EMPTY]`
- No-YubiKey state now includes a `Button::new("Refresh (R)")` beneath the explanation label
- Layout order: Header → device status Labels → spacer → nav Buttons → Footer

### Diagnostics (Task 2)

- Replaced 4 separate Label blocks with a `DataTable` (columns: Status 8ch, Component 25ch, Detail 40ch)
- 4 rows: PC/SC Daemon, GPG Agent, Scdaemon, SSH Agent — each with consistent `[OK]`/`[!!]`/`[  ]` badge
- Supplemental details (versions, socket paths, issues) shown as indented Labels after the table
- `Button::new("Run Diagnostics (R)")` added before Footer
- Layout order: Header → DataTable → detail Labels → spacer → action Button → Footer

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] DataTable API mismatch — plan showed `DataTable::new(columns, rows)` but actual API is `DataTable::new(columns)` with `add_row(&mut self)`**
- **Found during:** Task 2
- **Issue:** The plan's pseudocode passed rows as second argument to `DataTable::new()`, which does not match the 0.3.11 API
- **Fix:** Created local `mut table = DataTable::new(columns)` and called `table.add_row(...)` for each row before boxing
- **Files modified:** src/tui/diagnostics.rs
- **Commit:** 20627984

**2. [Rule 3 - Blocking] Snapshot tests failed after compose() output changed**
- **Found during:** Task 1 and Task 2
- **Issue:** insta snapshot tests compare exact terminal output; all 4 dashboard snapshots and 1 diagnostics snapshot needed updating
- **Fix:** Used `cargo insta accept` to accept new snapshots; all 4 tests now pass
- **Files modified:** 4 snapshot files
- **Commit:** b70fd6e6, 20627984

## Known Stubs

None — both screens render live data from the model layer. No hardcoded placeholder values or TODO markers were introduced.

## Self-Check: PASSED

- src/tui/dashboard.rs: FOUND
- src/tui/diagnostics.rs: FOUND
- Commit b70fd6e6: FOUND
- Commit 20627984: FOUND
