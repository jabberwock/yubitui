# Phase 2: UX Polish — Menus, Wizards & Bug Fixes - Research

**Researched:** 2026-03-24
**Domain:** ratatui TUI UX patterns, gpg card scripting, YubiKey key management, cross-platform gnupg paths
**Confidence:** HIGH (core patterns), MEDIUM (gpg scripting), HIGH (bug fixes)

---

## Summary

Phase 2 has three distinct workstreams: (1) UX architecture — adding context menus and multi-step wizard flows to a ratatui app; (2) bug fixes — two identified defects in SSH detection and PIN unblock; (3) feature additions — key attribute configuration and authorized_keys management.

The existing codebase already has a solid enum-state-machine pattern (`Screen`, `KeyScreen`, `PinScreen`, `SshScreen`). The dominant task is extending this pattern for wizard steps and overlaying popup menus using ratatui's built-in `Clear` widget with `area.centered()`. No new TUI crates are required. `ykman` is available on this machine (v5.0.1) and provides a cleaner interface for factory reset than raw APDU commands. The gnupg path bug is confirmed and documented below.

**Primary recommendation:** Extend existing enum-state-machine with `WizardStep` sub-enums per wizard, use `Clear` + `area.centered()` for all popups/menus, and use `gpgconf --list-dirs homedir` to resolve gnupg home reliably across platforms.

---

## Project Constraints (from CLAUDE.md)

- Run before every commit: `cargo clippy -- -D warnings`, `cargo fmt -- --check`, `cargo audit`, `gitleaks detect --source . --no-git`, `cargo test`
- No hardcoded secrets, keys, PINs, or credentials in source or tests
- No logging of sensitive values (PINs, private key material, card serial numbers)
- No `node_modules/` commits
- Prefer safe Rust; justify any `unsafe` blocks
- Cross-platform (Linux/macOS/Windows) is non-negotiable

---

## Standard Stack

### Core (no new dependencies needed)

| Library | Version (in Cargo.toml) | Purpose | Why Standard |
|---------|------------------------|---------|--------------|
| ratatui | 0.29 | TUI framework | Already in use; `Clear` widget + `area.centered()` cover all popup needs |
| crossterm | 0.28 | Terminal event handling | Already in use |
| dirs | 5.0 | Platform-aware home dir resolution | Already in use; use `config_dir()` for Windows gnupg path |

### No New Crates Required

The `tui-popup` crate (`joshka/tui-popup`, now part of `tui-widgets`) exists but is unnecessary overhead. ratatui 0.29 provides everything needed: `Clear` widget, `area.centered()`, `Constraint`-based layout. Adding `tui-popup` adds a transitive dependency for a pattern that is 15 lines of native ratatui code.

**Installation:** No new `cargo add` commands needed.

---

## Architecture Patterns

### Existing Pattern (must be extended, not replaced)

The app already uses a nested enum state machine:

```
App {
    current_screen: Screen          -- top-level navigation
    pin_state: PinState {
        screen: PinScreen           -- sub-screen within PIN section
        message: Option<String>
    }
    ssh_state: SshState { ... }
    key_state: KeyState { ... }
}
```

Event handling dispatches from `handle_key_event` into screen-specific match arms. This is the established pattern — wizards and menus extend it, they do not replace it.

### Pattern 1: Wizard Sub-Screens (extend existing `PinScreen` enum)

Wizards are modeled as additional `PinScreen` variants. The wizard steps become a `WizardStep` sub-enum stored inside the relevant state struct.

**What:** A wizard is a sequence of `PinScreen` variants. Each variant maps to a render function that shows one step. `Enter` advances, `Esc` cancels to `Main`.

**When to use:** Multi-step flows where the user must see state-dependent information before choosing a path (PIN unblock wizard, SSH enable wizard).

**Structure:**

