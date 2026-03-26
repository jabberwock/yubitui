# Stack Research

**Domain:** YubiKey TUI management (v1.1 new feature additions)
**Researched:** 2026-03-26
**Confidence:** HIGH (core protocol research) / MEDIUM (testing patterns)

---

## Scope

This file covers ONLY the new stack additions required for v1.1. The existing stack
(Rust, ratatui 0.29, crossterm 0.28, pcsc 2.8, tokio 1.40, anyhow, thiserror, tracing,
serde, clap, chrono, base64, sha2) is validated and not re-researched.

---

## New Dependencies Needed

### TOTP/HOTP Code Generation (RFC 6238 / RFC 4226)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `totp-rs` | 5.7.1 | TOTP RFC 6238 code generation and verification | Most maintained, RFC-compliant, optional `otpauth` URL parsing for importing credentials from QR codes, `zeroize` feature for secret memory clearing. Latest release March 9, 2026. |

**TOTP covers the on-device calculation use case.** The OATH application on YubiKey does its
own TOTP calculation via the YKOATH CALCULATE APDU — `totp-rs` is needed for the _display_
side: showing the current 6-digit code to the user by replicating the TOTP calculation locally
from the stored secret. It is also needed if yubitui ever manages OATH credentials on-card and
needs to verify codes.

**HOTP (RFC 4226) is covered by `totp-rs` indirectly** — the crate implements TOTP which is
built on HMAC-SHA1/SHA256 (same as HOTP), but does not expose a standalone HOTP function.
For HOTP counter-based OTP display specifically, implement it directly using the existing `sha2`
+ `hmac` dependency pattern, or add `hmac = "0.12"` as a dev dependency. The math is 5 lines.
Do not add a dedicated HOTP-only crate — none are well-maintained enough to justify.

**Alternatives considered:**

| Recommended | Alternative | Why Not |
|-------------|-------------|---------|
| `totp-rs 5.7.1` | `oath 0.10.2` | Last active in 2021, unmaintained |
| `totp-rs 5.7.1` | `totp_rfc6238` | Lower download count, less ecosystem presence |
| `totp-rs 5.7.1` | `thotp` | Smaller, but no otpauth URL support needed here |

**Cargo.toml addition:**
```toml
totp-rs = { version = "5.7", features = ["otpauth", "zeroize"] }
```

---

### YKOATH APDU (TOTP/HOTP On-Card Operations)

No new crate needed. The YKOATH protocol uses ISO 7816-4 APDUs over CCID — the existing
`pcsc` crate handles the transport. The protocol is fully documented and implementable
with the existing BER-TLV infrastructure already in `src/yubikey/card.rs`.

**OATH AID and key commands (verified from official Yubico docs):**

```
SELECT AID: 0xA4 → AID: A0 00 00 05 27 21 01
PUT credential: INS 0x01, TLV: 0x71 (name), 0x73 (key+algo+digits), 0x7A (HOTP IMF)
DELETE credential: INS 0x02, TLV: 0x71 (name)
LIST credentials: INS 0xA1
CALCULATE: INS 0xA2, TLV: 0x71 (name), 0x74 (challenge/timestamp)
RESET OATH app: INS 0x04, P1: 0xDE, P2: 0xAD

Algorithm byte in key tag (0x73):
  Type: HOTP=0x10, TOTP=0x20
  Hash: SHA1=0x01, SHA256=0x02, SHA512=0x03
```

Source: https://developers.yubico.com/OATH/YKOATH_Protocol.html (HIGH confidence)

---

### OTP Slot Management (Yubico OTP, Static Password, HMAC-SHA1, HOTP)

**Critical finding: OTP slots use HID keyboard transport, NOT CCID/PC-SC.**

The YubiKey OTP application routes through USB HID feature reports (70 bytes = 10 × 7-byte
reports) on the keyboard interface. It is NOT accessible via the existing `pcsc` channel.
This means OTP slot management requires a different transport layer entirely.

**Two options:**

**Option A: `hidapi` crate (RECOMMENDED for basic slot status read)**
```toml
hidapi = { version = "2.6", features = ["linux-static-hidraw"] }
```
- Provides cross-platform USB HID device access
- Can read YubiKey device info (VID=0x1050) and OTP slot status
- Requires OS-level HID access (udev rules on Linux, no admin on macOS/Windows)
- Used by `ctap-hid-fido2` as its transport layer

