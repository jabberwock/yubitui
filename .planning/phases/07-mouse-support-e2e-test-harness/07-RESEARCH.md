# Phase 7: Mouse Support + E2E Test Harness - Research

**Researched:** 2026-03-26
**Domain:** Ratatui mouse hit-testing, crossterm ConPTY, insta snapshot tests, tmux E2E harness
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**ClickRegionMap architecture (MOUSE-01, MOUSE-03)**

- D-01: `ClickRegionMap` lives as a field on `AppState` (`src/model/app_state.rs`): `pub click_regions: Vec<ClickRegion>`.
- D-02: `ClickRegion` uses a project-owned `Region` struct (no ratatui import in model layer):
  ```rust
  // src/model/click_region.rs
  #[derive(serde::Serialize, Clone, Debug)]
  pub struct Region { pub x: u16, pub y: u16, pub w: u16, pub h: u16 }

  pub struct ClickRegion {
      pub region: Region,
      pub action: ClickAction,
  }
  ```
  The tui layer maps `ratatui::Rect → Region` at registration time. CI lint (`no ratatui in src/model/`) continues to pass.
- D-03: `ClickAction` is a wrapping enum with one variant per screen — matches the existing per-screen action enum pattern from Phase 6:
  ```rust
  pub enum ClickAction {
      Dashboard(DashboardAction),
      Keys(KeyAction),
      Pin(PinAction),
      Piv(PivAction),
      Ssh(SshAction),
      Diagnostics(DiagnosticsAction),
      Help(HelpAction),
  }
  ```
- D-04: Render functions populate regions by receiving `&mut Vec<ClickRegion>` as an extra parameter. Each render call clears the vec and repopulates it. Regions are always in sync with what was actually rendered — accurate after any terminal resize.
  ```rust
  pub fn render(frame: &mut Frame, area: Rect,
                app: &App, state: &DashboardState,
                click_regions: &mut Vec<ClickRegion>) {
      click_regions.clear();
      // ... compute layout, render widgets ...
      click_regions.push(ClickRegion {
          region: Region::from(nav_area), // ratatui::Rect → Region
          action: ClickAction::Dashboard(DashboardAction::OpenKeys),
      });
  }
  ```

**Click target scope (MOUSE-01, MOUSE-02)**

- D-05: Full parity rule: every element that is keyboard-navigable or keyboard-activatable (via arrow keys + Enter) must also respond to mouse click. No exceptions across any existing screen.
- D-06: Scroll (MOUSE-02) is required on all screens with lists: Keys, PIV, SSH, Diagnostics.
- D-07: Info-only screens (Diagnostics read section) get scroll support but no click-activation needed for non-interactive elements. Help screen gets close-button click.
- D-08: View/model separation is non-negotiable. ClickRegion infrastructure in `src/model/` uses zero ratatui types. The render layer (tui/) does the coordinate mapping.

**Windows ConPTY degradation (MOUSE-04)**

- D-09: Wrap `EnableMouseCapture` in a `cfg` or runtime check. On ConPTY failure, log with `tracing::debug!` and continue keyboard-only — no visible error to the user, no crash.

**E2E test harness (TEST-01, TEST-02, TEST-03)**

- D-10: Shell scripts under `tests/e2e/`. One script per screen:
  `dashboard_smoke.sh`, `keys_smoke.sh`, `piv_smoke.sh`, `ssh_smoke.sh`, `pin_smoke.sh`, `diagnostics_smoke.sh`
- D-11: Each script launches `cargo run -- --mock`, navigates to the target screen, performs one key interaction, asserts result in `tmux capture-pane` output, navigates back to dashboard.
- D-12: A `tests/e2e/run_all.sh` driver invokes each screen script and aggregates pass/fail. CI calls `run_all.sh`.
- D-13: All scripts use `cargo run -- --mock` — never require hardware.

**Snapshot tests (TEST-04)**

- D-14: `insta` crate + ratatui `TestBackend`. Location: `tests/snapshots/` or as `#[test]` modules in `src/tui/<screen>.rs` (planner decides).
- D-15: ~3 snapshots per screen: (1) default populated state, (2) empty/no-data state (where applicable), (3) one interactive state.
- D-16: All snapshot tests use the shared mock fixture from Phase 6 (`src/model/mock.rs`).