```rust
// In src/ui/pin.rs
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PinScreen {
    Main,
    ChangeUserPin,
    ChangeAdminPin,
    SetResetCode,
    UnblockUserPin,          // existing: launches gpg passthrough
    // Wizard screens:
    UnblockWizardCheck,      // NEW: show reset_code_retries / admin_pin_retries status
    UnblockWizardWithAdmin,  // NEW: confirm path → use admin PIN
    UnblockWizardWithReset,  // NEW: confirm path → use reset code
    UnblockWizardFactoryReset, // NEW: last resort — ykman reset warning screen
}

// In PinState:
pub struct PinState {
    pub screen: PinScreen,
    pub message: Option<String>,
    // Add any wizard context if needed:
    pub wizard_path: Option<UnblockPath>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnblockPath {
    AdminPin,
    ResetCode,
    FactoryReset,
}
```

**Rendering:** Each wizard variant gets its own `render_*` function in `src/ui/pin.rs`. Navigation transitions stay in `app.rs` `handle_key_event`.

### Pattern 2: Context Menu Popup (new pattern — use `Clear` + `area.centered()`)

**What:** A floating popup rendered over the current screen. Implemented as a boolean `show_menu` flag in screen state plus a rendered `Clear`+`Block`+`List` overlay.

**When to use:** When an action list should float over the current content (dashboard context menu, per-slot key actions).

**Core pattern (ratatui 0.29, verified against official docs):**

```rust
// Source: https://ratatui.rs/recipes/render/overwrite-regions/
use ratatui::widgets::{Clear, Block, Borders, List, ListItem};
use ratatui::layout::Constraint;

fn render_with_popup(frame: &mut Frame, area: Rect, show_popup: bool, items: &[&str]) {
    // 1. Render background content normally
    render_main_content(frame, area);

    // 2. If popup is visible, overlay it
    if show_popup {
        let popup_area = area.centered(
            Constraint::Percentage(50),
            Constraint::Length(items.len() as u16 + 2),
        );

        // Clear the area first — prevents content bleeding from background
        frame.render_widget(Clear, popup_area);

        let menu_items: Vec<ListItem> = items.iter()
            .enumerate()
            .map(|(i, item)| ListItem::new(format!("[{}] {}", i + 1, item)))
            .collect();

        let menu = List::new(menu_items)
            .block(Block::default().borders(Borders::ALL).title("Actions"));
        frame.render_widget(menu, popup_area);
    }
}
```

**State extension for menus:**

```rust
// Add to relevant State structs:
pub struct KeyState {
    pub screen: KeyScreen,
    pub message: Option<String>,
    pub available_keys: Vec<String>,
    pub selected_key_index: usize,
    pub show_context_menu: bool,    // NEW
    pub menu_selected_index: usize, // NEW
}
```

### Pattern 3: Confirmation Dialog (blocking confirmation before destructive action)

**What:** A simple `Clear`-based popup with `[Y]es / [N]o` options. Stored as a `ConfirmationState` in App or the relevant screen state.

**When to use:** Before factory reset, before overwriting authorized_keys, before any irreversible operation.

```rust
pub struct ConfirmDialog {
    pub visible: bool,
    pub title: String,
    pub message: String,
    pub on_confirm: ConfirmAction,  // enum of actions
}

#[derive(Debug, Clone, Copy)]
pub enum ConfirmAction {
    FactoryReset,
    // future: OverwriteAuthorizedKeys, etc.
}
```

### Recommended File Structure Changes

```
src/
├── app.rs                         # Add ConfirmDialog field; extend handle_key_event for menus
├── ui/
│   ├── mod.rs                     # No changes needed
│   ├── pin.rs                     # Extend PinScreen enum + wizard render functions
│   ├── ssh.rs                     # Fix path display; add wizard status check
│   ├── keys.rs                    # Add context menu state; add key attribute screen
│   └── widgets/                   # NEW: shared popup/menu widget helpers
│       └── popup.rs               # render_popup(), render_confirmation_dialog()
├── yubikey/
│   ├── pin_operations.rs          # Fix unblock_user_pin(); add factory_reset()
│   └── ssh_operations.rs          # Fix get_gpg_agent_conf_path() for Windows
└── utils/
    └── config.rs                  # Fix gnupg_home() for Windows — use gpgconf homedir
```

### Anti-Patterns to Avoid

