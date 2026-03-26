# Phase 7: Mouse Support + E2E Test Harness - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Wire up complete mouse click navigation across all existing screens and build the automated test infrastructure (tmux E2E harness + insta snapshot tests) that every future phase will depend on.

Concrete deliverables:
- `ClickRegionMap` infrastructure: `src/model/click_region.rs` with own `Region` + `ClickRegion` + `ClickAction` types
- All existing screens (Dashboard, Keys, PIV, SSH, Pin, Diagnostics, Help) register click regions in their render functions
- Every keyboard-navigable/activatable element also responds to mouse click
- Mouse scroll works on all list screens
- Windows ConPTY graceful degradation (keyboard-only, no crash)
- `tests/e2e/` shell script harness: one script per screen, runs against `--mock` in CI
- insta snapshot tests: ~3 states per screen, shared mock fixture

Out of scope: new screens (OATH, FIDO2, OTP), new features, TUI library swap, Tauri GUI integration.

</domain>

<decisions>
## Implementation Decisions

### ClickRegionMap architecture (MOUSE-01, MOUSE-03)

- **D-01:** `ClickRegionMap` lives as a field on `AppState` (`src/model/app_state.rs`): `pub click_regions: Vec<ClickRegion>`.
- **D-02:** `ClickRegion` uses a project-owned `Region` struct (no ratatui import in model layer):
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
- **D-03:** `ClickAction` is a wrapping enum with one variant per screen — matches the existing per-screen action enum pattern from Phase 6:
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
- **D-04:** Render functions populate regions by receiving `&mut Vec<ClickRegion>` as an extra parameter. Each render call clears the vec and repopulates it. Regions are always in sync with what was actually rendered — accurate after any terminal resize.
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

### Click target scope (MOUSE-01, MOUSE-02)

- **D-05:** **Full parity rule**: every element that is keyboard-navigable or keyboard-activatable (via arrow keys + Enter) must also respond to mouse click. No exceptions across any existing screen.
- **D-06:** Scroll (MOUSE-02) is required on all screens with lists: Keys, PIV, SSH, Diagnostics.
- **D-07:** Info-only screens (Diagnostics read section) get scroll support but no click-activation needed for non-interactive elements. Help screen gets close-button click.
- **D-08:** **View/model separation is non-negotiable.** ClickRegion infrastructure in `src/model/` uses zero ratatui types. The render layer (tui/) does the coordinate mapping. This keeps the architecture TUI-library-swap ready and Tauri-GUI ready.

### Windows ConPTY degradation (MOUSE-04)

- **D-09:** Wrap `EnableMouseCapture` in a `cfg` or runtime check. On ConPTY failure, log with `tracing::debug!` and continue keyboard-only — no visible error to the user, no crash.

### E2E test harness (TEST-01, TEST-02, TEST-03)

- **D-10:** Shell scripts under `tests/e2e/`. One script per screen:
  `dashboard_smoke.sh`, `keys_smoke.sh`, `piv_smoke.sh`, `ssh_smoke.sh`, `pin_smoke.sh`, `diagnostics_smoke.sh`
- **D-11:** Each script launches `cargo run -- --mock`, navigates to the target screen, performs one key interaction (e.g. open a menu, move selection, open a popup), asserts the result appears in `tmux capture-pane` output, then navigates back to dashboard. Passes = no crash + expected text visible + back at dashboard.
- **D-12:** A `tests/e2e/run_all.sh` driver invokes each screen script and aggregates pass/fail. CI calls `run_all.sh`. Individual scripts remain runnable in isolation for debugging.
- **D-13:** All scripts use `cargo run -- --mock` — never require hardware. Consistent with Phase 6 mock fixture.

### Snapshot tests (TEST-04)

- **D-14:** `insta` crate + ratatui `TestBackend`. Location: `tests/snapshots/` or as `#[test]` modules in `src/tui/<screen>.rs` (planner decides which is cleaner).
- **D-15:** ~3 snapshots per screen covering: (1) default populated state, (2) empty/no-data state (where applicable), (3) one interactive state (menu open, popup visible, selection active).
- **D-16:** All snapshot tests use the **shared mock fixture** from Phase 6 (`src/model/mock.rs` or equivalent). One place to update if fixture changes.

