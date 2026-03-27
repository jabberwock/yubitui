---
phase: 08-textual-rs-migration
plan: 02
subsystem: tui-infrastructure
tags: [textual-rs, app-runner, theme, config, help-screen, widget-migration]

dependency_graph:
  requires:
    - phase: 08-01
      provides: "textual-rs git dep, ratatui 0.30, ClickRegion-free codebase"
  provides:
    - "textual-rs App::new().run() replacing crossterm event loop"
    - "src/tui/config.rs — theme config persistence (read/write ~/.config/yubitui/config.toml)"
    - "src/tui/theme.rs — load_theme_from_config() with tokyo-night default"
    - "src/tui/help.rs — HelpScreen as textual-rs Widget (first migrated screen)"
  affects: [08-03, 08-04, 08-05, 08-06, all-subsequent-screen-migrations]

tech-stack:
  added: []
  patterns:
    - "textual-rs Widget pattern: struct + impl Widget { widget_type_name, compose, key_bindings, on_action, render }"
    - "Screen App runner: thin pub fn run(mock: bool) -> Result<()> in app.rs calling App::new(factory).run()"
    - "Theme config: TOML at ~/.config/yubitui/config.toml, key = theme string"

key-files:
  created:
    - src/tui/config.rs
    - src/tui/theme.rs
  modified:
    - src/app.rs
    - src/main.rs
    - src/tui/mod.rs
    - src/tui/help.rs

key-decisions:
  - "app.rs is now a thin pub fn run() — old App struct and crossterm event loop fully deleted"
  - "HelpScreen uses compose() for content (Label widgets per line) rather than custom render() — simplest approach for display-only screen"
  - "Orphaned insta snapshot yubitui__tui__help__tests__help_screen.snap deleted — old test replaced by textual-rs TestApp::new pattern"
  - "Theme names verified against actual textual-rs builtin_themes(): tokyo-night, nord, gruvbox-dark, dracula, catppuccin-mocha (not the aliases from RESEARCH.md)"

patterns-established:
  - "Pattern A: Widget impl with compose() for container screens — return Header + content Widgets + Footer"
  - "Pattern B: key_bindings() static slice with show=true for footer-visible bindings"
  - "Pattern C: on_action() matches action strings and calls ctx.pop_screen_deferred() / ctx.push_screen_deferred()"

requirements-completed: [INFRA-03]

duration: ~20min
completed: 2026-03-27
---

# Phase 8 Plan 02: App Runner + Theme Infrastructure + Help Screen Migration Summary

**textual-rs App runner replaces crossterm event loop; theme/config persistence added; Help screen becomes first textual-rs Widget with Header/Footer and key bindings**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-03-27T13:10:00Z
- **Completed:** 2026-03-27T13:31:50Z
- **Tasks:** 2
- **Files modified:** 6 (2 created, 4 modified, 1 deleted)

## Accomplishments

- Old `App` struct (crossterm event loop, 950+ lines) deleted; replaced with 40-line `app::run()` calling `textual_rs::App::new(factory).run()`
- Theme infrastructure: `src/tui/config.rs` reads/writes `~/.config/yubitui/config.toml`; `src/tui/theme.rs` calls `theme_by_name()` with tokyo-night default
- `HelpScreen` is a fully functional textual-rs Widget with `compose()`, `key_bindings()`, `on_action()` — all 24 keybinding help lines ported as Label widgets
- All 109 tests pass (new `help_screen_renders` test via `TestApp::pilot().settle().await`)

## Task Commits

1. **Task 1: Create theme and config modules + rewrite app.rs as textual-rs runner** - `888abff` (feat)
2. **Task 2: Migrate Help screen to textual-rs Widget** - `f021c5f` (feat)

## Files Created/Modified

- `src/app.rs` — replaced old crossterm App struct with thin `pub fn run(mock: bool) -> Result<()>`
- `src/main.rs` — changed `App::new(args.mock)?.run()` to `app::run(args.mock)?`
- `src/tui/mod.rs` — added `pub mod config` and `pub mod theme`; removed `render_status_bar` (Footer replaces it)
- `src/tui/config.rs` (new) — `config_path()`, `read_theme_name()`, `save_theme_name()`
- `src/tui/theme.rs` (new) — `THEME_NAMES`, `DEFAULT_THEME`, `load_theme_from_config()`, `next_theme_name()`
- `src/tui/help.rs` — rewritten as `HelpScreen` implementing `textual_rs::Widget`

