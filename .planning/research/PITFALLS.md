# Pitfalls Research

**Domain:** Rust TUI (ratatui) — YubiKey management app, v1.1 feature additions
**Researched:** 2026-03-26
**Confidence:** HIGH (protocol specs from official Yubico docs + crossterm/ratatui issues); MEDIUM (tmux test patterns); LOW flagged inline

---

## Critical Pitfalls

### Pitfall 1: Mouse Coordinates Are Render-Time, Not Event-Time — Areas Must Be Stored

**What goes wrong:**
`MouseEvent` gives `(column, row)` in terminal coordinates. Widget areas (the `Rect` for each button, menu item, list row) are only known inside the `terminal.draw()` closure. If you do not save those rects after render, you cannot do hit-testing in the event handler. The naive fix — recalculating the same layout in the event handler — diverges when layout logic changes, causing silent misalignment between what the user clicks and what fires.

**Why it happens:**
ratatui's render pass is the single source of truth for geometry. There is no retained widget tree, no DOM, no automatic hit-testing. Developers assume they can reproduce layout math in the event handler, but Layout split results are not 100% deterministic across call sites if terminal width changes between render and event processing (e.g., window resize between frames).

**How to avoid:**
Introduce a `ClickMap` (or `RenderedAreas`) struct that is populated during render and read during event handling. The pattern:
```
// In App:
rendered_areas: RenderedAreas  // cleared each frame, repopulated in render()

// In render():
self.rendered_areas.dashboard_menu = chunks[2];
self.rendered_areas.pin_submit_btn = submit_rect;

// In handle_mouse_event():
if self.rendered_areas.dashboard_menu.contains(Position { x: col, y: row }) { ... }
```
`Rect::contains(Position)` is the correct API. Do NOT recompute `Layout::split()` in the event handler.

**Warning signs:**
- Click handlers test `col >= X && col < X+W` using hard-coded offsets
- Popup click-to-close fires even when clicking outside the popup area
- Scroll events affect the wrong panel when multiple scrollable views exist

**Phase to address:** Model/View Architecture phase (phase 1 of v1.1). The `RenderedAreas` struct belongs in the Model layer and must be defined before mouse features are added to individual screens.

---

### Pitfall 2: Z-Order Is Not Enforced — Popups Pass Clicks Through to Background

**What goes wrong:**
yubitui currently renders a context menu overlay over the dashboard. If `handle_mouse_event` processes click coordinates without checking whether a popup is active on top, a click intended to dismiss the popup can simultaneously trigger the background widget at the same coordinates.

**Why it happens:**
ratatui renders in painter order (last widget drawn is on top) but the event handler has no corresponding "hit test from top to bottom" mechanism. The event handler must manually mirror the render Z-order.

**How to avoid:**
In the event handler, check the highest-Z layer first. If the event is consumed by the overlay (popup/context-menu), return immediately without processing it against background widgets. Concretely:

```
fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<()> {
    // Highest Z first
    if self.dashboard_state.show_context_menu {
        if self.handle_context_menu_mouse(mouse) { return Ok(()); }
    }
    // Then background widgets
    self.handle_screen_mouse(mouse)
}
```

**Warning signs:**
- Clicking a menu item also triggers an action on the screen behind it
- Pressing Escape closes a popup but also moves the cursor one position

**Phase to address:** Mouse support phase. Requires `RenderedAreas` (Pitfall 1) to be in place first so overlay bounds are known.

---

### Pitfall 3: Model/View Split Breaks if ratatui Types Leak into YubiKeyState

**What goes wrong:**
The stated goal is "no ratatui in business logic." If `YubiKeyState`, `PivState`, or new `OathState`/`Fido2State` structs import `ratatui::style::Color` or `ratatui::widgets::ListState`, the Model layer is coupled to the renderer. Swapping to Tauri or a different TUI library requires touching every model struct.