- **Passing App by mutable reference into render functions:** `ssh.rs` currently takes `&App` in `render()`. This is fine for reading state but never pass `&mut App` into render — state mutations happen only in `app.rs`.
- **Shell string interpolation for gpg commands:** Existing code correctly uses `Command::new("gpg").arg(...)`. Never switch to `Command::new("sh").arg(format!("gpg ... {}", user_input))`.
- **Duplicating gnupg path logic:** Currently duplicated across `utils/config.rs`, `yubikey/ssh_operations.rs`, and `diagnostics/ssh_agent.rs`. All three must be unified to call `utils::config::gnupg_home()`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Factory reset of YubiKey OpenPGP app | Custom APDU sequence via `gpg-connect-agent --hex` | `ykman openpgp reset` | APDU approach requires 8 sequential commands with hex codes; `ykman` wraps this in one clean call |
| Centering a popup rect | Custom `centered_rect(percent_x, percent_y, r)` helper | `area.centered(Constraint::Percentage(x), Constraint::Percentage(y))` | ratatui 0.29 has this built in |
| Cross-platform gnupg home detection | If-cfg chains | `gpgconf --list-dirs homedir` | gpgconf is the authoritative source for the actual active homedir on any platform |
| Key attribute reading | Parsing `gpg --card-status` output | `ykman openpgp keys info SIG` / `ykman openpgp info` | ykman provides structured output for key metadata |

**Key insight:** `ykman` is already installed on the target machine (v5.0.1). For administrative operations where a TUI wrapper over raw gpg is fragile (factory reset, key attribute changes), prefer `ykman` which is purpose-built and has stable CLI semantics.

---

## Bug Fix Details

### Bug 1: SSH gpg-agent.conf Wrong Path on Windows

**Location:** `src/diagnostics/ssh_agent.rs:18-19`, `src/yubikey/ssh_operations.rs:222-228`, `src/utils/config.rs:9-11`

**Root cause:** On Windows with GPG4Win (native installer), the gnupg home is `%APPDATA%\gnupg` (= `C:\Users\user\AppData\Roaming\gnupg`). The code uses `dirs::home_dir().join(".gnupg")` which on Windows returns `C:\Users\user\.gnupg`. These are different directories.

**Confirmed on dev machine:** `%APPDATA%\gnupg\gpg-agent.conf` EXISTS and contains `enable-ssh-support`. `~/.gnupg` also exists (Git Bash MSYS2 installation). `gpgconf --list-dirs homedir` reports `/c/Users/mbeha/.gnupg` on this machine's Git Bash GPG. This is GPG-distribution-dependent.

**The authoritative fix:** Use `gpgconf --list-dirs homedir` to get the gnupg home that the running gpg instance actually uses, rather than guessing paths. Fall back to `$GNUPGHOME` then `dirs::home_dir().join(".gnupg")` only if `gpgconf` fails.

```rust
// In src/utils/config.rs — authoritative fix
pub fn gnupg_home() -> Result<PathBuf> {
    // 1. Explicit override wins
    if let Ok(gnupg_home) = std::env::var("GNUPGHOME") {
        return Ok(PathBuf::from(gnupg_home));
    }

    // 2. Ask gpgconf what it actually uses — works on all platforms
    if let Ok(output) = std::process::Command::new("gpgconf")
        .arg("--list-dirs")
        .arg("homedir")
        .output()
    {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                return Ok(PathBuf::from(path_str));
            }
        }
    }

    // 3. Platform-aware fallback
    #[cfg(target_os = "windows")]
    {
        // Windows GPG4Win uses %APPDATA%\gnupg
        if let Some(appdata) = dirs::config_dir() {  // returns %APPDATA% on Windows
            return Ok(appdata.join("gnupg"));
        }
    }

    // 4. Unix fallback
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    Ok(home.join(".gnupg"))
}
```

**Also fix:** `src/diagnostics/ssh_agent.rs` and `src/yubikey/ssh_operations.rs` both duplicate the path logic and must be updated to call `utils::config::gnupg_home()` instead.

**`dirs::config_dir()` on Windows:** Returns `C:\Users\user\AppData\Roaming` (the APPDATA Roaming folder). Source: dirs crate v5.0.1 docs, verified against `dirs::home_dir()` docs.

### Bug 2: PIN Unblock Fails Silently When Reset Code Exhausted

**Location:** `src/yubikey/pin_operations.rs:19-22`

**Current code:**
```rust
pub fn unblock_user_pin() -> Result<String> {
    execute_gpg_card_edit(&["admin", "passwd", "2", "q"])
}
```