### Claude's Discretion

- Exact file name for `Region`/`ClickRegion` types in `src/model/` (e.g., `click_region.rs`, `regions.rs`)
- Whether `ClickAction` lives alongside `ClickRegion` in the same file or in a separate `src/model/click_action.rs`
- Exact tmux timing / sleep values in E2E scripts (tune to CI environment)
- Whether snapshot tests live in `src/tui/<screen>.rs` test modules or a separate `tests/snapshots/` directory

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| MOUSE-01 | User can click any navigation item, menu entry, or button to activate it | ClickRegionMap + `MouseEvent.column/row` hit-test against `Region.contains()` |
| MOUSE-02 | User can scroll lists with the mouse wheel | `MouseEventKind::ScrollUp/ScrollDown` already partially implemented in keys.rs; extend to PIV, SSH, Diagnostics |
| MOUSE-03 | Mouse click regions use a `ClickRegionMap` rebuilt each frame — coordinates always accurate after resize | D-04: clear-and-repopulate in every `render()` call; `Region` carries `x/y/w/h` for contains check |
| MOUSE-04 | On Windows (ConPTY), mouse events degrade gracefully to keyboard-only | `execute!()` returns `Err` on ConPTY; catch with `if let Err` + `tracing::debug!`; continue without crash |
| TEST-01 | E2E test harness under `tests/e2e/` using tmux; runs without hardware via `--mock` | tmux 3.6a confirmed available; `cargo run -- --mock` confirmed working |
| TEST-02 | All existing screens have at least one tmux E2E smoke test | 7 screens confirmed (Dashboard, Keys, PIV, SSH, Pin, Diagnostics, Help); 6 screen scripts + run_all.sh |
| TEST-03 | New screens each have tmux E2E tests written before/alongside implementation (TDD) | D-13 establishes the harness pattern; TEST-03 scope extends to Phase 8+ screens only |
| TEST-04 | Ratatui TestBackend + insta snapshot tests cover each screen's key states | insta 1.47.0 confirmed available; ratatui 0.29.0 `TestBackend` confirmed; official recipe pattern documented |

</phase_requirements>

---

## Summary

Phase 7 has two independent workstreams that can be parallelized: (1) mouse hit-testing infrastructure and (2) automated test infrastructure. Both are well-understood problems with no research spikes needed — the architecture is locked in CONTEXT.md and the codebase is already partially wired for both.

The mouse work is an extension, not a rewrite. `EnableMouseCapture` is already active in `app.rs`. `handle_mouse_event()` already dispatches for Dashboard and Keys. The gap is: (a) building the `ClickRegionMap` infrastructure so hit-testing is region-based rather than per-screen ad-hoc, (b) extending dispatch to all 7 screens, (c) making every render function register its interactive elements, and (d) wrapping the Windows ConPTY path. The `MouseEvent.column` and `MouseEvent.row` fields from crossterm give terminal coordinates that map directly against `Region { x, y, w, h }`.

The test work is new infrastructure but follows well-documented patterns. The insta crate (1.47.0) + ratatui `TestBackend` pattern is officially documented by the ratatui project. The tmux E2E harness follows the `send-keys` / `sleep` / `capture-pane` / `grep` pattern common in TUI testing. tmux 3.6a is confirmed installed on the dev machine.

**Primary recommendation:** Build `src/model/click_region.rs` first as the shared type foundation, then wire all render functions, then implement hit-test dispatch in `handle_mouse_event()`. E2E and snapshot work can proceed in parallel once `--mock` mode is verified still compiling cleanly from Phase 6.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.29.0 (locked) | TUI rendering + `TestBackend` | Already in use; `TestBackend` is the official testing path |
| crossterm | 0.28.1 (locked) | Mouse event types (`MouseEvent`, `MouseEventKind`, `MouseButton`) | Already in use; provides `column`/`row` coordinates on every mouse event |
| insta | 1.47.0 (latest) | Snapshot assertion for Rust | Standard Rust snapshot testing library; officially recommended by ratatui docs |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| cargo-insta | 1.47.0 | CLI review tool for accepting/rejecting snapshot diffs | Run `cargo insta review` after intentional UI changes |
| tmux | 3.6a (system) | Session management for E2E harness | Required for `send-keys` + `capture-pane` E2E test pattern |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| insta | manual `assert_eq!(backend.buffer(), ...)` | No diff-review workflow, no stored `.snap` files, harder to update |
| tmux E2E | expect / pexpect | Requires Python runtime; tmux is already installed and language-agnostic |