**Why it happens:**
It is tempting to put display-formatting logic in model structs — e.g., `fn status_color(&self) -> Color` — because it is convenient during initial development. Once embedded, it is hard to extract without a big-bang refactor.

**How to avoid:**
The rule is absolute: `src/yubikey/` and any new `src/model/` modules must have zero imports from `ratatui::*`. UI formatting belongs in `src/ui/`. If a model value needs to be displayed with color, the UI layer maps the value:

```
// WRONG — in yubikey/oath.rs:
use ratatui::style::Color;
fn credential_color(&self) -> Color { ... }

// RIGHT — in ui/oath.rs:
fn credential_color(cred: &OathCredential) -> Color { ... }
```

**Warning signs:**
- `cargo grep 'use ratatui' src/yubikey/` returns any result
- Model structs have fields typed `Style`, `Color`, `ListState`
- A `render_*` function is defined in `src/yubikey/`

**Phase to address:** Model/View Architecture phase — enforce as a CI lint before any feature work begins.

---

### Pitfall 4: OATH CALCULATE Requires the Host to Supply the TOTP Timestamp

**What goes wrong:**
The YubiKey OATH applet has no clock. TOTP requires the current Unix timestamp divided by the time-step (default 30 seconds). If the caller does not send the correct challenge (the current 30-second epoch counter), the applet will compute a code for the wrong time window and every generated code will fail validation.

**Why it happens:**
Developers familiar with HOTP expect the device to manage counters internally. For TOTP the host is the time source. The CALCULATE APDU for TOTP requires the `challenge` TLV (tag 0x74) to contain the 8-byte big-endian current time-step value: `floor(unix_time_seconds / 30)` encoded as u64 BE.

**How to avoid:**
```rust
let timestep: u64 = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)?
    .as_secs() / 30;
let challenge: [u8; 8] = timestep.to_be_bytes();
// Include as TLV 0x74, 0x08, challenge[..] in the CALCULATE APDU
```
Always use `SystemTime::now()` at the moment of the APDU call — not cached at startup. For CALCULATE_ALL, send the same timestamp as a global challenge in the APDU.

**Warning signs:**
- Generated codes never validate against any TOTP server
- Codes validate only at key installation time (stale timestamp baked in at init)
- Code works for 30 seconds then permanently fails

**Phase to address:** OATH/TOTP feature phase.

---

### Pitfall 5: OATH Credentials May Be Password-Protected — Unauthenticated Commands Return 0x6982

**What goes wrong:**
The YKOATH applet supports an optional access key (derived from a user password via PBKDF2). If the user has set a password, every command except VALIDATE and RESET returns SW `0x6982` (authentication required). Attempting to LIST or CALCULATE without first completing the VALIDATE handshake silently fails.

The VALIDATE flow is a mutual challenge-response:
1. SELECT OATH AID — response includes tag `0x74` (challenge) if password is set
2. Compute HMAC-SHA1(access_key, device_challenge) and send with a new host challenge (tag `0x74` in request)
3. Device returns HMAC of host_challenge — verify it before trusting subsequent responses

**Why it happens:**
The presence of tag `0x74` in the SELECT response is the only indicator that authentication is required. Developers who test against unprotected keys never hit this path.

**How to avoid:**
- Always inspect the SELECT response for tag `0x74`
- If present, pause and prompt the user for the OATH password before issuing any command
- Treat `0x6982` from any OATH command as a recoverable auth error, prompt for password, re-VALIDATE, retry

**Warning signs:**
- OATH LIST returns empty on keys with known credentials
- Any OATH command returns `0x6982` or `0x6984`

**Phase to address:** OATH/TOTP feature phase.

---

### Pitfall 6: OATH CALCULATE_ALL Returns Incomplete Data for HOTP and Touch Credentials

**What goes wrong:**
`CALCULATE_ALL` (INS `0xa4`) intentionally does NOT compute codes for HOTP credentials (tag `0x77` — name only, no value) or touch-required credentials (tag `0x7c` — name only). Treating all entries as having a computed code produces nil/garbage values for these types.