**Root cause:** `gpg --card-edit passwd 2` requires either the Admin PIN OR the Reset Code. When the Reset Code retries counter is 0 (reset code was never set or has been exhausted) AND the user calls unblock, this still tries to unblock using Admin PIN. The fundamental issue is the app shows `UnblockUserPin` without first checking which recovery path is available.

**The actual gpg behavior** (confirmed from GnuPG documentation):
- `passwd 2` = unblock User PIN. Prompts for either Reset Code (if set) or Admin PIN.
- If neither is available (both blocked/absent), the operation fails with a cryptic error.
- Factory reset is the only remaining option when both admin PIN and reset code retries = 0.

**Correct wizard flow:**
```
User presses [U] → UnblockWizardCheck
    ↓
Read yubikey_state.pin_status:
  - reset_code_retries > 0  → show "Use Reset Code path" → launch passwd 2
  - admin_pin_retries > 0   → show "Use Admin PIN path" → launch passwd 2
  - Both = 0                → show warning "No recovery possible without factory reset"
                              → offer "Factory Reset (DESTROYS ALL KEYS)" confirmation
                              → if confirmed: `ykman openpgp reset -f`
```

**Factory reset command:**
```rust
// In src/yubikey/pin_operations.rs — NEW function
pub fn factory_reset_openpgp() -> Result<String> {
    let output = std::process::Command::new("ykman")
        .arg("openpgp")
        .arg("reset")
        .arg("--force")  // skip interactive confirmation (we showed our own)
        .output()?;

    if output.status.success() {
        Ok("OpenPGP application reset. Default PINs restored: User=123456, Admin=12345678".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Factory reset failed: {}", stderr)
    }
}
```

Note: `ykman` is confirmed available at `/c/Program Files/Yubico/YubiKey Manager/ykman` on this machine.

---

## Common Pitfalls

### Pitfall 1: Render Borrow Conflicts With App State

**What goes wrong:** `app.rs::render()` currently takes `&self` and passes sub-state references to render functions. If you try to pass `&mut self.pin_state` into a function that also needs `&self.yubikey_state`, Rust borrow checker rejects it.

**Why it happens:** Ratatui's design requires all rendering in a single closure. Splitting borrows from the same struct can conflict.

**How to avoid:** Keep render functions taking separate `&PinState` and `&Option<YubiKeyState>` parameters (already the existing pattern). Never pass `&mut App` or `&mut SubState` to render functions — mutations happen before or after the render closure, never inside it.

**Warning signs:** Compiler error `cannot borrow 'self' as mutable because it is also borrowed as immutable`.

### Pitfall 2: gpg --card-edit Command Piping Is Unreliable for Interactive Operations

**What goes wrong:** `execute_gpg_card_edit` pipes command strings to gpg's stdin. GPG's card-edit parser does not guarantee that piped input sequences work identically to interactive input. Specifically, the pinentry dialog (for PIN entry) uses a separate process (pinentry-curses or pinentry-qt) that reads from the TTY, not from stdin.

**Why it happens:** GPG's PIN entry goes through the gpg-agent → pinentry chain, which bypasses stdin piping. The current code works because it pipes "which menu option" but not "enter PIN value" — the PIN prompts go to TTY.

**How to avoid:** Only pipe menu navigation commands (numbers, "admin", "quit"). Never attempt to pipe actual PIN values — that will fail and risks confusing the user. The TUI correctly drops to the terminal for interactive operation. Keep this pattern.

**Warning signs:** Piped gpg session completes instantly with exit code 0 but no PIN change actually occurred.

### Pitfall 3: `area.centered()` Requires ratatui 0.26+

**What goes wrong:** Older ratatui examples show a manual `centered_rect()` helper function. Code copied from those examples will compile but is redundant with the built-in API.

**Why it happens:** `area.centered()` was added in ratatui 0.26. This project uses 0.29. Use the built-in.

**How to avoid:** Use `area.centered(Constraint::Percentage(x), Constraint::Percentage(y))` directly. Do not add a `centered_rect` helper.

### Pitfall 4: ykman Path on Windows Requires Full Path or PATH Update

