# Architecture Research

**Domain:** Rust TUI (ratatui) with Model/View separation for TUI+Tauri dual-frontend
**Researched:** 2026-03-26
**Confidence:** HIGH for patterns; MEDIUM for FIDO2/OTP APDU specifics

---

## Standard Architecture

### System Overview

The target architecture for v1.1 is a three-layer system. The middle layer (`AppModel`) is the load-bearing change — it must contain zero ratatui imports so it can be consumed by both the TUI and a future Tauri backend.

```
┌─────────────────────────────────────────────────────────────────┐
│                      PRESENTATION LAYER                          │
│                                                                  │
│  ┌──────────────────────┐        ┌──────────────────────────┐   │
│  │   TUI Frontend        │        │  Tauri Frontend (future) │   │
│  │   src/tui/            │        │  tauri-app/ (future)     │   │
│  │   (ratatui imports OK)│        │  (webview, no ratatui)   │   │
│  └──────────┬───────────┘        └──────────┬───────────────┘   │
│             │ reads/mutates                  │ reads/calls       │
├─────────────┴────────────────────────────────┴───────────────────┤
│                      MODEL LAYER (NO ratatui)                    │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                   AppModel / AppState                    │    │
│  │  current_screen, yubikey_states, pin_state, key_state... │    │
│  │  + ScreenModel per screen (PinModel, KeysModel, etc.)    │    │
│  └──────────────────────────┬──────────────────────────────┘    │
│                             │ calls                              │
├─────────────────────────────┴────────────────────────────────────┤
│                      SERVICE LAYER (NO ratatui)                  │
│                                                                  │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌─────────────┐  │
│  │ yubikey/   │ │ yubikey/   │ │ yubikey/   │ │ yubikey/    │  │
│  │ openpgp.rs │ │ pin_ops.rs │ │ oath.rs    │ │ fido2.rs    │  │
│  │            │ │            │ │ (new)      │ │ (new)       │  │
│  └────────────┘ └────────────┘ └────────────┘ └─────────────┘  │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              card.rs — native PC/SC APDUs                │    │
│  └─────────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | ratatui dependency |
|-----------|---------------|-------------------|
| `src/tui/app.rs` | Event loop, terminal setup/teardown, draw calls | YES — owns Terminal |
| `src/tui/screens/` | Per-screen render functions + mouse region registration | YES — Frame, Rect, widgets |
| `src/tui/widgets/` | Reusable popup/input widgets | YES |
| `src/model/mod.rs` | AppModel struct, screen routing, business state | NONE |
| `src/model/screens/` | Per-screen state structs (PinModel, KeysModel, etc.) | NONE |
| `src/yubikey/` | PC/SC card operations, parsers, card.rs | NONE |

---

## Recommended Project Structure

```
src/
├── model/                  # Zero ratatui — Tauri-safe
│   ├── mod.rs              # AppModel struct, Screen enum, global state
│   ├── pin.rs              # PinModel (replaces ui::pin::PinState)
│   ├── keys.rs             # KeysModel (replaces ui::keys::KeyState)
│   ├── ssh.rs              # SshModel
│   ├── dashboard.rs        # DashboardModel
│   ├── oath.rs             # OathModel — NEW (TOTP/HOTP screen state)
│   ├── fido2.rs            # Fido2Model — NEW
│   ├── otp.rs              # OtpModel — NEW (OTP slots screen state)
│   └── onboarding.rs       # OnboardingModel — NEW
│
├── tui/                    # ratatui-specific, imports OK
│   ├── app.rs              # Event loop, Terminal, owns AppModel
│   ├── screens/
│   │   ├── mod.rs          # render dispatch (match model.current_screen)
│   │   ├── dashboard.rs
│   │   ├── keys.rs
│   │   ├── pin.rs
│   │   ├── piv.rs
│   │   ├── ssh.rs
│   │   ├── diagnostics.rs
│   │   ├── help.rs
│   │   ├── oath.rs         # NEW
│   │   ├── fido2.rs        # NEW
│   │   ├── otp.rs          # NEW
│   │   └── onboarding.rs   # NEW
│   ├── widgets/
│   │   ├── pin_input.rs    # (existing)
│   │   ├── popup.rs        # (existing)
│   │   ├── progress.rs     # (existing)
│   │   └── help_panel.rs   # NEW — inline contextual help
│   └── mouse.rs            # ClickRegionMap, hit testing
│
├── yubikey/                # (existing — no changes to boundaries)
│   ├── card.rs
│   ├── openpgp.rs
│   ├── oath.rs             # NEW — YKOATH APDU layer
│   ├── fido2.rs            # NEW — FIDO2 info reads
│   ├── otp.rs              # NEW — OTP slot status reads
│   └── ... (existing modules)
│
├── diagnostics.rs
└── main.rs
```

### Structure Rationale

- **`src/model/`:** Contains all application state with no ratatui import. This is the shared contract between TUI and future Tauri. Structs here must be `Clone`, `Debug`, and ideally `serde::Serialize` for Tauri IPC.
- **`src/tui/`:** The entire ratatui surface. If ratatui is ever swapped (e.g., for `cursive` or `crosscurses`), only this directory changes.
- **`src/yubikey/`:** Unchanged boundary — pure PC/SC business logic. New modules `oath.rs`, `fido2.rs`, `otp.rs` added here following the existing pattern.

---

## Architectural Patterns

### Pattern 1: AppModel Extract (Model/View Split)

**What:** Pull all state fields out of `App` into a separate `AppModel` struct that has zero ratatui imports. The TUI's `App` struct owns `AppModel` and passes `&AppModel` (or individual screen models) to render functions.

**When to use:** This is the primary refactor for v1.1. Do this before adding new screens.

**Trade-offs:** Adds one indirection layer. The existing `ui::dashboard::render(frame, area, app, state)` signatures change to `render(frame, area, model)` — app is no longer passed. This is a clean improvement.

**Example:**

```rust
// src/model/mod.rs — NO ratatui imports
#[derive(Debug, Clone)]
pub struct AppModel {
    pub current_screen: Screen,
    pub previous_screen: Screen,
    pub yubikey_states: Vec<YubiKeyState>,
    pub selected_yubikey_idx: usize,
    pub diagnostics: Diagnostics,
    // Per-screen models — only the active one is used at a time
    pub pin: PinModel,
    pub keys: KeysModel,
    pub ssh: SshModel,
    pub dashboard: DashboardModel,
    pub oath: OathModel,        // new
    pub fido2: Fido2Model,      // new
    pub otp: OtpModel,          // new
}