### Claude's Discretion

- Exact file name for `Region`/`ClickRegion` types in `src/model/` (e.g., `click_region.rs`, `regions.rs`)
- Whether `ClickAction` lives alongside `ClickRegion` in the same file or in a separate `src/model/click_action.rs`
- Exact tmux timing / sleep values in E2E scripts (tune to CI environment)
- Whether snapshot tests live in `src/tui/<screen>.rs` test modules or a separate `tests/snapshots/` directory

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` §Mouse Support (MOUSE-01 through MOUSE-04) and §Testing (TEST-01 through TEST-04) — acceptance criteria for all 8 requirements

### Key source files (read before planning)
- `src/app.rs` — `handle_mouse_event()` at line ~145: current dispatch only covers Dashboard + Keys; all other screens need wiring
- `src/tui/dashboard.rs` — existing `handle_mouse()`: only closes context menu on left-click; needs click-activation and region registration
- `src/tui/keys.rs` — existing `handle_mouse()` stub: needs region registration
- `src/model/app_state.rs` — where `click_regions: Vec<ClickRegion>` field gets added
- All per-screen files in `src/tui/`: `dashboard.rs`, `keys.rs`, `piv.rs`, `ssh.rs`, `pin.rs`, `diagnostics.rs`, `help.rs`

### Phase 6 decisions (carry forward)
- `.planning/phases/06-tech-debt-infrastructure/06-CONTEXT.md` — D-09 (per-screen action enums), D-07 (AppState structure), D-08 (App vs AppState split)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `EnableMouseCapture` / `DisableMouseCapture` already in `app.rs` — mouse capture is on, just not fully dispatched
- Per-screen action enums from Phase 6 (`DashboardAction`, `KeyAction`, etc.) — `ClickAction` wraps these directly
- Mock fixture from Phase 6 (`--mock` mode + hardcoded `YubiKeyState`) — shared baseline for both E2E and snapshot tests
- `src/tui/widgets/` — existing popup/dialog widgets already rendered; register their Rects as click regions

### Established Patterns
- `handle_key()` returns typed action → `execute_*_action()` dispatches in `app.rs` — mouse follows same pattern
- `anyhow::bail!` + plain English errors; SW codes to `tracing::debug!` only
- No ratatui in `src/model/` (CI lint enforced from Phase 6)

### Integration Points
- `app.rs::handle_mouse_event()` — extend to dispatch all screens using ClickRegionMap lookup instead of per-screen calls
- Each `render()` call in `app.rs` — pass `&mut self.state.click_regions` as extra parameter
- `AppState` in `src/model/` — add `click_regions: Vec<ClickRegion>` field with `#[serde(skip)]` if Tauri serialization of coordinates isn't desired, OR keep serializable (Region is serde::Serialize)

### Current Gap
- `handle_mouse_event()` only handles `Screen::Dashboard` and `Screen::Keys` — Diagnostics, Help, Pin, PIV, SSH all fall through `_ => {}`
- Dashboard's `handle_mouse()` only closes context menu; doesn't click-activate nav items or menu entries
- No `tests/e2e/` directory exists yet
- No insta dependency in Cargo.toml yet

</code_context>

<specifics>
## Specific Ideas

- The user's guiding principle: **"every element that was highlightable/interactable via arrow keys and Enter should also have mouse support"** — use this as the checklist when auditing each screen.
- Architecture note from user: **"keep the view logic separate because the TUI library will change, and we need to add GUI (Tauri)"** — Region type in model/ is the concrete mechanism for this. No ratatui::Rect leaks into model layer.
- The Region type approach means Tauri can later use the same ClickRegion data to build its own hit-test map from the model's state — the architecture is intentionally Tauri-forward.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 07-mouse-support-e2e-test-harness*
*Context gathered: 2026-03-26*