**Installation:**

```bash
# Add to [dev-dependencies] in Cargo.toml
cargo add insta --dev
# Install cargo-insta CLI for snapshot review
cargo install cargo-insta
```

**Version verification:** insta 1.47.0 confirmed via `cargo search insta` on 2026-03-26. ratatui 0.29.0 and crossterm 0.28.1 confirmed via `cargo metadata`.

---

## Architecture Patterns

### Recommended Project Structure

```
src/
├── model/
│   ├── click_region.rs    # Region, ClickRegion, ClickAction (NEW — no ratatui imports)
│   ├── app_state.rs       # Add: pub click_regions: Vec<ClickRegion>
│   └── mod.rs             # Export click_region types
├── tui/
│   ├── dashboard.rs       # render() gains &mut Vec<ClickRegion> param; registers nav items, menu entries
│   ├── keys.rs            # render() gains &mut Vec<ClickRegion> param; registers action buttons, list items
│   ├── pin.rs             # render() gains param; registers PIN operation buttons
│   ├── piv.rs             # render() gains param; registers back button (info-only, minimal)
│   ├── ssh.rs             # render() gains param; registers wizard step buttons
│   ├── diagnostics.rs     # render() gains param; registers back button
│   └── help.rs            # render() gains param; registers close button
tests/
├── e2e/
│   ├── run_all.sh         # Driver: invokes all screen scripts, aggregates pass/fail
│   ├── dashboard_smoke.sh
│   ├── keys_smoke.sh
│   ├── piv_smoke.sh
│   ├── ssh_smoke.sh
│   ├── pin_smoke.sh
│   └── diagnostics_smoke.sh
└── snapshots/             # (if planner chooses external location)
    └── <screen>__<test_name>.snap
```

### Pattern 1: Region Type and Hit-Test

**What:** A project-owned rectangle type in the model layer that covers the screen area of a rendered element.
**When to use:** Any time a render function needs to register a clickable element.

```rust
// src/model/click_region.rs
// Source: CONTEXT.md D-02, crossterm MouseEvent.column/row docs
#[derive(serde::Serialize, Clone, Debug)]
pub struct Region {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Region {
    pub fn contains(&self, col: u16, row: u16) -> bool {
        col >= self.x && col < self.x + self.w
            && row >= self.y && row < self.y + self.h
    }
}

// Conversion from ratatui::Rect — lives in tui/ layer, not model/
// (impl From<ratatui::Rect> for Region placed in a tui/click_region_ext.rs or inline in each screen)
impl From<ratatui::layout::Rect> for crate::model::click_region::Region {
    fn from(r: ratatui::layout::Rect) -> Self {
        Self { x: r.x, y: r.y, w: r.width, h: r.height }
    }
}
```

### Pattern 2: ClickRegionMap Dispatch in app.rs

**What:** Replaces per-screen ad-hoc mouse handling with a single region scan.
**When to use:** In `handle_mouse_event()` for `MouseEventKind::Down(MouseButton::Left)`.

```rust
// src/app.rs — handle_mouse_event() rewrite
// Source: CONTEXT.md D-04, crossterm MouseEvent field docs
fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<()> {
    use crossterm::event::MouseEventKind;
    match mouse.kind {
        MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
            let col = mouse.column;
            let row = mouse.row;
            // Scan registered regions; first match wins
            if let Some(region_entry) = self.state.click_regions.iter()
                .find(|r| r.region.contains(col, row))
            {
                let action = region_entry.action.clone();
                self.execute_click_action(action)?;
            }
        }
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
            // Delegate scroll to per-screen handlers (already exists for Keys/Dashboard)
            // Extend for PIV, SSH, Diagnostics
            self.handle_scroll_event(mouse)?;
        }
        _ => {}
    }
    Ok(())
}
```

### Pattern 3: Render Function Signature Change

**What:** Every render function in `src/tui/` gains an extra `&mut Vec<ClickRegion>` parameter.
**When to use:** All 7 screen render functions.