// src/tui/app.rs — ratatui imports fine here
pub struct App {
    model: AppModel,
    // terminal handle lives here, not in model
}

impl App {
    fn render(&self, frame: &mut ratatui::Frame) {
        screens::render(frame, frame.area(), &self.model);
    }
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        // mutates self.model only — no ratatui types touched
        model::update::handle_key(&mut self.model, key)
    }
}
```

**Tauri readiness:** `AppModel` fields can be exposed as Tauri commands once `serde::Serialize` is derived. The Tauri backend calls the same `yubikey::` service functions. No shared runtime between TUI and Tauri is needed — they're separate binaries.

### Pattern 2: Per-Screen Event Delegation

**What:** Rather than one giant `handle_key_event` match in `app.rs` (currently 1,617 lines), each screen model gets its own `handle_key(key: KeyCode) -> ScreenAction` function. `app.rs` dispatches to it.

**When to use:** Immediately after the AppModel extract. Prerequisite for adding new screens without further bloating app.rs.

**Trade-offs:** Requires defining a `ScreenAction` enum per screen (or a shared `Action` enum). Slight overhead vs. inline match. The benefit is that adding a new screen means adding a new module, not modifying a 1600-line file.

**Example:**

```rust
// src/model/keys.rs
pub enum KeysAction {
    Navigate(KeyScreen),
    SetMessage(String),
    ClearMessage,
    RefreshState,
    Quit,
}