Additionally, if the total response exceeds 256 bytes, the applet returns `SW 0x61xx` (more data available). Failing to issue `SEND REMAINING` (INS `0xa5`) until `0x9000` is received will produce a truncated credential list.

**How to avoid:**
- Parse the response tag per entry: `0x76` = truncated TOTP code (use this), `0x75` = full TOTP, `0x77` = HOTP (display as "press button to generate"), `0x7c` = touch required (display as "touch key to generate")
- Implement SEND REMAINING chaining in the OATH response reader, not just in the GET DATA path
- yubitui already has T=0 GET RESPONSE chaining in `get_data()` — write a parallel `send_remaining_loop()` for OATH responses using `0xa5`

**Warning signs:**
- OATH screen shows blank codes for some credentials
- Credential list is cut off when the user has more than ~8 credentials
- HOTP credentials crash with array-index-out-of-bounds when parsing response bytes

**Phase to address:** OATH/TOTP feature phase.

---

### Pitfall 7: FIDO2 CTAP2 over CCID Uses a Double-Wrapped APDU — Inner CBOR Framing Is Mandatory

**What goes wrong:**
CTAP2 commands sent over CCID (ISO 7816) are NOT plain APDUs. The outer APDU is `CLA=80 INS=10 P1=00 P2=00`, and the data field contains a one-byte command byte followed by CBOR-encoded parameters. Sending a raw CTAP2 CBOR map without this framing, or using the wrong outer INS, returns `0x6D00` (INS not supported) or garbled responses.

The FIDO2 AID is `A0 00 00 06 47 2F 00 01 00`. It must be selected before any CTAP2 commands. The AID response contains version strings — parse these to determine CTAP1/CTAP2 capability before proceeding.

**Why it happens:**
CTAP2 specifications focus on the HID transport. The CCID transport section is a short annex. The command byte (`0x04` for getInfo, `0x06` for clientPin, etc.) is easy to miss when reading the spec.

**How to avoid:**
```
// FIDO2 getInfo over CCID:
// SELECT AID: 00 A4 04 00 08 A0 00 00 06 47 2F 00 01
// getInfo command: 80 10 00 00 01 04
// Response: first byte is CTAP2 status (0x00 = OK), rest is CBOR map
```
Parse the first byte of every CTAP2 response as the status byte *before* feeding the rest to a CBOR decoder. A non-zero status byte means error — the rest of the response is not valid CBOR.

**Warning signs:**
- getInfo returns `0x6D00` or `0x6700`
- CBOR decoder panics on response bytes
- PINToken commands time out (may be sending to wrong AID)

**Phase to address:** FIDO2 feature phase. Read the FIDO Alliance CTAP spec section 8.2 (CTAP over NFC/ISO 7816) alongside Yubico SDK APDU docs.

---

### Pitfall 8: OTP Slot 1 May Have a Yubico OTP Config with an Access Code — Blind Writes Will Fail Silently

**What goes wrong:**
An off-the-shelf YubiKey ships with slot 1 programmed with Yubico OTP registered to YubiCloud. The user may have protected this slot with a 6-byte access code. Writing a new OTP slot configuration without providing the current access code will fail with a non-fatal status code but the slot will be unchanged. The write appears to succeed at the APDU layer (no exception thrown) but the old configuration persists.

**Why it happens:**
The OTP STATUS APDU response includes a `touch_triggered` flag but does not directly expose whether an access code is set. The write failure status is easy to confuse with success if the caller only checks `SW1 == 0x90`.

**How to avoid:**
- Read the OTP configuration flags before attempting any slot write
- Present an "Access Code" input field if the user is reconfiguring an existing slot
- Never auto-write slot 1 without explicit user confirmation and access code entry
- After any slot write, re-read the configuration and verify it matches the intended state (write-then-verify pattern)
- Document clearly: if the user loses their access code, that OTP slot is permanently unmodifiable (only the OATH/PIV/FIDO applets are not affected — slot is specific to the OTP application)