**Option B: Skip OTP slot write operations in v1.1 (RECOMMENDED)**
Reading OTP slot status (which slots are programmed) is feasible via hidapi.
Writing slot configuration (programming Yubico OTP, static password, HMAC challenge-response)
requires implementing the full Yubico HID frame protocol — 70-byte feature reports with a
specific sequence format. This is non-trivial and underdocumented. **Defer slot write operations
to a future milestone.** Scope v1.1 OTP as: read slot status, display what's configured,
and explain each slot type in the in-TUI education system.

| Recommended | Alternative | Why Not |
|-------------|-------------|---------|
| `hidapi 2.6` for read-only OTP status | Full HID frame protocol for write | Write protocol is complex, underdocumented, high implementation risk |
| Skip slot writes in v1.1 | ykman subprocess | ykman dependency is explicitly banned per project constraints |

**Cargo.toml addition (if OTP slot status read is in scope):**
```toml
hidapi = "2.6"
```

---

### FIDO2/WebAuthn Information Display

**Critical finding: FIDO2 uses HID FIDO transport (USB HID class 0xF1D0), NOT CCID.**

FIDO2 communicates via CTAP2 over HID FIDO — a dedicated USB interface separate from both
the keyboard HID (OTP) and CCID (PIV/OATH/OpenPGP) interfaces. The existing `pcsc` crate
cannot access FIDO2 data.

**Recommended crate: `ctap-hid-fido2 3.5.9`**
```toml
ctap-hid-fido2 = "3.5"
```
- Latest version: 3.5.9 (released March 15, 2026) — actively maintained
- Implements CTAP 2.0 and 2.1 with GetInfo command
- `get_info()` returns AuthenticatorInfo: aaguid, firmware versions, supported algorithms,
  options (rk, uv, plat, etc.), max resident credentials, PIN retry count
- Cross-platform: macOS, Windows, Linux (requires `libusb-1.0-0-dev` + `libudev-dev` on Linux)
- Uses `hidapi` internally (not pcsc)
- GetInfo does NOT require PIN — suitable for read-only display

**What GetInfo returns (CTAP2 standard, up to 20 elements on YubiKey):**
- AAGUID, supported CTAP versions, extensions, options map (rk, uv, clientPin, etc.)
- Max credentials, max large blob, PIN retry counter
- Resident credential count (requires PIN)

**Platform caveat:** On Windows, `ctap-hid-fido2` requires administrator privileges for raw
HID access. This is a known limitation of CTAP2 USB access on Windows and affects all FIDO2
libraries, not just this one. Document this limitation in the UI.

**Alternative: implement GetInfo directly via `hidapi`**
CTAP2 GetInfo is command byte `0x04` with CBOR response. Would require adding `ciborium` or
`serde_cbor` for CBOR decoding. More work than using `ctap-hid-fido2`, no clear benefit.

---

### Tmux-Based E2E Test Suite

**Finding: No mature Rust crate provides a complete tmux-based TUI test harness. Two viable
approaches exist; use them in combination.**

**Approach A: ratatui TestBackend + insta snapshots (unit/integration level)**

The `ratatui` TestBackend renders to a buffer without a real terminal. Combined with `insta`
for snapshot assertions, this covers rendering correctness without hardware.

```toml
[dev-dependencies]
insta = { version = "1.42", features = ["filters"] }
```

Pattern:
```rust
let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
terminal.draw(|frame| frame.render_widget(&app, frame.area())).unwrap();
insta::assert_snapshot!(terminal.backend());
```

This is the ratatui project's own recommended testing approach and is HIGH confidence.
Source: https://ratatui.rs/recipes/testing/snapshots/

**Approach B: tmux shell scripts with `send-keys` / `capture-pane` (true E2E)**

For testing actual keyboard interaction flows (PIN entry, wizard navigation, mouse clicks),
a tmux-based shell script harness is the practical industry approach for TUI E2E testing.
Lazygit uses a variant of this (sandboxed test sessions). The pattern:

```bash
# Launch app in detached tmux session
tmux new-session -d -s yubitui-test -x 220 -y 50 "./target/debug/yubitui"
sleep 0.5

# Send keystrokes
tmux send-keys -t yubitui-test "6" ""    # Navigate to PIV screen

# Capture and assert
OUTPUT=$(tmux capture-pane -t yubitui-test -p)
echo "$OUTPUT" | grep -q "PIV Certificates" || exit 1

# Cleanup
tmux kill-session -t yubitui-test
```

**No Rust crate needed for this approach.** Scripts live in `tests/e2e/` and run via
`cargo test --test e2e` with `std::process::Command` invoking the shell scripts, or
standalone bash scripts called from a Makefile.