pub fn handle_key(model: &mut KeysModel, key: KeyCode) -> KeysAction {
    match model.screen {
        KeyScreen::Main => handle_main_key(model, key),
        KeyScreen::ViewStatus => handle_view_status_key(model, key),
        // ...
    }
}

// src/tui/app.rs — dispatch only
fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
    match self.model.current_screen {
        Screen::Keys => {
            let action = model::keys::handle_key(&mut self.model.keys, key.code);
            self.apply_keys_action(action)?;
        }
        // ...
    }
    Ok(())
}
```

### Pattern 3: Mouse Click Region Map

**What:** During each render pass, widgets that need click handling write their `Rect` into a `ClickRegionMap` stored in `AppModel` (or a separate `MouseState`). The mouse event handler looks up `(col, row)` against stored `Rect`s to resolve what was clicked.

**When to use:** Required for full mouse support. There is no built-in hit testing in ratatui — widget areas are only known at render time, so they must be captured during render and stored for the subsequent event handling cycle.

**Trade-offs:** One render cycle of lag between render and click handling (negligible at 100ms poll). Requires that render functions accept a `&mut ClickRegionMap` to register regions. The alternative library `ratatui-interact` handles this automatically but adds a dependency.

**Example:**

```rust
// src/tui/mouse.rs
#[derive(Default)]
pub struct ClickRegionMap {
    regions: Vec<(ClickTarget, Rect)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClickTarget {
    MenuItemIndex(usize),
    NavButton(Screen),
    ButtonConfirm,
    ButtonCancel,
    ListItem(usize),
    TabNext,
    TabPrev,
}

impl ClickRegionMap {
    pub fn register(&mut self, target: ClickTarget, area: Rect) {
        self.regions.push((target, area));
    }

    pub fn hit_test(&self, col: u16, row: u16) -> Option<&ClickTarget> {
        self.regions.iter()
            .find(|(_, rect)| rect.contains(Position { x: col, y: row }))
            .map(|(target, _)| target)
    }

    pub fn clear(&mut self) {
        self.regions.clear();
    }
}

// Usage in render function:
pub fn render(frame: &mut Frame, area: Rect, model: &DashboardModel, regions: &mut ClickRegionMap) {
    // ... render menu items
    for (i, item) in menu_items.iter().enumerate() {
        let item_rect = menu_area; // computed from layout
        regions.register(ClickTarget::MenuItemIndex(i), item_rect);
        frame.render_widget(item_widget, item_rect);
    }
}

// In app.rs mouse handler:
fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<()> {
    if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
        if let Some(target) = self.click_regions.hit_test(mouse.column, mouse.row) {
            match target.clone() {
                ClickTarget::MenuItemIndex(i) => { /* activate menu item */ }
                ClickTarget::NavButton(screen) => { self.model.current_screen = screen; }
                _ => {}
            }
        }
    }
    Ok(())
}
```

**Note:** `ClickRegionMap` must be cleared at the start of each render frame (not per-event), since regions are rebuilt every draw cycle.

### Pattern 4: Tmux-Based E2E Testing

**What:** Each test spawns the binary in a dedicated tmux pane, sends keystrokes via `tmux send-keys`, waits for render via `sleep` or polling, captures the pane via `tmux capture-pane -p`, and asserts on the captured text.

**When to use:** For integration tests that verify full screen flows — onboarding, navigation, error states — without hardware. Hardware-dependent operations (actual card reads) can be mocked by building a `--mock` flag into the binary.

**Trade-offs:** Requires `tmux` on the test machine (Linux/macOS only — Windows requires WSL). Tests have timing sensitivity around `sleep` durations. Start with longer sleeps (500ms) and tighten as the suite matures.

**Standard script structure:**

```bash
#!/usr/bin/env bash
# tests/e2e/test_navigation.sh

SESSION="yubitui_test_$$"
BINARY="./target/debug/yubitui"
PASS=0; FAIL=0

# Helpers
start_app() {
    tmux new-session -d -s "$SESSION" -x 220 -y 50
    tmux send-keys -t "$SESSION" "$BINARY --mock" Enter
    sleep 0.5  # wait for initial render
}