**What goes wrong:** `Command::new("ykman")` may fail on Windows if `ykman` is in `C:\Program Files\Yubico\YubiKey Manager\` which is not in PATH in all contexts.

**Why it happens:** ykman Windows installer does not always add itself to PATH. On this dev machine it's at `/c/Program Files/Yubico/YubiKey Manager/ykman`.

**How to avoid:** When spawning ykman, try `ykman` first; if the spawn fails with "not found", fall back to the well-known Windows path `C:\Program Files\Yubico\YubiKey Manager\ykman.exe`. Use a helper function that probes both.

```rust
fn find_ykman() -> Result<std::path::PathBuf> {
    // Try PATH first
    if which_ykman().is_ok() {
        return Ok("ykman".into());
    }
    // Well-known Windows location
    #[cfg(target_os = "windows")]
    {
        let path = std::path::PathBuf::from(
            r"C:\Program Files\Yubico\YubiKey Manager\ykman.exe"
        );
        if path.exists() {
            return Ok(path);
        }
    }
    anyhow::bail!("ykman not found. Install from https://www.yubico.com/support/download/yubikey-manager/")
}
```

### Pitfall 5: Factory Reset Warning UX — Must Be Explicit

**What goes wrong:** Showing a factory reset option without prominent warnings leads to accidental key destruction.

**Why it happens:** Users in a "locked out" panic state may press keys quickly without reading.

**How to avoid:** The factory reset screen must:
1. Show a `[WARNING]` in red with `Modifier::BOLD`
2. Explicitly state "This will PERMANENTLY DELETE all GPG keys on the card"
3. Require typing `RESET` or pressing `Y` after a confirmation dialog — not just `Enter`
4. Show default PINs that will be restored (User: `123456`, Admin: `12345678`)

---

## Code Examples

### Popup Overlay (ratatui 0.29)

```rust
// Source: https://ratatui.rs/recipes/render/overwrite-regions/
use ratatui::widgets::{Clear, Block, Borders, Paragraph};
use ratatui::layout::Constraint;

// Inside a render function:
if show_popup {
    let popup_area = area.centered(
        Constraint::Percentage(60),
        Constraint::Percentage(40),
    );
    frame.render_widget(Clear, popup_area);
    let block = Block::default().borders(Borders::ALL).title("Confirm");
    let paragraph = Paragraph::new("Are you sure? [Y]es / [N]o")
        .block(block);
    frame.render_widget(paragraph, popup_area);
}
```

### ykman Factory Reset

```bash
# Verified available on dev machine:
ykman openpgp reset --force
# Output: "WARNING! This will delete all stored OpenPGP keys..."
# With --force/-f: skips interactive prompt
```

```rust
// Source: ykman CLI docs https://docs.yubico.com/software/yubikey/tools/ykman/OpenPGP_Commands.html
let output = Command::new("ykman")
    .arg("openpgp")
    .arg("reset")
    .arg("--force")
    .output()?;
```

### gpg key-attr command sequence (for key attribute wizard)

```
gpg --edit-card --expert
> admin
> key-attr
  Changing card key attribute for: Signature key
  Please select what kind of key you want:
   (1) RSA
   (2) ECC
  Your selection? 2
  Please select which elliptic curve you want:
   (1) Curve 25519
   (4) NIST P-384
  Your selection? 1
  [Repeat for Encryption key]
  [Repeat for Authentication key]
> quit
```

This requires `--expert` flag to access ECC options. RSA requires only `--edit-card` (no `--expert`).

### ykman openpgp info (key attribute reading)

```bash
ykman openpgp info
# Shows: key types per slot (SIG, ENC, AUT), touch policy, etc.
ykman openpgp keys info SIG
ykman openpgp keys info ENC
ykman openpgp keys info AUT
```

### Authoritative gnupg homedir detection

```bash
gpgconf --list-dirs homedir
# Returns the path gpg actually uses, cross-platform
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual `centered_rect()` helper function | `area.centered(Constraint::...)` built-in | ratatui 0.26 | Remove any copied helper |
| APDU hex commands for YubiKey reset | `ykman openpgp reset` | ykman 1.x | Simpler, less error-prone |
| `dirs::home_dir().join(".gnupg")` for Windows | `gpgconf --list-dirs homedir` | Always correct | Fixes Windows GPG4Win path |