```rust
// Before (example from piv.rs):
pub fn render(frame: &mut Frame, area: Rect, yubikey_state: &Option<YubiKeyState>) { ... }

// After:
pub fn render(frame: &mut Frame, area: Rect, yubikey_state: &Option<YubiKeyState>,
              click_regions: &mut Vec<crate::model::click_region::ClickRegion>) {
    click_regions.clear();
    // ... compute layout ...
    click_regions.push(ClickRegion {
        region: back_button_rect.into(),
        action: ClickAction::Piv(PivAction::NavigateTo(Screen::Dashboard)),
    });
}
```

The call site in `app.rs::render()` becomes:
```rust
Screen::Piv => {
    let yk = self.yubikey_state().cloned();
    crate::tui::piv::render(frame, chunks[0], &yk,
                            &mut self.state.click_regions)
}
```

### Pattern 4: Windows ConPTY Graceful Degradation

**What:** Wrap `execute!(stdout, EnableMouseCapture)` so that failure on Windows ConPTY is silently swallowed.
**When to use:** `app.rs::run()` setup block.

```rust
// Source: CONTEXT.md D-09; crossterm issue #168 confirms ConPTY incompatibility
// Before:
execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

// After:
execute!(stdout, EnterAlternateScreen)?;
if let Err(e) = execute!(stdout, EnableMouseCapture) {
    tracing::debug!("Mouse capture unavailable (likely ConPTY): {}", e);
    // Continue keyboard-only — no crash, no user-visible error
}
```

The same pattern applies to `DisableMouseCapture` in the cleanup block.

### Pattern 5: insta Snapshot Test with TestBackend

**What:** Render a screen to a fixed-size `TestBackend`, then snapshot the buffer output.
**When to use:** Each screen in `src/tui/<screen>.rs` test module or `tests/snapshots/<screen>.rs`.

```rust
// Source: ratatui.rs/recipes/testing/snapshots/ (official recipe)
#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};
    use crate::model::mock::mock_yubikey_states;

    #[test]
    fn dashboard_default_state() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let yk_states = mock_yubikey_states();
        let state = crate::tui::dashboard::DashboardState::default();
        let mut click_regions = Vec::new();
        terminal.draw(|frame| {
            // pass a minimal App-like struct or restructure to accept state directly
            crate::tui::dashboard::render(frame, frame.area(), /* app ref */, &state, &mut click_regions);
        }).unwrap();
        assert_snapshot!(terminal.backend());
    }
}
```

**Note on App coupling:** Several render functions currently accept `&App` (dashboard.rs, ssh.rs). The planner must decide whether to pass a thin `AppView` data struct instead, or restructure the snapshot test to construct a full App in mock mode. The cleanest path is to refactor render functions to accept `&AppState` instead of `&App` — this also benefits the Tauri-forward architecture (AppState is already `Serialize`).

### Pattern 6: tmux E2E Smoke Test Script

**What:** Shell script that launches the app, navigates to a screen, asserts text, then cleans up.
**When to use:** One script per screen; `run_all.sh` drives all of them.

```bash
#!/usr/bin/env bash
# tests/e2e/dashboard_smoke.sh
set -euo pipefail

SESSION="yubitui_e2e_$$"
BINARY="cargo run --quiet -- --mock"

# Start a new detached tmux session
tmux new-session -d -s "$SESSION" -x 200 -y 50 -- bash -c "$BINARY"
sleep 1  # Wait for app to start and render

# Assert: dashboard title visible
OUTPUT=$(tmux capture-pane -t "$SESSION" -p)
if ! echo "$OUTPUT" | grep -q "YubiTUI"; then
    echo "FAIL: dashboard_smoke - 'YubiTUI' not found in output"
    tmux kill-session -t "$SESSION" 2>/dev/null || true
    exit 1
fi

# Assert: navigation instructions visible
if ! echo "$OUTPUT" | grep -q "Dashboard"; then
    echo "FAIL: dashboard_smoke - 'Dashboard' not found in output"
    tmux kill-session -t "$SESSION" 2>/dev/null || true
    exit 1
fi

# Cleanup
tmux send-keys -t "$SESSION" "q" ""
sleep 0.3
tmux kill-session -t "$SESSION" 2>/dev/null || true

echo "PASS: dashboard_smoke"
```