send_key() {
    tmux send-keys -t "$SESSION" "$1" ""
    sleep 0.2
}

capture() {
    tmux capture-pane -t "$SESSION" -p
}

assert_contains() {
    local text="$1"
    if capture | grep -qF "$text"; then
        echo "PASS: contains '$text'"
        ((PASS++))
    else
        echo "FAIL: expected '$text'"
        echo "--- actual output ---"
        capture
        ((FAIL++))
    fi
}

cleanup() {
    tmux kill-session -t "$SESSION" 2>/dev/null
}
trap cleanup EXIT

# Test: dashboard renders on startup
start_app
assert_contains "YubiTUI"
assert_contains "Dashboard"

# Test: navigate to PIN screen
send_key "2"
assert_contains "PIN Management"

# Test: escape returns to dashboard
send_key "Escape"
assert_contains "Dashboard"

echo "Results: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
```

**Mock mode pattern:** Add `--mock` CLI flag to main.rs that substitutes `YubiKeyState::detect_all()` with a fixed fixture. This lets E2E tests verify UI without hardware.

**Alternative:** Microsoft's `tui-test` (JavaScript/Node.js framework, uses pty not tmux) provides better cross-platform coverage and auto-wait logic. Viable if the team accepts a Node.js devDependency for testing. The tmux approach requires no extra dependencies and fits the existing Rust-only dev toolchain better.

---

## Data Flow

### Event Loop (current → target)

```
Current:
[crossterm Event] → App::handle_events() → App::handle_key_event() [1617 lines]
                                                   ↓
                                           mutates App fields directly

Target:
[crossterm Event] → tui::App::handle_events()
                           ↓
                    dispatch to model::update::handle_key(&mut self.model, key)
                           ↓
                    per-screen update fn → returns Action
                           ↓
                    tui::App applies Action (screen transitions, card I/O)
                           ↓
                    AppModel updated
                           ↓
                    terminal.draw() → screens::render(&model)
```

### Mouse Event Flow

```
[MouseEvent (col, row)] → tui::App::handle_mouse_event()
                               ↓
                         self.click_regions.hit_test(col, row)
                               ↓
                         Option<ClickTarget> → dispatch to model update
                               ↓
                         AppModel mutated → next render reflects change