---

## Open Questions

1. **Key attribute wizard scope for Phase 2**
   - What we know: `gpg --card-edit --expert key-attr` works; requires 3 separate selections (SIG/ENC/AUT)
   - What's unclear: Is the key attribute wizard in scope for Phase 2 or deferred? The phase objective says "potentially"
   - Recommendation: Scope to read-only display first (show current algorithm via `ykman openpgp info`); make the set-attributes path a Phase 3 candidate since it requires `--expert` mode and is rarely needed

2. **authorized_keys management implementation approach**
   - What we know: `ssh_operations::add_to_remote_authorized_keys()` already exists but is marked `#[allow(dead_code)]`
   - What's unclear: Should the TUI expose this as an interactive flow (requires user to type hostname)?
   - Recommendation: Display-and-copy UX first — show SSH public key in a copyable popup, include a static hint about `ssh-copy-id` command

3. **ykman availability on user machines**
   - What we know: ykman 5.0.1 is installed on this dev machine
   - What's unclear: Should factory reset be blocked if ykman is absent?
   - Recommendation: Detect ykman at wizard entry; if absent, show `ykman is required for factory reset. Install from yubico.com/yubikey-manager` and disable the option

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| ykman | Factory reset, key attr info | Yes | 5.0.1 | Show install instructions; disable factory reset option |
| gpg | All PIN operations | Yes | 2.4.5 | Fatal — app cannot function |
| gpgconf | gnupg home detection, agent restart | Yes | 2.4.5 | Fall back to path heuristics |
| Rust | Build | Yes | 1.94.0 | — |
| cargo | Build | Yes | 1.94.0 | — |

**Missing dependencies with no fallback:** None that block core TUI functionality.

**Missing dependencies with fallback:** ykman (graceful degradation for factory reset path).

---

## Sources

### Primary (HIGH confidence)
- ratatui docs `https://ratatui.rs/recipes/render/overwrite-regions/` — Clear widget popup pattern verified
- ratatui docs `https://ratatui.rs/recipes/layout/center-a-widget/` — `area.centered()` API verified
- ratatui 0.29 highlights `https://ratatui.rs/highlights/v029/` — confirmed features available in current version
- ykman CLI docs `https://docs.yubico.com/software/yubikey/tools/ykman/OpenPGP_Commands.html` — `ykman openpgp reset --force` verified
- dirs crate v5.0.1 docs `https://docs.rs/dirs/5.0.1/dirs/fn.home_dir.html` — Windows returns `C:\Users\user`
- Live machine probe: `%APPDATA%\gnupg\gpg-agent.conf` exists and contains `enable-ssh-support`; `gpgconf --list-dirs homedir` returns `~/.gnupg` (Git Bash GPG)

### Secondary (MEDIUM confidence)
- GnuPG card-edit documentation `https://www.gnupg.org/howtos/card-howto/en/ch03s02.html` — PIN menu options (1-4)
- Yubico Card Edit docs `https://developers.yubico.com/PGP/Card_edit.html` — menu structure verified
- Yubico YubiKey 5.2.3 docs `https://developers.yubico.com/PGP/YubiKey_5.2.3_Enhancements_to_OpenPGP_3.4.html` — key-attr command flow
- WebSearch: Windows gnupg paths — `%APPDATA%\gnupg` is config (Roaming), `%LOCALAPPDATA%\gnupg` is sockets (Local), verified against live filesystem

### Tertiary (LOW confidence)
- WebSearch: `factory-reset` via APDU commands — noted for reference; superseded by `ykman openpgp reset`

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already in Cargo.toml; no new crates needed
- Architecture: HIGH — ratatui docs verified; pattern is extension of existing working code
- Bug fixes: HIGH — root causes confirmed via live filesystem probe + gpgconf output
- Pitfalls: HIGH (borrow checker, piping) / MEDIUM (ykman PATH)
- gpg scripting: MEDIUM — interactive flow is reliable; exact behavior on all edge cases unverified without live YubiKey

**Research date:** 2026-03-24
**Valid until:** 2026-06-24 (stable ecosystem; ratatui 0.30 may add new APIs but 0.29 patterns remain valid)