**Key pattern for navigation:** Send a key with a literal flag:
```bash
# Navigate to Keys screen
tmux send-keys -t "$SESSION" "3" ""
sleep 0.5  # Wait for render

# Assert Keys screen content
OUTPUT=$(tmux capture-pane -t "$SESSION" -p)
echo "$OUTPUT" | grep -q "Key Management" || { echo "FAIL"; cleanup; exit 1; }
```

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Snapshot diffing/storage | Custom string comparison + file management | `insta` crate | insta handles `.snap` file creation, review workflow, CI pass/fail, inline snapshots |
| Terminal emulation for tests | PTY wrapper, custom backend | `ratatui::TestBackend` | TestBackend is the official synchronous in-process render target; zero PTY overhead |
| Mouse coordinate math | Custom Rect overlap logic | `Region::contains()` (3 lines) | This IS hand-rolled intentionally — it is trivial and must avoid ratatui dependency in model layer |
| E2E process management | Custom process spawner | tmux + shell | tmux is already installed, handles terminal size, PTY, and capture reliably |

**Key insight:** The only custom code that's actually appropriate to write is the `Region::contains()` hit-test — it's 3 lines and must be in `src/model/` without ratatui imports. Everything else (snapshot testing, E2E harness) has excellent off-the-shelf solutions.

---

## Common Pitfalls

### Pitfall 1: Render Function Coupling to `&App` Blocks Snapshot Tests

**What goes wrong:** `dashboard::render()` and `ssh::render()` accept `&crate::app::App`. Creating an `App` in a test requires a full mock setup including terminal state — this is heavyweight and couples tests to the TUI runtime.
**Why it happens:** The render functions were written to access app-level helpers (`app.yubikey_state()`, `app.yubikey_count()`).
**How to avoid:** Refactor those render functions to accept `&AppState` (which is `Clone + Serialize` and lives in `src/model/`) instead of `&App`. The helpers `yubikey_state()` and `yubikey_count()` can be moved to `AppState` methods. This is also the Tauri-forward architecture move.
**Warning signs:** If snapshot test setup requires `App::new(true)` (full init), the abstraction is leaking.

### Pitfall 2: Click Regions Go Stale After Resize

**What goes wrong:** If regions are populated once at startup and not refreshed, a terminal resize changes the layout but the stored `Region` coordinates still point to pre-resize positions.
**Why it happens:** Terminal resize invalidates all ratatui layout computations.
**How to avoid:** D-04 mandates `click_regions.clear()` at the start of every render call. Since `render()` is called every event loop iteration, regions are always current. The planner must ensure the clear-and-repopulate pattern is enforced in every render function, not just dashboard.
**Warning signs:** Mouse clicks activate wrong elements after resize (MOUSE-03 acceptance criterion explicitly tests this).

### Pitfall 3: ClickAction Enum Requires Clone on Action Enums

**What goes wrong:** The `ClickAction` dispatch in `handle_mouse_event()` needs to clone the found action before executing it (because `execute_click_action` takes ownership while the borrow of `self.state.click_regions` is live).
**Why it happens:** Rust borrow checker: cannot mutably borrow `self` while immutably borrowing `self.state.click_regions` through the iterator.
**How to avoid:** Derive `Clone` on all per-screen action enums (`DashboardAction`, `KeyAction`, etc.) and on `ClickAction`. Currently these enums do NOT derive `Clone` — this is a prerequisite change. Alternatively, collect the matching action to a local variable before calling the executor, then drop the borrow.
**Warning signs:** `cannot borrow 'self' as mutable because it is also borrowed as immutable` compiler error in `handle_mouse_event()`.

### Pitfall 4: tmux Session Name Collision in CI

**What goes wrong:** If two CI jobs run simultaneously, tmux session names clash and `new-session` fails.
**Why it happens:** Fixed session names like `"yubitui_e2e"` are globally unique per tmux server.
**How to avoid:** Append PID to session name: `SESSION="yubitui_e2e_$$"`. Already shown in the example above.
**Warning signs:** `duplicate session: yubitui_e2e` error in CI logs.

### Pitfall 5: insta Snapshots Fail in CI Without `INSTA_UPDATE=unseen`