```

### New Screen Addition Flow

```
1. Add variant to model::Screen enum
2. Add MyNewModel struct in src/model/my_new.rs (no ratatui)
3. Add field to AppModel: pub my_new: MyNewModel
4. Add handle_key() in src/model/my_new.rs
5. Add render() in src/tui/screens/my_new.rs (ratatui OK)
6. Wire into screens::render() match
7. Wire into tui::App key/mouse dispatch
8. Add nav keybind to status bar and help screen
```

---

## New Screen Architecture

### OATH/TOTP Screen

**Protocol:** YKOATH over CCID (ISO 7816-4). Accessible via PC/SC — same `card.rs` infrastructure as OpenPGP.

**AID:** `A0 00 00 05 27 21 01` (7 bytes)

**Key APDUs:**
- `SELECT` (INS `0xA4`, P1 `0x04`) with AID — connects to OATH application
- `LIST` (INS `0xA1`) — returns credential names + algorithm bytes (type `0x20`=TOTP, `0x10`=HOTP in high nibble; algo `0x01`=SHA1, `0x02`=SHA256 in low nibble)
- `CALCULATE` (INS `0xA2`, P2 `0x01` for truncated) with name tag `0x71` and challenge tag `0x74` — returns OTP code and digit count

**TOTP challenge construction:** Current Unix time divided by 30, encoded as big-endian 8-byte integer.

**What the screen shows:** List of stored OATH credentials (names, type TOTP/HOTP), current OTP value on selection, time remaining in 30-second window, credential count.

**What it cannot do (safely):** Add/delete credentials requires card mutation — phase these as follow-on work. v1.1 can be read-only: list + calculate.

**Confidence:** HIGH — Yubico publishes the full YKOATH specification at `developers.yubico.com/OATH/YKOATH_Protocol.html`.

### FIDO2 Screen

**Protocol:** CTAP 2.x over HID (keyboard interface), NOT CCID. This is the critical constraint.

**FIDO2 is NOT accessible via PC/SC on most systems.** The FIDO2 application uses a separate USB HID interface (USB usage page `0xF1D0`). The `pcsc` crate cannot reach it. Native FIDO2 access requires either:
1. `hidapi` crate — direct HID access (cross-platform, requires USB HID permissions on Linux)
2. `libfido2` via FFI — battle-tested C library from Yubico
3. Read-only workaround: FIDO2 GetInfo is accessible via CTAP over HID

**Practical approach for v1.1:** The FIDO2 screen shows static status only — whether FIDO2 is supported (from `YubiKeyInfo.model.supports_fido2()`), whether a PIN is set (requires HID access), resident key count (requires HID). If HID access is blocked, show "FIDO2 info requires libfido2 or hidapi — not available on this system."

**Do not route FIDO2 through PC/SC.** While a CTAPHID bridge over PC/SC exists (github.com/StarGate01/CTAP-bridge), it is a development tool, not a production pattern. The YubiKey FIDO2 documentation confirms CTAP uses HID natively.

**Confidence:** MEDIUM — confirmed FIDO2 uses HID not CCID from Yubico documentation, but exact HID APDU structure requires `hidapi` experimentation.

### OTP Slots Screen

**Protocol:** Routed through a single APDU over HID or CCID (the OTP application straddles both transports). The documentation states "almost all OTP commands are routed through a single APDU and dispatched based off of the first parameter in the payload."

**Status reads:** Slot occupancy is readable via the OTP status structure: major version (1B), minor version (1B), patch (1B), sequence number (1B per slot — non-zero = configured), touch level (2B).

**What the screen shows:** Slot 1 (short press) configured/empty, Slot 2 (long press) configured/empty, slot type if known (Yubico OTP / HOTP / static / challenge-response). Read-only status is achievable.

**Confidence:** MEDIUM — Yubico SDK docs describe the status structure; exact APDU for PC/SC read requires verification against card.rs patterns.

---

## Integration Points with Existing app.rs

### What Must Change (Modified)

| Current Location | Change | Risk |
|-----------------|--------|------|
| `App` struct fields | Move to `AppModel` in `src/model/mod.rs` | Medium — many call sites |
| `App::handle_key_event()` | Split into per-screen `handle_key()` fns | Medium — careful surgery |
| `App::handle_mouse_event()` | Rewrite to use `ClickRegionMap::hit_test()` | Low |
| `App::render()` | Pass `&self.model` instead of `self` | Low |
| `ui::dashboard::render(app, state)` | Change to `render(model)` — remove App ref | Low |
| `ui::ssh::render(app, state)` | Change to `render(model)` | Low |
| `Screen` enum | Move to `src/model/mod.rs` | Low |
| `src/ui/pin::PinState` | Rename/move to `src/model/pin::PinModel` | Low |
| `src/ui/keys::KeyState` | Rename/move to `src/model/keys::KeysModel` | Medium |
| `src/ui/ssh::SshState` | Rename/move to `src/model/ssh::SshModel` | Low |
| `src/ui/dashboard::DashboardState` | Rename/move to `src/model/dashboard::DashboardModel` | Low |

### What Stays (Unchanged)

| Component | Why Unchanged |
|-----------|--------------|
| `src/yubikey/card.rs` | Already has no ratatui imports |
| `src/yubikey/openpgp.rs` | Already boundary-clean |
| `src/yubikey/pin_operations.rs` | Already boundary-clean |
| `src/yubikey/key_operations.rs` | Already boundary-clean |
| `src/ui/widgets/pin_input.rs` | Stays in `tui/widgets/`, ratatui OK |
| `src/ui/widgets/popup.rs` | Stays in `tui/widgets/`, ratatui OK |
| BER-TLV parser, GET RESPONSE chaining | Core protocol infrastructure — no changes |

### What is New (Added)

| New Component | Purpose |
|--------------|---------|
| `src/model/mod.rs` | AppModel struct, Screen enum (extracted from app.rs) |
| `src/model/oath.rs` | OathModel state |
| `src/model/fido2.rs` | Fido2Model state |
| `src/model/otp.rs` | OtpModel state |
| `src/model/onboarding.rs` | OnboardingModel state |
| `src/tui/mouse.rs` | ClickRegionMap, ClickTarget, hit testing |
| `src/tui/screens/oath.rs` | TOTP/HOTP screen render |
| `src/tui/screens/fido2.rs` | FIDO2 info screen render |
| `src/tui/screens/otp.rs` | OTP slots screen render |
| `src/tui/widgets/help_panel.rs` | Inline contextual help sidebar |
| `src/yubikey/oath.rs` | YKOATH PC/SC APDU layer |
| `src/yubikey/fido2.rs` | FIDO2 status (stub or hidapi) |
| `src/yubikey/otp.rs` | OTP slot status reads |
| `tests/e2e/` | Tmux-based E2E test scripts |

---

## Build Order (Dependencies)

The Model/View split is a prerequisite for everything else. New screens added before the split will be harder to integrate and will violate the Tauri-ready constraint.

```
Phase 1: Model/View Split (prerequisite)
    1a. Create src/model/ with AppModel (extract App fields, no logic)
    1b. Move Screen enum to model::
    1c. Move per-screen state structs (PinState → PinModel, etc.)
    1d. Update render fn signatures to accept &AppModel not &App
    1e. Move key/mouse event logic into model::update fns
    → Validation: cargo test passes, TUI behavior identical