## Decisions Made

- `app.rs` is now a thin `pub fn run()` function. The `App` struct name was in conflict with `textual_rs::App` — using a free function avoids the name clash entirely and matches the research's Pattern 3.
- `HelpScreen::compose()` uses one `Label` widget per content line. This is the simplest correct approach for a display-only screen — no custom `render()` implementation needed.
- Theme names verified against actual `textual_rs::css::theme::builtin_themes()` at compile time. The RESEARCH.md used shorthand aliases ("gruvbox", "catppuccin") while the actual names are "gruvbox-dark" and "catppuccin-mocha". Corrected in `theme.rs`.
- Old insta snapshot `yubitui__tui__help__tests__help_screen.snap` deleted — the old ratatui-direct test no longer exists; replaced by textual-rs `TestApp::new` test.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Theme name aliases in RESEARCH.md don't match actual textual-rs names**
- **Found during:** Task 1 (theme.rs creation)
- **Issue:** RESEARCH.md listed `"gruvbox"` and `"catppuccin"` but `textual_rs::css::theme::builtin_themes()` uses `"gruvbox-dark"` and `"catppuccin-mocha"`. Using wrong names would cause silent fallback to default theme.
- **Fix:** Used the exact `theme.name` strings from the actual textual-rs source (`theme_by_name()` matches on `t.name`).
- **Files modified:** src/tui/theme.rs
- **Verification:** Confirmed by reading `/Users/michael/.cargo/git/checkouts/textual-rs-b56a553ad3b971f3/84e01b6/crates/textual-rs/src/css/theme.rs`
- **Committed in:** 888abff (Task 1 commit)

**2. [Rule 1 - Bug] Old App struct name conflicts with textual_rs::App import**
- **Found during:** Task 1 (app.rs rewrite)
- **Issue:** Plan showed `use textual_rs::App` while keeping the `App` struct — this creates an ambiguous name. The old struct was being entirely replaced so it was a non-issue, but the plan's code snippet showed both in the same file.
- **Fix:** Replaced the entire App struct with a free function `pub fn run(mock: bool)`. `main.rs` updated accordingly. No naming conflict.
- **Files modified:** src/app.rs, src/main.rs
- **Verification:** `cargo check` passes, `grep "struct App {" src/app.rs` returns 0
- **Committed in:** 888abff (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (2 Rule 1 bugs)
**Impact on plan:** Both fixes necessary for correctness. No scope creep.

## Issues Encountered

- Pre-condition: This worktree branch was behind the 08-01 branch (plan 08-01 was executed on worktree-agent-a8f933c0). Fast-forward merged `worktree-agent-a8f933c0` onto this branch before starting work. All 08-01 changes (ratatui 0.30, ClickRegion removal, E2E deletion) are now present.

## Known Stubs

- `src/app.rs`: `_app_state` constructed but not passed to `HelpScreen::new()` — subsequent plans (08-03+) will wire `AppState` through to screens that need it. For the Help screen this is correct (Help displays static content, no YubiKey state needed).
- `src/app.rs`: Hard-coded `HelpScreen` as root widget — this is intentional; subsequent plans migrate remaining 6 screens and a `RootScreen` will be built to dispatch based on `AppState.current_screen`.

## Next Phase Readiness

- Pattern established for all remaining screen migrations (08-03: Diagnostics, 08-04: SSH, 08-05: PIV+Pin, 08-06: Dashboard+Keys)
- `load_theme_from_config()` wired into `app::run()` — theme loading works end-to-end
- 109 tests passing, no regressions

---
*Phase: 08-textual-rs-migration*
*Completed: 2026-03-27*

## Self-Check: PASSED

| Check | Result |
|-------|--------|
| src/tui/config.rs exists | FOUND |
| src/tui/theme.rs exists | FOUND |
| 08-02-SUMMARY.md exists | FOUND |
| Commit 888abff (Task 1) | FOUND |
| Commit f021c5f (Task 2) | FOUND |
| textual_rs::App in app.rs | FOUND |
| Old App struct deleted | CONFIRMED |
| impl Widget for HelpScreen | FOUND |
| cargo check | Passes (0 errors) |
| cargo test | 109 passed, 0 failed |