**What goes wrong:** On first CI run, snapshot files don't exist yet. `assert_snapshot!` fails with "snapshot not found" rather than creating the file.
**Why it happens:** insta by default requires snapshot files to exist and match.
**How to avoid:** On the first run (Wave 0), run `cargo insta review` locally to accept all initial snapshots and commit the `.snap` files to git. Set `INSTA_UPDATE=no` in CI to ensure CI never silently creates new snapshots. Run `cargo test` locally after accepting to confirm green.
**Warning signs:** "snapshot assertion for 'X' failed" in CI with no diff (file missing).

### Pitfall 6: `render_context_menu` Returns No Rect — Cannot Register Click Regions

**What goes wrong:** The popup widget in `src/tui/widgets/popup.rs::render_context_menu()` computes the popup area internally via `centered_area()` and does not return it. The dashboard render function cannot register a click region for the popup without knowing its position.
**Why it happens:** The popup helpers were designed for rendering only, not for hit-testing.
**How to avoid:** Modify `render_context_menu()` to return the computed `Rect` (or the individual item `Rect`s). Alternatively, replicate the `centered_area()` calculation in the dashboard render function to predict the popup position. The return-the-rect approach is cleaner.
**Warning signs:** Context menu items don't respond to mouse click even after ClickRegion infrastructure is in place.

---

## Code Examples

### Complete Region Type (src/model/click_region.rs)

```rust
// Source: CONTEXT.md D-02; verified pattern
use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct Region {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Region {
    pub fn contains(&self, col: u16, row: u16) -> bool {
        col >= self.x
            && col < self.x.saturating_add(self.w)
            && row >= self.y
            && row < self.y.saturating_add(self.h)
    }
}

// ClickAction and ClickRegion in same file (Claude's Discretion — single file is cleaner)
#[derive(Clone, Debug)]
pub enum ClickAction {
    Dashboard(crate::tui::dashboard::DashboardAction),
    Keys(crate::tui::keys::KeyAction),
    Pin(crate::tui::pin::PinAction),
    Piv(crate::tui::piv::PivAction),
    Ssh(crate::tui::ssh::SshAction),
    Diagnostics(crate::tui::diagnostics::DiagnosticsAction),
    Help(crate::tui::help::HelpAction),
}

#[derive(Clone, Debug)]
pub struct ClickRegion {
    pub region: Region,
    pub action: ClickAction,
}
```

**Note:** `ClickAction` contains TUI action types, creating a `model → tui` import. This is a circular dependency risk if `src/model/` imports from `src/tui/`. Resolution: move the per-screen action enums to `src/model/actions/` (already suggested in Phase 6 D-09 pattern), OR keep `ClickAction` in `src/tui/` and use a model-only `ClickRegionKey` enum in AppState. The planner should resolve this — the simplest path that avoids circular imports is to move action enums to `src/model/actions.rs`.

### Rect-to-Region Conversion (tui layer)

```rust
// Placed in src/tui/mod.rs or inline in each screen file
// Source: verified crossterm/ratatui Rect fields
impl From<ratatui::layout::Rect> for crate::model::click_region::Region {
    fn from(r: ratatui::layout::Rect) -> Self {
        crate::model::click_region::Region {
            x: r.x,
            y: r.y,
            w: r.width,
            h: r.height,
        }
    }
}
```

### AppState Click Region Field

```rust
// src/model/app_state.rs — add to AppState struct
#[derive(Debug, Clone, Serialize)]
pub struct AppState {
    pub current_screen: Screen,
    pub previous_screen: Screen,
    pub should_quit: bool,
    pub yubikey_states: Vec<super::YubiKeyState>,
    pub selected_yubikey_idx: usize,
    pub mock_mode: bool,
    #[serde(skip)]  // Coordinates are ephemeral; no value serializing them to Tauri
    pub click_regions: Vec<crate::model::click_region::ClickRegion>,
}
```

### tmux run_all.sh Driver

```bash
#!/usr/bin/env bash
# tests/e2e/run_all.sh
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PASS=0
FAIL=0

for script in "$SCRIPT_DIR"/*_smoke.sh; do
    if bash "$script"; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
    fi
done

echo ""
echo "E2E Results: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]  # exit 0 on success, 1 on any failure
```

### Snapshot Test for Dashboard (in test module)