**Warning signs:**
- Slot write returns no error but configuration does not change
- Status flags read back identically to pre-write state
- User reports "my slot 1 still sends old Yubico OTP after reconfiguring"

**Phase to address:** OTP Slot Management phase.

---

### Pitfall 9: scdaemon Card-Busy Race on Multi-Applet Operations

**What goes wrong:**
yubitui already kills scdaemon before exclusive PIV/OpenPGP card access. The v1.1 features add OATH and FIDO2 applets — each requiring a fresh card connection and AID SELECT. If scdaemon restarts between two operations (e.g., user opens gpg-agent in another terminal while OATH is being read), the new card connection attempt returns `SCARD_E_SHARING_VIOLATION` (`0x8010000B`).

This is exacerbated by the 50ms post-kill sleep TODO still listed in PROJECT.md as unpaid tech debt.

**Why it happens:**
`gpgconf --kill scdaemon` is fire-and-forget. scdaemon can restart in under 200ms on fast systems, especially if a gpg operation is triggered externally.

**How to avoid:**
- Pay the 50ms sleep debt from v1.0 immediately before any v1.1 APDU work begins
- For OATH and FIDO2 operations, reuse the same PC/SC context across applet switches within a single user action — do not reconnect between SELECT OATH and CALCULATE
- Consider a 3-retry loop with exponential backoff (50ms, 100ms, 200ms) for `SCARD_E_SHARING_VIOLATION` specifically
- On Linux only: verify pcscd is running before connecting (diagnostics screen already does this — call the same check)

**Warning signs:**
- OATH operations fail intermittently but not consistently
- Error `0x8010000B` or `Card already in use` in logs
- Failures correlate with gpg operations in other terminals

**Phase to address:** Phase 0 (tech debt) — before any new protocol work.

---

### Pitfall 10: Incremental Refactor of app.rs Can Introduce Borrow Checker Violations at Render Time

**What goes wrong:**
`app.rs` currently does `terminal.draw(|f| self.render(f))`. The closure captures `&mut self`. If `render()` calls methods that need `&mut self.some_field` while the closure also holds `&self`, the borrow checker rejects the code. This manifests as a hard-to-fix cycle: render needs to read shared state AND write to `rendered_areas`, which requires `&mut`.

**Why it happens:**
ratatui's `terminal.draw()` takes `FnOnce(&mut Frame)`. If render is a `&mut self` method, the borrow is `&mut self` for the entire draw call. Moving `rendered_areas` writes inside the closure hits "cannot borrow `self` as mutable more than once" if anything else holds a borrow.

**How to avoid:**
Treat the Model/View split as the primary fix: `render()` takes `&self` (read-only), populates a `RenderedAreas` that is returned from `render()` and stored afterward:

```rust
// event_loop becomes:
let areas = self.render(frame);  // &self, returns RenderedAreas
self.rendered_areas = areas;      // &mut self, after draw closure exits
```

This pattern avoids the borrow conflict. `RenderedAreas` is rebuilt every frame — no partial-update bugs.

**Warning signs:**
- Compiler error: "cannot borrow `self` as mutable because it is also borrowed as immutable"
- `RefCell<RenderedAreas>` used to work around borrow check (a code smell)
- Render functions that take `&mut App` instead of `&App`