Phase 2: Mouse Support
    2a. Add src/tui/mouse.rs (ClickRegionMap)
    2b. Add regions.register() calls in render fns
    2c. Rewrite handle_mouse_event() to use hit_test()
    2d. Verify click-to-navigate, scroll, button interactions

Phase 3: Tmux E2E Suite Foundation
    3a. Add --mock flag to main.rs (fixture YubiKeyState)
    3b. Write tests/e2e/helpers.sh (start_app, send_key, assert_contains)
    3c. Write 5-10 navigation smoke tests
    → CI integration: add tmux E2E job (Linux only)

Phase 4: New Screens (can be parallelized after Phase 1)
    4a. OATH/TOTP — highest value, fully doable via PC/SC
    4b. OTP Slots — moderate complexity, OTP status readable via APDU
    4c. FIDO2 — start with static info from YubiKeyInfo, flag HID limitation

Phase 5: Contextual Help System
    5a. Add help_panel.rs widget
    5b. Wire per-screen help text into model
    5c. Context-sensitive: show help for active sub-screen
```

---

## Anti-Patterns

### Anti-Pattern 1: Adding New Screens Before the Model/View Split

**What people do:** Add OathModel fields directly into the existing `App` struct and `handle_key_event()` match arm to ship faster.

**Why it's wrong:** Increases the already-1617-line `app.rs`. Makes the future Tauri integration harder because `AppModel` still has ratatui types mixed in. Each new screen added this way increases refactor cost later.

**Do this instead:** Do the Model/View split in a single PR first (it is largely mechanical — move fields, update call sites). Then add new screens into the clean structure. The split will take 1-2 days but saves 3-4x that time later.

### Anti-Pattern 2: Passing `&App` to Render Functions

**What people do:** Keep the existing `render(frame, area, app, state)` signatures where `app: &App` provides access to all state including the ratatui terminal.

**Why it's wrong:** The `App` struct will own the `Terminal` (a ratatui type). Passing it to render functions creates a transitive ratatui dependency even if the render function only uses `app.yubikey_state()`. When Tauri needs to call the same view logic for WebSocket state updates, the ratatui dependency blocks it.

**Do this instead:** Render functions accept `&AppModel` (no ratatui import) or specific sub-models (`&DashboardModel`). The `Frame` is passed separately — it is already an explicit parameter.

### Anti-Pattern 3: Storing Rect Values in AppModel

**What people do:** To avoid ratatui imports in the model, they serialize `Rect` coordinates as raw `u16` fields in `AppModel`.

**Why it's wrong:** `Rect` is a ratatui type, but its fields (`x`, `y`, `width`, `height`) are plain `u16`. Storing them as `u16` in the model is fine for hit testing, but naming them confusingly as "rects" couples the model to the rendering concept. Terminal resize events change all rects instantly.

**Do this instead:** Store click regions in a separate `ClickRegionMap` that lives in the TUI layer (`tui::App`), not in `AppModel`. This map is rebuilt each render frame and is not part of the model's persistent state.

### Anti-Pattern 4: Routing FIDO2 Through PC/SC

**What people do:** Attempt to access FIDO2 credential counts or PIN status via PC/SC CCID commands because the existing card.rs infrastructure is already there.

**Why it's wrong:** FIDO2/CTAP uses the USB HID interface (`0xF1D0` usage page), not CCID. The YubiKey exposes FIDO2 on a separate USB interface that PC/SC does not enumerate. Attempting PC/SC FIDO2 access will either fail silently or produce misleading responses.

**Do this instead:** For v1.1, the FIDO2 screen shows only what can be derived from existing `YubiKeyInfo` (model capability flags). Add a note in the UI that full FIDO2 management requires `libfido2`. Implement `hidapi` integration in a later milestone.

### Anti-Pattern 5: One Monolithic Action Enum

**What people do:** Define a single `Action` enum for all screens: `Action::PinSetUser`, `Action::KeysImport`, `Action::OathCalculate`, etc.

**Why it's wrong:** Every screen module must import and match on every other screen's actions. Adding a screen adds variants that all other match arms must handle. Compile errors cascade.

**Do this instead:** Per-screen action enums (`PinAction`, `KeysAction`, `OathAction`). The top-level dispatch in `tui::App` handles the routing; each screen's `handle_key()` returns only its own action type.

---

## Scaling Considerations

This is a desktop application, not a server. "Scaling" here means adding screens and features without the codebase becoming unmanageable.

| Scale | Architecture Concern | Approach |
|-------|---------------------|---------|
| 5-7 screens (current) | app.rs monolith | Per-screen modules with own handle_key() |
| 10-15 screens (v1.1) | Model coherence across screens | AppModel stays flat; per-screen sub-models |
| TUI + Tauri (v2.0) | Shared model, two frontends | model/ crate with zero UI deps; feature flags |
| TUI library swap | Ratatui → other | All ratatui in src/tui/ only; model untouched |

---

## Sources

- [Ratatui: The Elm Architecture (TEA)](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/) — Model/Update/View pattern, Rust code examples
- [Ratatui: Component Architecture](https://ratatui.rs/concepts/application-patterns/component-architecture/) — Component trait, per-component handle_events, Action enum
- [Ratatui: Event Handling](https://ratatui.rs/concepts/event-handling/) — Centralized vs. distributed event loops
- [Ratatui: Discussion #1051 — Mouse hit testing on Rect](https://github.com/ratatui/ratatui/discussions/1051) — No native hit testing; community pattern of storing Rect in state
- [YKOATH Protocol Specification](https://developers.yubico.com/OATH/YKOATH_Protocol.html) — AID, SELECT, LIST, CALCULATE INS bytes and TLV format
- [Yubico SDK: FIDO2 Commands](https://docs.yubico.com/yesdk/users-manual/application-fido2/fido2-commands.html) — GetInfo, HID transport requirement
- [Yubico SDK: OTP Commands](https://docs.yubico.com/yesdk/users-manual/application-otp/otp-commands.html) — Status structure, sequence number
- [Microsoft tui-test](https://github.com/microsoft/tui-test) — PTY-based E2E testing alternative to tmux
- [Tauri State Management](https://v2.tauri.app/develop/state-management/) — Arc<Mutex<T>> pattern, Tauri command IPC

---

*Architecture research for: yubitui v1.1 Model/View separation + new screens*
*Researched: 2026-03-26*
