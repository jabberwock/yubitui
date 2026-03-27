# Phase 8: textual-rs Migration - Research

**Researched:** 2026-03-27
**Domain:** textual-rs 0.2, Rust TUI component migration, ratatui 0.30
**Confidence:** MEDIUM-HIGH (textual-rs API verified via official GitHub source; ratatui 0.30 breaking changes verified via official docs)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** textual-rs replaces raw ratatui widget composition. textual-rs sits on top of ratatui — same rendering engine, higher-level component model. Not a ratatui removal.
- **D-02:** All 7 existing screens migrate in this phase: Dashboard, Keys, Pin, SSH, Diagnostics, PIV, Help. No partial migration — all screens use textual-rs by phase end.
- **D-03:** `src/model/` is byte-for-byte unchanged by this migration. Zero model layer changes are permitted. If a migration task requires touching model code, that is a bug in the plan.
- **D-04:** All `#[derive(serde::Serialize)]` types, action enums, handle_key functions in `src/tui/` remain — they become the action layer that textual-rs `on_action()` calls dispatch to.
- **D-05:** The manual `ClickRegionMap` / `Vec<ClickRegion>` infrastructure is retired. textual-rs Button widgets are the click target primitive.
- **D-06:** Every previously keyboard-navigable element becomes a textual-rs Button or interactive widget. Rule-of-thirds layout via textual-rs flexbox/grid (TCSS).
- **D-07:** textual-rs Footer widget renders keybindings on-screen at all times. Every screen declares its bindings via `key_bindings()`.
- **D-08:** tmux E2E harness (`tests/e2e/` shell scripts + run_all.sh) is retired in this phase.
- **D-09:** All screen coverage replaced by textual-rs Pilot-based tests (`TestApp` + `pilot.press()` etc). These run inside `cargo test`.
- **D-10:** insta snapshot tests are kept. textual-rs renders to a ratatui Buffer underneath — existing snapshots remain valid.
- **D-11:** User can select a theme from textual-rs built-ins: tokyo-night, nord, gruvbox, dracula, catppuccin. Theme choice is persisted (config file or env var — Claude's discretion on mechanism).
- **D-12:** No default theme locked by user — Claude picks most neutral/readable default.
- **D-13:** Rule-of-thirds layout using textual-rs CSS grid/flex.
- **D-14:** Mouse regions must be visually obvious. Button widgets with borders/styling make click targets self-evident.
- **D-15:** Keyboard shortcuts are visible on every screen via Footer.

### Claude's Discretion

- Exact TCSS styling per screen (colors, padding, borders within theme)
- Whether Screen navigation uses textual-rs screen stack (push/pop_screen_deferred) or retains the flat Screen enum model
- Config mechanism for theme persistence (TOML file, env var, etc.)
- How the existing per-screen action enums integrate with textual-rs on_action() dispatch
- Order of screen migration within the phase (waves)

### Deferred Ideas (OUT OF SCOPE)

- QR code scanning for OATH
- Tauri GUI integration
- ratatui 0.30 upgrade (as a separate concern — BUT see critical dependency finding below)
- Custom theme authoring
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| INFRA-03 | App state is split into `src/model/` (zero ratatui imports) and `src/tui/` (all ratatui rendering) with no cross-contamination | D-03 locks this: zero model changes permitted. textual-rs sits on ratatui — all rendering stays in src/tui/. Migration must keep this boundary clean. |
</phase_requirements>

---

## Summary

Phase 8 migrates all 7 yubitui TUI screens from raw ratatui widget composition to textual-rs 0.2 components. The textual-rs crate (version 0.2.0, released 2026-03-26) provides a Python Textual-inspired component model built on top of ratatui — it is an abstraction layer, not a ratatui replacement. All rendering still goes through ratatui's `Buffer`, which means insta snapshot tests survive unchanged.

**Critical dependency finding:** textual-rs 0.2 requires `ratatui = "0.30.0"`. The project currently uses `ratatui = "0.29"`. Upgrading to ratatui 0.30 was listed as deferred in CONTEXT.md, but it is structurally unavoidable for this migration — textual-rs 0.2 will not compile against ratatui 0.29. Wave 0 of this phase must include the ratatui 0.29 → 0.30 upgrade. ratatui 0.30 has a MSRV of 1.86; the installed Rust toolchain is 1.92.0, so this is satisfied. textual-rs itself requires MSRV 1.88, also satisfied.

The existing `App` struct in `src/app.rs` (crossterm event loop, `Terminal`, mouse capture, screen dispatch) is replaced by `textual_rs::App::new(factory).run()`. The per-screen `*State`, `*Action` enums, and `handle_key()` functions are preserved — they become the data model that textual-rs `on_action()` callbacks call into. The `ClickRegion` / `ClickRegionMap` infrastructure in `src/model/click_region.rs` can be deleted; textual-rs Button widgets handle all click targets natively.

**Primary recommendation:** Start with Wave 0 (ratatui 0.30 upgrade + add textual-rs dependency + delete tmux harness), then migrate screens from simplest to most complex: Help → Diagnostics → SSH → PIV → Pin → Dashboard → Keys. Each screen migration is an independent PR-sized task.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| textual-rs | 0.2.0 | TUI component framework (the migration target) | Only framework meeting all D-01 through D-15 requirements |
| ratatui | 0.30.0 | Underlying renderer (upgraded from 0.29) | textual-rs 0.2 requires exactly this version |
| crossterm | 0.29.0 | Terminal backend | textual-rs 0.2 specifies crossterm 0.29 via `crossterm_0_29` feature flag |
| insta | 1.47 (existing) | Snapshot regression tests | D-10: kept as-is; textual-rs renders to ratatui Buffer so existing snapshots valid |
| toml | 0.8 (existing) | Theme config persistence | Already a dependency; use for theme config file |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| taffy | 0.9.2 (transitive) | Flexbox layout engine | Used internally by textual-rs for TCSS grid/flex — no direct usage needed |
| dirs | 5.0 (existing) | Config file location | Needed for persisting theme config to ~/.config/yubitui/config.toml |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| TOML config for theme | env var `YUBITUI_THEME` | Env var simpler but doesn't persist across sessions without shell profile edit; TOML is user-editable and matches existing serde/toml dep |
| Screen stack (push/pop_screen_deferred) | Flat Screen enum retained | Screen stack enables proper modal overlay for popups (key gen wizard, pin dialogs); flat enum requires manual z-ordering. Both work — discretion area. |

**Installation (Cargo.toml changes required):**
```toml
# Replace:
ratatui = "0.29"
crossterm = { version = "0.28", features = ["event-stream"] }

# With:
ratatui = { version = "0.30.0", features = ["crossterm_0_29"] }
crossterm = { version = "0.29.0", features = ["event-stream"] }
textual-rs = "0.2"

# Update rust-version:
rust-version = "1.88"  # textual-rs MSRV (Rust 1.92 installed, so satisfied)
```

**Version verification:** textual-rs 0.2.0 confirmed from GitHub CHANGELOG.md (released 2026-03-26). ratatui 0.30.0 confirmed from official ratatui.rs docs.

---

## Architecture Patterns

### Recommended Project Structure

```
src/
├── model/           # UNTOUCHED — all state, zero ratatui imports
├── tui/
│   ├── mod.rs       # Remove From<Rect> impl (ClickRegion gone); keep nothing or thin shim
│   ├── app.rs       # NEW: textual-rs App builder (replaces src/app.rs)
│   ├── theme.rs     # NEW: theme loading from config
│   ├── config.rs    # NEW: theme persistence (read/write ~/.config/yubitui/config.toml)
│   ├── dashboard.rs # MIGRATED: DashboardScreen widget + DashboardState (kept) + DashboardAction (kept)
│   ├── diagnostics.rs # MIGRATED
│   ├── help.rs      # MIGRATED
│   ├── keys.rs      # MIGRATED (most complex — 2023 lines)
│   ├── pin.rs       # MIGRATED
│   ├── piv.rs       # MIGRATED
│   ├── ssh.rs       # MIGRATED
│   ├── widgets/     # Keep custom sub-widgets (pin_input, popup, progress) — port to textual-rs Widget trait
│   └── snapshots/   # KEPT — insta snapshots still valid
└── app.rs           # REPLACED — old crossterm event loop deleted; new main entry calls tui::app::run()
tests/
└── e2e/             # DELETED — all 7 *.sh smoke tests + run_all.sh retired (D-08)
```

### Pattern 1: Screen Widget with Reactive State

Every screen becomes a struct implementing the textual-rs `Widget` trait. Existing `*State` structs become `Reactive<*State>` fields.

**What:** Widget struct wraps existing `*State` in `Reactive<T>`, composes child widgets in `compose()`, declares bindings in `key_bindings()`, dispatches to existing `handle_key()` equivalent in `on_action()`.

**When to use:** All 7 screen migrations follow this pattern.

**Example (based on verified textual-rs guide API):**
```rust
// Source: https://raw.githubusercontent.com/jabberwock/textual-rs/master/docs/guide.md
use textual_rs::{App, Widget, Footer, Header, Button};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
use textual_rs::reactive::Reactive;
use crossterm::event::{KeyCode, KeyModifiers};

struct DiagnosticsScreen {
    state: Reactive<DiagnosticsTuiState>,  // existing struct, now reactive
    app_state: Reactive<AppState>,          // passed in, read-only
}

impl Widget for DiagnosticsScreen {
    fn widget_type_name(&self) -> &'static str { "DiagnosticsScreen" }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        vec![
            Box::new(Header::new("Diagnostics")),
            // ... content widgets built from self.app_state.get()
            Box::new(Footer),  // D-07: Footer always present
        ]
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        &[
            KeyBinding {
                key: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                action: "back",
                description: "Back to Dashboard",
                show: true,
            },
        ]
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "back" => ctx.pop_screen_deferred(),
            _ => {}
        }
    }
}
```

### Pattern 2: Action String Mapping (bridging existing action enums)

textual-rs `on_action` uses string-based actions (not typed enums). The existing `DashboardAction`, `PinAction`, etc. enums are preserved (D-04) and called from `on_action` string dispatch.

**What:** `on_action(&str)` matches action strings declared in `key_bindings()`, then instantiates the typed action enum and dispatches to the existing `handle_key()` equivalent logic.

**Why:** Preserves all existing logic while adopting textual-rs event model. Action enums stay for Tauri serialization (D-04 / INFRA-06).

```rust
fn on_action(&self, action: &str, ctx: &AppContext) {
    // Bridge from textual-rs string actions to existing typed enums
    let typed_action = match action {
        "navigate_keys" => DashboardAction::NavigateTo(Screen::Keys),
        "refresh"       => DashboardAction::Refresh,
        "quit"          => DashboardAction::Quit,
        _               => DashboardAction::None,
    };
    self.dispatch_action(typed_action, ctx);
}
```

### Pattern 3: Theme Loading from Config

**What:** On startup, read `~/.config/yubitui/config.toml`. Extract `theme` string. Call `theme_by_name()`. Fall back to `default_dark_theme()` if missing or invalid. Persist changes when user cycles via Ctrl+T (or new keybinding).

```rust
// Source: verified from textual-rs css/theme.rs
use textual_rs::css::theme::{theme_by_name, default_dark_theme};

fn load_theme_from_config() -> textual_rs::css::theme::Theme {
    // Read ~/.config/yubitui/config.toml
    let config_path = dirs::config_dir()
        .map(|p| p.join("yubitui").join("config.toml"));
    let theme_name = config_path
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| toml::from_str::<toml::Value>(&s).ok())
        .and_then(|v| v.get("theme")?.as_str().map(String::from));
    theme_name
        .and_then(|name| theme_by_name(&name))
        .unwrap_or_else(default_dark_theme)
}

// In main:
let mut app = App::new(|| Box::new(RootScreen::new(app_state)));
app.set_theme(load_theme_from_config());
app.run()?;
```

### Pattern 4: insta Snapshot Test Compatibility

textual-rs renders to a ratatui `Buffer` via `TestBackend`. The `test_app.backend()` accessor returns the `TestBackend`. Existing insta snapshot patterns using `assert_display_snapshot!` continue to work.

```rust
// Pattern confirmed from textual-rs testing/mod.rs
#[tokio::test]
async fn dashboard_default_populated() {
    let mock_state = crate::model::mock::mock_yubikey_states();
    let mut test_app = TestApp::new(80, 24, || {
        Box::new(DashboardScreen::new(mock_state))
    });
    test_app.pilot().settle().await;
    insta::assert_display_snapshot!(test_app.backend());
}
```

**Warning:** Snapshot content will change because the layout changes (rule-of-thirds replaces current layout). All 15 existing snapshots in `src/tui/snapshots/` need to be re-accepted after migration. Use `cargo insta review` or `INSTA_UPDATE=always cargo test`.

### Pattern 5: Pilot-based Tests (replacing tmux E2E)

```rust
// Source: https://raw.githubusercontent.com/jabberwock/textual-rs/master/docs/guide.md
#[tokio::test]
async fn pin_change_user_pin_navigation() {
    let mut app = TestApp::new(80, 24, || Box::new(PinScreen::new(None)));
    let mut pilot = app.pilot();
    pilot.press(KeyCode::Char('c')).await;  // navigate to ChangeUserPin sub-screen
    pilot.settle().await;
    // assert buffer contains "Change User PIN" text
    let buf = app.buffer();
    assert!(format!("{:?}", buf).contains("Change User PIN"));
}
```

### Anti-Patterns to Avoid

- **Importing ratatui directly in screen widgets:** textual-rs wraps ratatui — direct `use ratatui::widgets::*` in screen widget structs couples you to raw ratatui and bypasses the component model. Only use ratatui types in `render()` method body.
- **Retaining ClickRegion registration in render():** The whole `click_regions.push(...)` pattern is retired. D-05 is absolute. Any remaining manual click region math is a bug.
- **Calling `handle_key()` directly from `on_action()`:** The old `handle_key(KeyEvent)` signature takes a crossterm KeyEvent — it doesn't map cleanly to textual-rs actions. Instead, extract the logic into a method callable from both the old signature (for testing legacy paths) and the new `on_action()` dispatch.
- **Keeping crossterm event loop in app.rs:** The old `App::run()` / `event_loop()` / `handle_events()` structure is fully replaced by `textual_rs::App::new(...).run()`. Do not try to retain both.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Click target detection | Custom ClickRegion math per frame | textual-rs Button widget | Button handles hit-testing, focus, mouse hover, keyboard activation — ClickRegion misses all of these |
| Keyboard shortcut display | Manual status bar text concatenation | textual-rs Footer widget | Footer auto-renders from key_bindings() declarations — no string formatting needed |
| Flexbox/grid layout | Manual ratatui Layout::default() calculations | TCSS via textual-rs | Taffy handles shrink/grow, fractional units, responsive resize — ratatui constraints don't |
| Theme variable resolution | Hardcoded Color:: values | TCSS `$primary`, `$surface` etc. | Theme variables change with theme — hardcoded colors break theme switching |
| Test event dispatch | tmux send-keys + timing loops | Pilot::press() / settle() | Pilot is synchronous, deterministic, no timing races, runs in cargo test without tmux |
| Config file location | Hardcoded path | dirs::config_dir() | Cross-platform: ~/.config/yubitui on Linux/macOS, %APPDATA%\yubitui on Windows |

**Key insight:** textual-rs eliminates the three most brittle parts of the current codebase: manual click math, manual shortcut display, and tmux timing-sensitive tests.

---

## Critical Dependency Issue

### ratatui 0.29 vs 0.30

**Finding (HIGH confidence — verified from textual-rs Cargo.toml):**

textual-rs 0.2 specifies `ratatui = { version = "0.30.0", features = ["crossterm_0_29"] }`. The project currently declares `ratatui = "0.29"`. These are semver-incompatible. Adding `textual-rs = "0.2"` to Cargo.toml without upgrading ratatui will cause a dependency resolution failure.

**Impact:** Wave 0 of this phase MUST include the ratatui 0.29 → 0.30 upgrade. This was listed as "deferred" in CONTEXT.md as a separate concern, but it is structurally unavoidable here.

**Migration cost (MEDIUM confidence — from ratatui.rs/highlights/v030/):**

ratatui 0.30 breaking changes that affect yubitui:
1. `Alignment` renamed to `HorizontalAlignment` — affects any `Alignment::Left/Center/Right` usage
2. `Block::title()` now accepts `Into<Line>` not `Into<Title>` — affects Block title calls
3. `widgets::block` module no longer exported — any `use ratatui::widgets::block::*` breaks
4. `List::highlight_symbol` now accepts `Into<Line>` — affects list highlight usage

Application developers (vs widget authors) can continue using the `ratatui` crate — the core API is stable, only specific widget/alignment APIs change. Migration is mechanical search-and-replace, not architectural.

**MSRV:** ratatui 0.30 requires Rust 1.86. textual-rs 0.2 requires Rust 1.88. Installed toolchain: Rust 1.92.0. Satisfied. However, `Cargo.toml` `rust-version = "1.75"` needs updating to `"1.88"`.

---

## Common Pitfalls

### Pitfall 1: Snapshot Invalidation on Migration

**What goes wrong:** Every migrated screen produces different snapshot output because layout changes (rule-of-thirds vs current flat layout). If the planner scopes "keep insta snapshots" as zero-effort, the first migrated screen will cause all tests to fail.

**Why it happens:** D-10 says "insta snapshots survive" — true in mechanism (textual-rs renders to ratatui Buffer), but NOT in content (new layout = new visual output).

**How to avoid:** Each screen migration task explicitly includes a step to run `cargo insta review` and accept the new snapshots for that screen. Treat snapshot re-acceptance as part of done-definition per screen.

**Warning signs:** `cargo test` failing with "snapshot mismatch" after any screen migration.

### Pitfall 2: Reactive<T> Borrow Conflicts

**What goes wrong:** `Reactive<T>` uses `.get()` for tracked reads inside `compose()` and `render()`. Calling `.get_untracked()` inside reactive closures or calling `.set()` inside `render()` causes panics or infinite re-render loops.

**Why it happens:** textual-rs reactive system tracks reads to know what to re-render. Writing to a reactive during a read causes a cycle.

**How to avoid:** Use `.get()` in `compose()` and `render()` for tracked reads. Use `.get_untracked()` when you need a value without subscribing. Only call `.set()` or `.update()` inside `on_action()`, `on_mount()`, or async workers — never in `render()`.

**Warning signs:** Stack overflow from infinite re-render, or panics mentioning "reactive borrow already borrowed."

### Pitfall 3: crossterm Version Mismatch

**What goes wrong:** textual-rs 0.2 uses crossterm 0.29 (via the `crossterm_0_29` feature flag on ratatui 0.30). The project currently uses crossterm 0.28. If Cargo.toml is updated to `crossterm = "0.29"`, direct crossterm usages in remaining code (e.g., `EnableMouseCapture`, `KeyEvent`, `KeyCode`) stay compatible since the API surface is stable between 0.28 and 0.29.

**Why it happens:** Cargo requires compatible feature sets. ratatui 0.30 with `crossterm_0_29` feature + a direct crossterm 0.28 dependency = two incompatible crossterm versions in the dep graph.

**How to avoid:** Update `crossterm = "0.29"` in Cargo.toml simultaneously with the ratatui upgrade in Wave 0.

**Warning signs:** Cargo dependency resolution error mentioning crossterm version conflict.

### Pitfall 4: Old App::run() and New textual-rs App::run() Coexisting

**What goes wrong:** During migration, if `src/app.rs` `App::run()` and the new textual-rs app runner both exist, `main.rs` must call exactly one. Having dead code in `app.rs` that still compiles but isn't called is confusing but safe. Having it partially called causes double terminal initialization.

**Why it happens:** Incremental migration leaves old code in place.

**How to avoid:** In Wave 1 (first screen migration), delete the old `App` struct and `event_loop()` entirely. Replace `src/app.rs` with a thin `run(mock: bool) -> Result<()>` that calls the textual-rs App. All subsequent screen migrations work against the new runner.

**Warning signs:** Terminal left in raw mode after crash, alternate screen not restored.

### Pitfall 5: handle_key() vs on_action() Mismatch

**What goes wrong:** Existing `handle_key(state, key: KeyEvent) -> *Action` functions take a crossterm `KeyEvent`. textual-rs `on_action(&str)` takes a string. Naively trying to pass a constructed `KeyEvent` from within `on_action()` adds unnecessary complexity.

**Why it happens:** The two event models are different. textual-rs handles key dispatch internally — it calls `on_action("increment")` not `on_action_key(KeyEvent { code: KeyCode::Char('+'), ... })`.

**How to avoid:** Do not port `handle_key()` 1:1 into `on_action()`. Instead, extract the state mutation logic from `handle_key()` into named methods on the `*State` struct (e.g., `DashboardState::open_context_menu()`), then call those from `on_action()`. The old `handle_key()` can be deleted or kept as dead code for reference during migration.

---

## Code Examples

Verified patterns from official sources:

### App Runner (main.rs replacement)
```rust
// Source: https://raw.githubusercontent.com/jabberwock/textual-rs/master/docs/guide.md
use textual_rs::App;
use textual_rs::css::theme::{theme_by_name, default_dark_theme};

pub fn run(mock: bool) -> anyhow::Result<()> {
    let app_state = build_initial_state(mock)?;  // existing model logic
    let theme = load_theme_from_config();

    let mut app = App::new(move || {
        Box::new(crate::tui::dashboard::DashboardScreen::new(app_state))
    });
    app.set_theme(theme);
    app.run()?;
    Ok(())
}
```

### Theme Persistence
```rust
// Source: textual-rs css/theme.rs (theme_by_name verified)
use textual_rs::css::theme::{builtin_themes, theme_by_name, default_dark_theme};

fn load_theme(name: Option<&str>) -> textual_rs::css::theme::Theme {
    name.and_then(|n| theme_by_name(n))
        .unwrap_or_else(default_dark_theme)
}

fn save_theme(name: &str) -> anyhow::Result<()> {
    let path = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("no config dir"))?
        .join("yubitui")
        .join("config.toml");
    std::fs::create_dir_all(path.parent().unwrap())?;
    let content = format!("theme = \"{}\"\n", name);
    std::fs::write(path, content)?;
    Ok(())
}
```

### TestApp + Pilot (replacing tmux)
```rust
// Source: https://raw.githubusercontent.com/jabberwock/textual-rs/master/docs/guide.md
#[tokio::test]
async fn help_screen_renders() {
    let mut app = TestApp::new(80, 24, || Box::new(HelpScreen::new()));
    app.pilot().settle().await;
    insta::assert_display_snapshot!(app.backend());
}

#[tokio::test]
async fn dashboard_esc_navigates_to_keys() {
    let mut app = TestApp::new(80, 24, || {
        Box::new(DashboardScreen::new(mock_yubikey_states()))
    });
    let mut pilot = app.pilot();
    pilot.press(KeyCode::Char('1')).await;  // navigate to Keys
    pilot.settle().await;
    // assert current screen via buffer content
}
```

### Screen Stack Navigation
```rust
// Source: textual-rs docs/guide.md — push/pop_screen_deferred
fn on_action(&self, action: &str, ctx: &AppContext) {
    match action {
        "open_keys" => ctx.push_screen_deferred(
            Box::new(KeysScreen::new(self.yubikey_state.get()))
        ),
        "back" => ctx.pop_screen_deferred(),
        _ => {}
    }
}
```

### TCSS Rule-of-Thirds Layout
```css
/* Screen layout: header + main content area + footer */
DashboardScreen {
    layout-direction: vertical;
}

#content {
    flex-grow: 1;
    layout-direction: horizontal;
}

#sidebar {
    width: 33%;  /* one-third */
    min-width: 20;
}

#main {
    flex-grow: 1;  /* two-thirds */
}

Button {
    border: inner;
    min-width: 16;
    height: 3;
}

Button:focus {
    border: tall $accent;
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual click region math (ClickRegion, Vec<ClickRegion>) | textual-rs Button widget | This phase | Delete ~200 lines of hit-test math across all screens |
| crossterm event loop in app.rs | textual-rs App::run() | This phase | Delete event_loop(), handle_events(), handle_key_event(), apply_mouse_capture() |
| tmux send-keys E2E tests | textual-rs Pilot::press() in cargo test | This phase | No tmux dependency, no timing races, CI-friendly |
| Manual status bar with shortcut hints | textual-rs Footer (auto-renders from key_bindings()) | This phase | Shortcuts visible on every screen without string formatting |
| ratatui 0.29 direct widget composition | textual-rs 0.2 component model on ratatui 0.30 | This phase | Unavoidable ratatui upgrade bundled with migration |

**Deprecated/outdated:**
- `src/app.rs` App struct: replaced by textual-rs App runner
- `src/model/click_region.rs` ClickRegion/ClickRegionMap/ClickAction: retired (D-05)
- `tests/e2e/*.sh` all 7 smoke scripts + run_all.sh + helpers.sh: deleted (D-08)
- `src/tui/mod.rs` From<Rect> for Region impl: no longer needed once ClickRegion is gone

---

## Open Questions

1. **Screen navigation model: screen stack vs flat enum**
   - What we know: textual-rs supports both `push_screen_deferred` / `pop_screen_deferred` (stack) and keeping a flat navigation model
   - What's unclear: The existing `Screen` enum in `src/model/app_state.rs` is model-layer state (Tauri-serializable). If we use the screen stack, the model-layer `Screen` enum becomes redundant — but D-03 forbids touching model.
   - Recommendation: Retain the flat `Screen` enum in the model as the source of truth. Implement screen transitions by having the root widget react to `AppState.current_screen` changes. This avoids touching model code and keeps Tauri serialization intact.

2. **Theme cycling at runtime**
   - What we know: Ctrl+T cycles themes in the built-in theme list. The app needs to persist the selected theme name to config.
   - What's unclear: Does textual-rs fire a named action for Ctrl+T, or does it handle it internally? If internal, there's no hook to call `save_theme()`.
   - Recommendation: Override the Ctrl+T binding in the root screen's `key_bindings()` as `"cycle_theme"`, handle in `on_action()`, call `set_theme()` on the app context, and call `save_theme()`. Verify during Wave 1 implementation.

3. **Keys screen complexity (2023 lines)**
   - What we know: Keys is by far the largest screen, with multi-step wizards (KeyGenWizard with 7 steps), sub-screens, popups, and complex state.
   - What's unclear: How well do multi-step wizard flows map to textual-rs screen stack vs internal state machine.
   - Recommendation: Implement Keys last (Wave N). Each wizard step becomes either a pushed screen or a reactive state change within the KeysScreen widget. Evaluate during wave planning.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | All compilation | Yes | 1.92.0 | — |
| Cargo | Package management | Yes | 1.92.0 | — |
| textual-rs 0.2 | Core migration | Yes (GitHub + crates.io) | 0.2.0 | — |
| ratatui 0.30 | textual-rs dep | Yes (crates.io) | 0.30.0 | — |
| insta 1.47 | Snapshot tests | Yes (existing dep) | 1.47 | — |
| tmux | E2E harness (being deleted) | N/A | N/A | Retired |

**Missing dependencies with no fallback:** None — all required tools available.

**Missing dependencies with fallback:** None.

---

## Sources

### Primary (HIGH confidence)
- `https://raw.githubusercontent.com/jabberwock/textual-rs/master/docs/guide.md` — Widget trait, App runner, TestApp/Pilot API, TCSS layout, Footer/Button constructors, screen stack API
- `https://raw.githubusercontent.com/jabberwock/textual-rs/master/crates/textual-rs/src/app.rs` — App builder methods: with_css, with_css_file, set_theme
- `https://raw.githubusercontent.com/jabberwock/textual-rs/master/crates/textual-rs/src/css/theme.rs` — theme_by_name(), builtin_themes(), all 7 theme constructors
- `https://raw.githubusercontent.com/jabberwock/textual-rs/master/crates/textual-rs/src/testing/mod.rs` — TestApp and Pilot method signatures
- `https://github.com/jabberwock/textual-rs/blob/master/CHANGELOG.md` — version 0.2.0 confirmed, released 2026-03-26
- `https://github.com/jabberwock/textual-rs/blob/master/crates/textual-rs/Cargo.toml` — ratatui 0.30.0 + crossterm 0.29 dependency confirmed
- `https://ratatui.rs/highlights/v030/` — ratatui 0.30 MSRV (1.86), breaking changes documented

### Secondary (MEDIUM confidence)
- `https://github.com/jabberwock/textual-rs/blob/master/crates/textual-rs/src/lib.rs` — exported modules and widget inventory confirmed
- ratatui 0.30 breaking changes (Alignment rename, Block::title signature, list highlight) — from official ratatui changelog

### Tertiary (LOW confidence)
- None — all critical claims verified from official sources.

---

## Metadata

**Confidence breakdown:**
- Standard stack (library versions): HIGH — verified from Cargo.toml and CHANGELOG.md in official repo
- Architecture (Widget trait, App runner, testing API): HIGH — verified from source code and official guide
- Pitfalls (reactive loops, crossterm version, snapshot invalidation): MEDIUM — derived from API analysis; some may surface only at implementation time
- ratatui 0.30 breaking change impact on yubitui: MEDIUM — breaking changes listed but yubitui codebase not fully audited for every affected callsite

**Research date:** 2026-03-27
**Valid until:** 2026-04-27 (textual-rs is actively developed; API stable within 0.2.x series)