**Phase to address:** Model/View Architecture phase — define the `render(&self) -> RenderedAreas` signature contract before any widget refactors.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Hard-code screen geometry constants (e.g., `MENU_Y = 5`) instead of tracking rendered rects | Fast initial mouse support | Breaks silently on terminal resize or layout changes; every resize is a regression | Never — tracking rendered areas is a one-time investment |
| Parse CBOR manually with index arithmetic instead of using `ciborium` or `serde_cbor` | Avoid dependency | Fragile against optional fields; FIDO2 getInfo has 21 optional keys | Never for CTAP2 responses — use a CBOR crate |
| `sleep(Duration::from_millis(N))` as a poll substitute in tmux tests | Tests pass on developer machine | CI flakiness across platforms; Windows/Linux timing differs by 3-5x | Only as a last resort with values ≥ 500ms; prefer content polling |
| Sharing `App` struct reference into render closures via `unsafe` or `RefCell` | Sidesteps borrow issues quickly | Makes Model/View split impossible later; subtle aliasing UB risk | Never |
| Adding ratatui `ListState` to `OathCredential` struct | Convenient scrolling in OATH screen | Couples model to renderer; blocks Tauri migration | Never |
| Using `ykman` binary fallback for FIDO2 "just in case" | Avoids CTAP2 implementation complexity | Violates the project's core constraint; breaks on clean systems | Never — this project explicitly forbids ykman |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| OATH applet SELECT | Ignore `0x74` (challenge) tag in response — proceed to LIST | Check for `0x74` presence; if found, run VALIDATE before any command |
| OATH CALCULATE_ALL | Assume all entries have a code response value | Parse response tag: `0x77` and `0x7c` mean "no code yet" — display placeholder |
| OATH CALCULATE_ALL | Ignore `0x61xx` response | Issue SEND REMAINING (`0xa5`) in a loop until `0x9000` |
| FIDO2 getInfo | Decode raw response bytes as CBOR directly | Strip first byte (CTAP2 status code) before CBOR decoding |
| FIDO2 clientPin | Send PIN as UTF-8 bytes without length check | Max PIN length is 63 bytes (UTF-8); enforce in UI before APDU |
| OTP slot write | Skip access code input for existing configurations | Always prompt for access code when slot status shows existing config |
| Mouse coordinate check | Use `col == x && row == y` for button hit | Use `Rect::contains(Position { x: col, y: row })` |
| tmux tests | `send-keys` then immediately `capture-pane` | Poll for expected text with a timeout loop (`capture-pane -p | grep -q "text"`) |
| Multi-applet CCID | Connect/disconnect card for each applet switch | Maintain one `pcsc::Card` handle; only re-SELECT the AID |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Calling `YubiKeyState::detect_all()` on every event loop tick | 100ms+ input lag; card busy errors from repeated CONNECT/DISCONNECT | Cache state; refresh only on Tab press, explicit reload, or connection change event | Immediately on first use |
| OATH credential refresh on every render frame | Card busy errors; OATH screen feels sluggish; touch credentials trigger infinitely | Refresh on screen entry or explicit user action, not in the draw loop | First render |
| Holding `pcsc::Card` open across the full app lifetime | Blocks gpg from card access; triggers scdaemon conflicts | Connect exclusively per-operation, disconnect immediately after; never store `Card` in `App` | Whenever user runs `gpg --card-status` in parallel |
| CALCULATE_ALL for every screen render tick | Each call is a full card round-trip (~50ms) | Compute once on OATH screen entry, cache result, show "Refresh" button | First use with > 5 credentials |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Logging the OATH credential name list | Exposes which 2FA accounts the user has configured on the key | Never log OATH credential names or TOTP codes; treat credential metadata as sensitive |
| Displaying a TOTP code after its 30-second window expires without UI indication | User copies an expired code, authentication fails, user confused | Show a countdown timer per code; auto-clear or dim expired codes |
| Caching OATH access key (PBKDF2 derivation) in heap memory after VALIDATE | Key material resident in memory longer than necessary | Zeroize after VALIDATE handshake completes; use `zeroize` crate |
| Confirming OTP slot write without showing what will be written | User accidentally overwrites slot 1 Yubico OTP that grants YubiCloud access | Show full diff (old config vs new config) before write confirmation |
| FIDO2 PIN input logged at debug level | PIN material in log files | Never log FIDO2 PIN input; existing yubitui no-sensitive-logs policy extends to PIN |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| TOTP screen shows codes without countdown timer | User cannot tell if code is about to expire; submits a stale code | Show per-credential progress bar or countdown (seconds remaining in current 30s window) |
| OATH password prompt appears as a modal with no context | User confused why a password is needed for "TOTP" | Explain: "This YubiKey's OATH app is protected. Enter the password you set in Yubico Authenticator." |
| OTP slot 1 appears editable without warning that it has YubiCloud OTP | User overwrites factory-registered Yubico OTP and loses YubiCloud 2FA | Detect slot 1 as "Yubico OTP (factory configured)" and show a prominent warning before any edit |
| Mouse click that dismisses a popup also activates the background widget | User experience feels "broken" — accidental actions | Enforce Z-order in event handler (Pitfall 2) |
| FIDO2 PIN change succeeds but UI shows old PIN retry count | User distrust; thinks operation failed | Refresh FIDO2 status from device after every PIN operation |
| Onboarding flow that detects no YubiKey and shows a generic error | New user plugs in YubiKey but gets "No YubiKey found" | Check pcscd/WinSCard service first; provide platform-specific "start pcscd" or "check USB" guidance |

