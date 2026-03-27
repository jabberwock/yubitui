# Phase 8: textual-rs Migration - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Migrate all 7 existing screens from raw ratatui widget composition to textual-rs components. The `src/model/` layer is completely untouched — this is a pure TUI layer replacement.

Concrete deliverables:
- All 7 screens rebuilt as textual-rs widgets: Dashboard, Keys, Pin, SSH, Diagnostics, PIV, Help
- `src/tui/` rewritten using textual-rs component model (compose, render, key_bindings, on_action)
- `app.rs` event loop replaced by textual-rs App runner
- `ClickRegionMap` infrastructure retired — textual-rs Button widgets provide explicit click targets natively
- tmux E2E harness (`tests/e2e/`) retired — replaced by textual-rs Pilot-based tests in `cargo test`
- insta snapshot tests kept — textual-rs renders to ratatui Buffer underneath, snapshots still valid
- User-configurable theme selection from textual-rs built-ins

Out of scope: new screens (OATH, FIDO2, OTP), new features, model layer changes, Tauri GUI.

</domain>

<decisions>
## Implementation Decisions

### TUI library (locked)
- **D-01:** textual-rs replaces raw ratatui widget composition. textual-rs sits on top of ratatui — same rendering engine, higher-level component model. Not a ratatui removal.
- **D-02:** All 7 existing screens migrate in this phase: Dashboard, Keys, Pin, SSH, Diagnostics, PIV, Help. No partial migration — all screens use textual-rs by phase end.

### Model layer (locked)
- **D-03:** `src/model/` is byte-for-byte unchanged by this migration. Zero model layer changes are permitted. If a migration task requires touching model code, that is a bug in the plan.
- **D-04:** All `#[derive(serde::Serialize)]` types, action enums, handle_key functions in `src/tui/` remain — they become the action layer that textual-rs `on_action()` calls dispatch to.

### Click targets (locked)
- **D-05:** The manual `ClickRegionMap` / `Vec<ClickRegion>` infrastructure is retired. textual-rs Button widgets are the click target primitive — explicit, visually bounded, mouse-compliant by default.
- **D-06:** Every previously keyboard-navigable element becomes a textual-rs Button or interactive widget. Rule-of-thirds layout via textual-rs flexbox/grid (TCSS). No manual hit-region math.

### Keyboard shortcuts visibility (locked)
- **D-07:** textual-rs Footer widget renders keybindings on-screen at all times. Every screen declares its bindings via `key_bindings()` — no more "no shortcuts visible anywhere" problem.

### Test harness (locked)
- **D-08:** tmux E2E harness (`tests/e2e/` shell scripts + run_all.sh) is retired in this phase.
- **D-09:** All screen coverage is replaced by textual-rs Pilot-based tests (`TestApp` + `pilot.press()` / `pilot.type_text()` / assertions). These run inside `cargo test` — no tmux, no timing races, no CI flakiness.
- **D-10:** insta snapshot tests are kept. textual-rs renders to a ratatui Buffer underneath — existing snapshots remain valid and continue to serve as regression guards.

### Themes (locked)
- **D-11:** User can select a theme from the textual-rs built-ins: tokyo-night, nord, gruvbox, dracula, catppuccin. Theme choice is persisted (config file or env var — Claude's discretion on mechanism).
- **D-12:** No default theme is locked in by the user — Claude picks the most neutral/readable default. User can change it at runtime or via config.

### Design direction (locked)
- **D-13:** Rule-of-thirds layout using textual-rs CSS grid/flex. Screens are visually structured — not a wall of text with scattered key hints.
- **D-14:** Mouse regions must be visually obvious. Button widgets with borders/styling make click targets self-evident without documentation.
- **D-15:** Keyboard shortcuts are visible on every screen via Footer — not buried in a help screen.

### Claude's Discretion
- Exact TCSS styling per screen (colors, padding, borders within theme)
- Whether Screen navigation uses textual-rs screen stack (push/pop_screen_deferred) or retains the flat Screen enum model
- Config mechanism for theme persistence (TOML file, env var, etc.)
- How the existing per-screen action enums integrate with textual-rs on_action() dispatch
- Order of screen migration within the phase (waves)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### textual-rs
- https://github.com/jabberwock/textual-rs/blob/master/docs/guide.md — full textual-rs API guide (widgets, layout, events, testing, workers, themes)

### Existing codebase (read before planning)
- `src/model/app_state.rs` — Screen enum, AppState struct, ClickRegion (ClickRegion infrastructure being retired)
- `src/tui/mod.rs` — current render dispatch
- `src/tui/dashboard.rs` — most complete screen example (DashboardState, DashboardAction, handle_key, render)
- `src/tui/keys.rs` — most complex screen (sub-screens, popups, action enum)
- `src/app.rs` — current event loop (being replaced by textual-rs App runner)
- `tests/e2e/` — tmux harness being retired (understand scope before deleting)

### Prior phase context
- `.planning/phases/06-tech-debt-infrastructure/06-CONTEXT.md` — model/view split decisions
- `.planning/phases/07-mouse-support-e2e-test-harness/07-CONTEXT.md` — ClickRegionMap and tmux harness decisions (both superseded by this phase)

### Requirements
- `.planning/REQUIREMENTS.md` — INFRA-03, INFRA-04 (model/view boundary must be preserved through migration)

</canonical_refs>

<specifics>
## Specific References

- **yubioath-flutter UX problems to avoid**: no rule of thirds, eye has to search all over, regions not visually mouse-compliant, no visible shortcut keys. Our textual-rs migration directly addresses all four.
- **textual-rs sits on ratatui**: not a ratatui removal — it's a component abstraction layer. insta snapshots survive.
- **textual-rs version**: 0.2 — confirmed production-ready with full widget set, testing framework, and theme support.

</specifics>

<deferred>
## Deferred Ideas

- QR code scanning for OATH — no camera in a TUI, out of scope always
- Tauri GUI integration — future milestone, not v1.1
- ratatui 0.30 upgrade — separate concern, not part of migration
- Custom theme authoring — user can pick from built-ins; custom themes are v2

</deferred>

---

*Phase: 08-textual-rs-migration*
*Context gathered: 2026-03-27 via /gsd:discuss-phase 8*