**`tmux_interface` crate (v0.4.0, released March 10, 2026) exists** but is marked
"experimental/unstable" by its authors. It wraps tmux CLI commands in Rust types.
Useful if test harness complexity grows beyond shell scripts, but adds a dependency for
something shell handles natively. **Skip for v1.1; revisit if test suite grows large.**

**Recommended v1.1 testing stack:**

| Layer | Tool | Coverage |
|-------|------|----------|
| Widget rendering | ratatui TestBackend + insta | Visual correctness, layout |
| Navigation flows | tmux send-keys bash scripts | Keyboard-driven user journeys |
| Protocol parsing | existing cargo test (87 tests) | APDU parsers, BER-TLV |

```toml
[dev-dependencies]
insta = { version = "1.42", features = ["filters"] }
```

---

### Mouse Event Handling (Fix Existing Broken Support)

**No new crate needed.** The fix is in how crossterm mouse capture is enabled and how
click positions are mapped to widget areas.

**Root cause of current breakage (HIGH confidence from crossterm docs):**
Mouse events require `EnableMouseCapture` to be executed on the terminal at startup, and
`DisableMouseCapture` at shutdown. If these aren't called in the right order relative to
raw mode setup, mouse events are silently dropped.

**Required pattern (crossterm 0.28, already in Cargo.toml):**
```rust
// Startup
execute!(terminal.backend_mut(), EnableMouseCapture)?;

// Shutdown (must be before disable_raw_mode)
execute!(terminal.backend_mut(), DisableMouseCapture)?;
disable_raw_mode()?;
```

**Hit testing with `Rect::contains()`:**
`Rect::contains(Position { x, y })` was added in ratatui 0.26. Already available in
ratatui 0.29. The pattern is: store widget Rect areas in app state during render, then
check `rect.contains(Position { x: event.column, y: event.row })` in the mouse handler.

**Mouse event types available in crossterm 0.28:**
- `MouseEventKind::Down(MouseButton)` — click
- `MouseEventKind::Up(MouseButton)` — release
- `MouseEventKind::ScrollUp` / `ScrollDown` — scroll
- `MouseEventKind::Drag(MouseButton)` — drag
- `MouseEventKind::Moved` — hover (rarely needed)

**Note on ratatui 0.30 upgrade:** Ratatui 0.30 raises MSRV to 1.86.0. The project currently
specifies `rust-version = "1.75"`. Upgrading to ratatui 0.30 would require raising the MSRV.
**Stay on ratatui 0.29 for v1.1.** The mouse fix does not require 0.30.

---

### Model/View Architectural Separation (Tauri-Ready)

**No new crates needed.** This is a code architecture change, not a dependency change.

**Pattern: Pure Rust state structs + message enum (Elm-like without a framework)**

The goal is that `src/yubikey/` (model/business logic) has zero ratatui imports, and
`src/ui/` (view) owns all ratatui types. This is the correct separation for Tauri
compatibility because Tauri's command pattern expects plain Rust types as return values
from `#[tauri::command]` functions.

**Architectural layers:**

```
src/
  model/          ← pure Rust state; no ratatui imports; serializable
    app_state.rs  ← AppState struct (replaces app.rs mixing)
    yubikey.rs    ← YubiKeyState, SlotInfo, OathCredential, etc.
    messages.rs   ← Message/Action enum for state transitions
  yubikey/        ← existing hardware I/O; returns model types only
  ui/             ← ratatui-only; reads from model, emits Messages
    app.rs        ← event loop, wires model + view
    screens/      ← one file per screen; takes &AppState, returns Option<Message>
```

**Why this makes Tauri integration straightforward:**
Tauri commands are async Rust functions that receive and return serde-serializable types.
If `AppState` and `YubiKeyState` are plain Rust structs with `#[derive(Serialize)]`,
a Tauri frontend can call the same yubikey/ logic that the TUI uses, just with a different
View layer. No "porting" needed — add a new `src/ui_tauri/` that calls the same model.

**Existing serde dependency** (`serde = { version = "1.0", features = ["derive"] }`) already
in Cargo.toml covers serialization of model types. No new dependencies required.

---

## Complete Cargo.toml Additions for v1.1

```toml
[dependencies]
# OATH/TOTP support (on-device calculation + local display)
totp-rs = { version = "5.7", features = ["otpauth", "zeroize"] }

# FIDO2 info display (CTAP2 GetInfo via HID FIDO transport)
ctap-hid-fido2 = "3.5"

# OTP slot status read (HID keyboard transport) — add only if OTP slot read is in v1.1 scope
# hidapi = "2.6"

[dev-dependencies]
# Snapshot testing for TUI rendering
insta = { version = "1.42", features = ["filters"] }
```