```rust
// src/tui/dashboard.rs or tests/snapshots/dashboard.rs
#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};
    use crate::model::{mock::mock_yubikey_states, AppState, Screen};

    fn make_app_state_mock() -> AppState {
        AppState {
            yubikey_states: mock_yubikey_states(),
            mock_mode: true,
            ..AppState::default()
        }
    }

    #[test]
    fn dashboard_default_populated() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = super::DashboardState::default();
        let app_state = make_app_state_mock();
        let mut click_regions = Vec::new();
        terminal.draw(|frame| {
            super::render(frame, frame.area(), &app_state, &state, &mut click_regions);
        }).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn dashboard_no_yubikey() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = super::DashboardState::default();
        let app_state = AppState { yubikey_states: vec![], ..AppState::default() };
        let mut click_regions = Vec::new();
        terminal.draw(|frame| {
            super::render(frame, frame.area(), &app_state, &state, &mut click_regions);
        }).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn dashboard_context_menu_open() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = super::DashboardState { show_context_menu: true, menu_selected_index: 2 };
        let app_state = make_app_state_mock();
        let mut click_regions = Vec::new();
        terminal.draw(|frame| {
            super::render(frame, frame.area(), &app_state, &state, &mut click_regions);
        }).unwrap();
        assert_snapshot!(terminal.backend());
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Per-screen ad-hoc mouse handling (check hardcoded row/col) | `ClickRegionMap` rebuilt every frame | Phase 7 (this phase) | Resize-safe; scales to N screens without per-screen math |
| No automated TUI tests | tmux E2E + insta snapshot | Phase 7 (this phase) | Future phases must include tests before marking complete |
| `assert_eq!(render_output, "...")` string matching | `assert_snapshot!()` with review workflow | Phase 7 (this phase) | Diff-based review; easy to update for intentional changes |

**Deprecated/outdated:**

- Per-screen `handle_mouse()` stubs that ignore coordinates: replaced by unified ClickRegionMap dispatch in `handle_mouse_event()`. The old stubs (`dashboard::handle_mouse`, `keys::handle_mouse`) are removed and their scroll logic is migrated to a centralized `handle_scroll_event()`.

---

## Open Questions

1. **Circular dependency: action enums in model vs. tui**
   - What we know: `ClickAction` wraps per-screen action enums. Those enums currently live in `src/tui/<screen>.rs`. Moving them to `src/model/actions.rs` resolves the dependency and is consistent with INFRA-05 (per-screen action enums) and INFRA-03 (model/tui separation).
   - What's unclear: Phase 6 completed INFRA-05 but did it move the enums to `src/model/` or leave them in `src/tui/`? Inspection shows they are still in `src/tui/` (e.g., `DashboardAction` in `dashboard.rs`).
   - Recommendation: The planner's Wave 0 task should migrate action enums to `src/model/actions/` (or `src/model/actions.rs`). This is a prerequisite for `ClickAction` to live cleanly in `src/model/click_region.rs` without circular imports. Alternatively, keep `ClickAction` in `src/tui/click_dispatch.rs` and keep `AppState.click_regions` using a plain `Vec<(Region, ClickAction)>` without the `ClickRegion` wrapper needing to be in model — though this violates D-08's intent.

2. **`render()` functions accept `&App` — needs decoupling for snapshot tests**
   - What we know: `dashboard::render()` and `ssh::render()` take `&crate::app::App`, which requires a full App construction in tests.
   - What's unclear: What data from `App` do these functions actually use? Inspection shows: `app.yubikey_state()`, `app.yubikey_count()`, `app.selected_yubikey_idx()`, `app.state.mock_mode` (likely).
   - Recommendation: Wave 0 should replace `&App` params with `&AppState` in both render functions. `yubikey_state()` and `yubikey_count()` become methods on `AppState`. This is a small refactor with big benefit for testability and Tauri-forward architecture.

3. **`render_context_menu` needs to return its computed Rect**
   - What we know: The function computes `centered_area()` internally and does not expose the resulting `Rect`.
   - What's unclear: Whether the planner should return `Rect` from the function or replicate the calculation.
   - Recommendation: Modify `render_context_menu()` to `-> Rect` and return `popup_area`. The dashboard render function then uses the returned rect to register item click regions.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| tmux | E2E test harness (TEST-01, TEST-02) | Yes | 3.6a | None needed |
| cargo / rustc | All compilation + tests | Yes | (project MSRV 1.75) | None needed |
| bash | E2E shell scripts | Yes (macOS/Linux) | system | None needed |
| insta (crate) | TEST-04 snapshot tests | Not yet in Cargo.toml | 1.47.0 available | None needed — add to dev-deps |
| cargo-insta (CLI) | Snapshot review workflow | Not installed | 1.47.0 available | Wave 0 install step |

**Missing dependencies with no fallback:**
- None that block execution.

**Missing dependencies with fallback:**
- `insta` dev-dependency: not yet in Cargo.toml. Wave 0 must add `insta = "1.47"` to `[dev-dependencies]`. No fallback needed — the install is trivial.
- `cargo-insta` CLI: must be installed by developer before running `cargo insta review`. Not required for CI (CI runs `cargo test`; snapshot `.snap` files must already be committed).

---

## Validation Architecture

Skipped — `workflow.nyquist_validation` is explicitly set to `false` in `.planning/config.json`.

---

## Project Constraints (from CLAUDE.md)

CLAUDE.md does not exist in the working directory. The following constraints are inferred from the memory files and project decisions documented in STATE.md and CONTEXT.md:

- **NEVER use ykman**: All operations must be native PC/SC APDUs. (Memory: `feedback_no_ykman.md`)
- **No ratatui imports in `src/model/`**: CI lint enforced from Phase 6. `Region` and `ClickRegion` in `src/model/click_region.rs` must have zero ratatui imports.
- **Run `cargo test` before asking user to verify**: Stop hook auto-runs tests. (Memory: `feedback_self_test_before_verify.md`)
- **Run tmux E2E tests before asking user to verify**: TDD required; no "prior bugs" excuses; ship only 100% working features. (Memory: `feedback_tmux_e2e_tests.md`)
- **No AI signatures in git commits**: (Memory: `feedback_no_ai_signature.md`)
- **`gpg --edit-key` state machine**: Not directly applicable to this phase. (Memory: `feedback_gpg_interactive_state_machine.md`)
- **UI/data separation**: No ratatui imports in business logic. TUI library swap + Tauri GUI are planned. (Memory: `project_ui_data_separation.md`) — directly drives the `Region` type decision.
- **`tracing::debug!` for SW codes and non-user-visible info**: ConPTY failure logged at debug level, not user-visible.

---

## Sources

### Primary (HIGH confidence)

- crossterm 0.28.1 `MouseEvent` struct — `column: u16`, `row: u16`, `kind: MouseEventKind` fields confirmed via docs.rs
- ratatui 0.29.0 `TestBackend` — `assert_snapshot!(terminal.backend())` pattern confirmed via [official ratatui recipe](https://ratatui.rs/recipes/testing/snapshots/)
- insta 1.47.0 — `assert_snapshot!` macro API confirmed via [docs.rs/insta](https://docs.rs/insta/latest/insta/)
- Cargo.toml — ratatui 0.29.0, crossterm 0.28.1 locked versions confirmed via `cargo metadata`
- CONTEXT.md (07-CONTEXT.md) — all architectural decisions locked (D-01 through D-16)
- Source code inspection: `src/app.rs`, `src/model/app_state.rs`, all `src/tui/*.rs` files

### Secondary (MEDIUM confidence)

- crossterm Windows ConPTY compatibility: [Issue #168](https://github.com/crossterm-rs/crossterm/issues/168) confirms ConPTY incompatibility with WinAPI calls; `execute!()` returning `Err` is the expected failure mode.
- tmux 3.6a confirmed installed at `/opt/homebrew/bin/tmux` on the dev machine.

### Tertiary (LOW confidence)

- tmux E2E pattern (send-keys + sleep + capture-pane + grep): community-documented pattern, no authoritative spec, but universally consistent across multiple sources.

---

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH — versions confirmed via `cargo metadata` and `cargo search`
- Architecture: HIGH — locked in CONTEXT.md; source code confirmed existing patterns
- Pitfalls: HIGH for Pitfalls 1-3 (found in source code), MEDIUM for 4-6 (inferred from patterns)
- E2E harness: MEDIUM — tmux mechanics confirmed, specific sleep timings need tuning in CI

**Research date:** 2026-03-26
**Valid until:** 2026-06-26 (stable libraries; insta and ratatui change infrequently)