---

## "Looks Done But Isn't" Checklist

- [ ] **OATH CALCULATE_ALL:** Handles `0x61xx` pagination — verify with a key having > 10 credentials
- [ ] **OATH VALIDATE:** Tested against a key with a password set, not just passwordless keys
- [ ] **TOTP display:** Shows countdown timer — verifying code is still valid at display time
- [ ] **FIDO2 getInfo:** Handles optional CBOR keys gracefully — do not panic if key 15 absent
- [ ] **OTP slot write:** Write-then-verify pattern implemented — re-reads config after write
- [ ] **Mouse Z-order:** Click on popup does not fire background action — test by clicking center of context menu overlay
- [ ] **RenderedAreas tracking:** All scrollable/clickable widgets register their rects — verify by resizing terminal to 80x24 and clicking each widget
- [ ] **Model purity:** `cargo grep 'use ratatui' src/yubikey/ src/model/'` returns zero results
- [ ] **Windows mouse:** Mouse events tested in Windows Terminal (ConPTY) — if not supported, degrade gracefully with keyboard fallback rather than blank screen
- [ ] **tmux tests:** Test suite passes 5 consecutive times without hardware changes (flakiness check)
- [ ] **OATH credential 64-byte name limit:** UI input field enforces max 64 bytes before PUT APDU

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| ratatui types leaked into model | HIGH | Extract all ratatui imports from yubikey/ modules; introduce adapter/mapping layer in ui/; fix all callers |
| OATH VALIDATE not implemented, all OATH ops return 0x6982 | MEDIUM | Add VALIDATE handshake before any OATH command dispatch; add password prompt widget |
| Broken mouse Z-order causing double-action bugs | MEDIUM | Introduce event-consumed flag or early-return pattern in handle_mouse_event; no structural change |
| TOTP codes always wrong (no timestamp challenge) | LOW | Fix CALCULATE APDU construction to include current timestep as 8-byte BE challenge |
| OTP slot 1 overwritten (user lost Yubico OTP) | HIGH (user data) | No software recovery — Yubico OTP re-registration required via YubiCloud portal; document this in the confirmation dialog |
| tmux tests flaky in CI | MEDIUM | Replace fixed `sleep` with poll loops; add `--retries 3` flag to test runner |
| FIDO2 CBOR parse panics on unexpected response | MEDIUM | Wrap CBOR decode in `Result`, surface as "FIDO2 not supported on this firmware" with raw bytes in debug log |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| ratatui types in model (Pitfall 3) | Phase 1: Model/View Architecture | `cargo grep 'use ratatui' src/yubikey/'` in CI returns zero |
| Mouse coordinate hit-testing (Pitfall 1) | Phase 1: Model/View Architecture (RenderedAreas struct) | Click each widget at terminal widths 80, 120, 200 |
| Mouse Z-order click passthrough (Pitfall 2) | Phase 2: Mouse Support | Click center of context menu while it is open; no background action fires |
| Borrow checker in render refactor (Pitfall 10) | Phase 1: Model/View Architecture | `render()` takes `&self`; no `RefCell` in RenderedAreas path |
| OATH timestamp challenge (Pitfall 4) | Phase 3: OATH/TOTP Feature | Generated codes validate against Google Authenticator for same credential |
| OATH password-protected keys (Pitfall 5) | Phase 3: OATH/TOTP Feature | Test against a key with OATH password set |
| OATH CALCULATE_ALL incomplete data (Pitfall 6) | Phase 3: OATH/TOTP Feature | Test with HOTP credential in list; test with > 10 credentials |
| FIDO2 CTAP2 double-wrapped APDU (Pitfall 7) | Phase 4: FIDO2 Feature | getInfo APDU returns valid CBOR with `versions` field |
| OTP access code / slot 1 overwrite (Pitfall 8) | Phase 5: OTP Slot Management | Test against slot 1 with access code set; write must fail cleanly with prompt |
| scdaemon card-busy race (Pitfall 9) | Phase 0: Tech Debt (50ms sleep) | Run OATH LIST 20 times in a loop while `gpg --card-status` runs in parallel; zero failures |
| tmux test flakiness | Phase 2 (E2E test suite setup) | CI passes 5 consecutive runs on Linux and macOS without hardware changes |
| Windows ConPTY mouse absence (cross-platform) | Phase 2: Mouse Support | Windows CI run shows keyboard fallback active; no crash or blank screen |