**Note:** `hidapi` is listed as conditional because `ctap-hid-fido2` already pulls it in
transitively. Check dependency tree before adding it explicitly.

---

## What NOT to Add

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `ykman` subprocess calls | Explicitly banned; defeats the project's core purpose | Native APDUs via pcsc, hidapi |
| `yubikey` crate (the PIV crate) | PIV already implemented natively; this crate is PIV-only, unaudited | Existing src/yubikey/card.rs |
| `oath` crate | Unmaintained (last activity 2021) | totp-rs |
| `tmux_interface` crate | Marked experimental/unstable by authors | Shell scripts + tmux CLI directly |
| `ratatui-testlib` | Released Dec 2025, v0.1.0, very new, PTY complexity not needed | insta + TestBackend |
| ratatui upgrade to 0.30 in v1.1 | MSRV bump to 1.86 is a separate concern; no v1.1 feature requires it | Stay on 0.29; schedule 0.30 upgrade separately |
| `ctap2` or `fido-hid-rs` | Less maintained than ctap-hid-fido2 | ctap-hid-fido2 3.5.9 |

---

## Transport Summary by Feature

| Feature | Transport | Access Method | New Dependency |
|---------|-----------|---------------|---------------|
| OATH TOTP/HOTP credentials | CCID (PC/SC) | Existing pcsc crate + YKOATH APDUs | totp-rs (local calculation) |
| PIV certificates | CCID (PC/SC) | Existing pcsc crate (already done) | none |
| OpenPGP | CCID (PC/SC) | Existing pcsc crate (already done) | none |
| FIDO2 info display | HID FIDO (USB HID 0xF1D0) | ctap-hid-fido2 crate | ctap-hid-fido2 |
| OTP slots (read status) | HID Keyboard (USB HID) | hidapi crate | hidapi (transitively from ctap-hid-fido2) |
| OTP slots (write config) | HID Keyboard feature reports | 70-byte HID frame protocol | DEFER — complex, high risk |
| Mouse events | crossterm 0.28 | EnableMouseCapture fix + Rect::contains | none (fix in code) |
| TOTP display (local) | n/a | totp-rs 5.7 | totp-rs |
| E2E testing | tmux CLI | Shell scripts + insta snapshots | insta (dev) |

---

## Version Compatibility

| Package | Version | Compatible With | Notes |
|---------|---------|-----------------|-------|
| ratatui | 0.29.x | crossterm 0.28.x | Stay here; 0.30 bumps MSRV to 1.86 |
| crossterm | 0.28.x | ratatui 0.29.x | Already in use; EnableMouseCapture in event module |
| totp-rs | 5.7.1 | sha2 0.10.x | sha2 already in Cargo.toml; no conflict |
| ctap-hid-fido2 | 3.5.9 | hidapi 2.x | Brings hidapi transitively; check for duplicate |
| insta | 1.42.x | ratatui TestBackend | Snapshot format is stable across insta 1.x |

---

## Sources

- https://developers.yubico.com/OATH/YKOATH_Protocol.html — OATH APDU command set (HIGH confidence)
- https://docs.yubico.com/hardware/yubikey/yk-tech-manual/yk5-apps.html — YubiKey transport per application (HIGH confidence)
- https://docs.yubico.com/yesdk/users-manual/application-otp/hid.html — OTP uses HID keyboard transport (HIGH confidence)
- https://docs.rs/crate/totp-rs/latest — totp-rs 5.7.1 features and dependencies (HIGH confidence)
- https://github.com/gebogebogebo/ctap-hid-fido2 — ctap-hid-fido2 3.5.9 capabilities (HIGH confidence)
- https://ratatui.rs/recipes/testing/snapshots/ — insta snapshot testing pattern (HIGH confidence)
- https://ratatui.rs/highlights/v030/ — ratatui 0.30 MSRV change to 1.86 (HIGH confidence)
- https://docs.rs/crossterm/latest/crossterm/event/struct.EnableMouseCapture.html — mouse capture pattern (HIGH confidence)
- https://github.com/ratatui/ratatui/discussions/1051 — Rect::contains() added in 0.26 (MEDIUM confidence via search)
- https://github.com/AntonGepting/tmux-interface-rs — tmux_interface v0.4.0 marked experimental (MEDIUM confidence)

---
*Stack research for: yubitui v1.1 new features*
*Researched: 2026-03-26*