---

## Sources

- [YKOATH Protocol — Yubico Developer Docs](https://developers.yubico.com/OATH/YKOATH_Protocol.html) — HIGH confidence (official spec)
- [OATH Commands and APDUs — Yubico SDK Manual](https://docs.yubico.com/yesdk/users-manual/application-oath/oath-commands.html) — HIGH confidence
- [FIDO2 getInfo APDU — Yubico SDK Manual](https://docs.yubico.com/yesdk/users-manual/application-fido2/apdu/get-info.html) — HIGH confidence
- [OTP Slot Access Codes — Yubico SDK Manual](https://docs.yubico.com/yesdk/users-manual/application-otp/how-to-slot-access-codes.html) — HIGH confidence
- [OTP Slots Overview — Yubico SDK Manual](https://docs.yubico.com/yesdk/users-manual/application-otp/slots.html) — HIGH confidence
- [ratatui Mouse Capture — ratatui.rs docs](https://ratatui.rs/concepts/backends/mouse-capture/) — MEDIUM confidence (stub page; confirmed by crossterm issues)
- [ratatui Discussion #220: Best Practices](https://github.com/ratatui/ratatui/discussions/220) — MEDIUM confidence (community, corroborated by official patterns)
- [ratatui Discussion #1051: Mouse hit-testing on Rect](https://github.com/ratatui/ratatui/discussions/1051) — MEDIUM confidence (community, describes documented gap)
- [crossterm Issue #446: Windows Terminal mouse events](https://github.com/crossterm-rs/crossterm/issues/446) — HIGH confidence (upstream issue; ConPTY limitation confirmed by Microsoft)
- [Windows Terminal Issue #376: ConPTY mouse input](https://github.com/microsoft/terminal/issues/376) — HIGH confidence (Microsoft developer confirmed ConPTY does not transit mouse reports)
- [CTAP-bridge: FIDO2 PC/SC CTAPHID Bridge](https://github.com/StarGate01/CTAP-bridge) — MEDIUM confidence (reference implementation)
- [tmux send-keys race condition with shell init](https://github.com/anthropics/claude-code/issues/23513) — MEDIUM confidence (real-world documented race)
- [yubitui PROJECT.md — tech debt items](/.planning/PROJECT.md) — HIGH confidence (project's own documented debt)

---
*Pitfalls research for: yubitui v1.1 — ratatui TUI + YubiKey protocol extensions*
*Researched: 2026-03-26*
