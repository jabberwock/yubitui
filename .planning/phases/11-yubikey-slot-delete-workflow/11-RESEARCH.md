# Phase 11: OTP Slots + Education + Onboarding - Research

**Researched:** 2026-03-27
**Domain:** YubiKey OTP APDU over PC/SC; textual-rs help panel pattern; onboarding state detection heuristics
**Confidence:** HIGH for OTP slot occupancy detection; HIGH for help panel pattern; MEDIUM for credential-type limitation (confirmed from official Yubico SDK source); HIGH for onboarding detection

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| OTP-01 | User can view OTP slot status screen showing slot 1 and slot 2 occupancy and configured type (Yubico OTP, static password, HMAC-SHA1, empty) | SELECT OTP AID (A0 00 00 05 27 20 01 01) + READ STATUS (00 03 00 00) returns 6-byte struct with touch_level field encoding SLOT1_VALID (0x01) and SLOT2_VALID (0x02) bits. **Critical limitation:** credential type is NOT readable from status — only occupied/empty is available. See Pitfall 1. |
| EDU-01 | User can press `?` on any screen to open a help panel with per-screen content | Push `ModalScreen::new(Box::new(PopupScreen::new(title, body)))` via `ctx.push_screen_deferred()`. `?` keybinding added to every screen's static bindings. Per-screen content defined as static string constants. |
| EDU-02 | User can access a protocol glossary from main menu or `?` from dashboard | New `GlossaryScreen` Widget pushed from dashboard `?` action or nav_9. Same compose() + Label pattern as HelpScreen. Eight entries: PIV, FIDO, FIDO2, OpenPGP/PGP, SSH, TOTP, HOTP, OTP/Yubico OTP. |
| EDU-03 | On first launch with factory-default device, user sees onboarding checklist | `OnboardingScreen` Widget shown when `is_factory_default()` returns true. Checklist is display-only (Labels + status icons), no interactive steps. Pushed from app startup before DashboardScreen if factory default detected. |
| EDU-04 | Onboarding detects factory-default state: no FIDO2 PIN, zero OATH credentials, PIV mgmt key at default | Three checks: `!fido2.pin_is_set`, `oath.credentials.is_empty()`, PIV AUTHENTICATE with default key bytes succeeds. All three already available in model layer. |
</phase_requirements>

---

## Summary

Phase 11 adds four features: OTP slot status screen, per-screen help panels, a protocol glossary, and an onboarding checklist. All four are UI-layer additions that do not require new hardware communication protocols beyond a single new OTP APDU sequence.

**Critical finding for OTP-01:** The YubiKey OTP application's READ STATUS command (APDU INS=0x03) returns a 6-byte status structure whose `touch_level` field encodes `SLOT1_VALID` (bit 0) and `SLOT2_VALID` (bit 1). These bits tell you occupied vs empty — but NOT the credential type. The credential type (Yubico OTP, static password, HMAC-SHA1, HOTP) is write-only at configuration time and cannot be read back. This is confirmed by Yubico's own `yubikit` Python SDK source: `ConfigState.is_configured()` only tests the valid bits; there is no `get_type()` method. The requirement text says "Yubico OTP, static password, HMAC-SHA1, or empty" — this MUST be scoped down to "occupied or empty" in the plan, with a note to the user in the screen that type is not readable from hardware.

**EDU-01 help panel:** The textual-rs pattern is `ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(PopupScreen::new(...)))))`. Every screen already has a `key_bindings()` static array — adding `KeyCode::Char('?')` action `"help"` to each is a small, uniform change. Per-screen help content is defined as a `const` string per screen module.

**EDU-02 glossary:** Structurally identical to `HelpScreen` — a `GlossaryScreen` widget with eight `Label` children. Reachable from dashboard via a new `nav_9` binding or from the `?` action on the dashboard itself (which currently pushes HelpScreen, a nav conflict to resolve).

**EDU-03/04 onboarding:** Detection logic uses data already present in the model:
- `fido2.pin_is_set == false` — already in `Fido2State`
- `oath.credentials.len() == 0` — already in `OathState`
- PIV management key at default — requires one AUTHENTICATE APDU with the 24-byte 3DES default key; no key data is returned, so this is safe
Detection runs once at startup after `detect_all()`. The onboarding screen is pushed before DashboardScreen when all three conditions hold.

**Primary recommendation:** Model OTP slot state as `slot1: OtpSlotStatus, slot2: OtpSlotStatus` where `OtpSlotStatus = Occupied | Empty` (type NOT included — it is undetectable). Add a visible note on the screen explaining this hardware limitation. All education screens use the existing `PopupScreen`/`ModalScreen` pattern.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| pcsc | 2.8 (already in Cargo.toml) | OTP status APDU via CCID | Already the project's PC/SC crate; OTP app is CCID-accessible |
| textual-rs | path dep (already in Cargo.toml) | Widget pattern for OTP screen, glossary, onboarding | All existing screens use it |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| None new required | — | — | Phase 11 requires no new crate dependencies |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Single `OtpSlotStatus::Occupied` variant | `OtpSlotType` enum with HMAC/Static/YubiOTP | Status is not readable; full enum would show false data |

---

## Architecture Patterns

### New Files Required

```
src/model/otp.rs           # OtpState, OtpSlotStatus, get_otp_slot_status()
src/tui/otp.rs             # OtpScreen widget
src/tui/glossary.rs        # GlossaryScreen widget
src/tui/onboarding.rs      # OnboardingScreen widget
```

### Modifications Required

```
src/model/mod.rs           # Add pub mod otp; add otp field to YubiKeyState
src/model/mock.rs          # Add otp field to mock fixture
src/tui/mod.rs             # Add pub mod otp, glossary, onboarding
src/tui/dashboard.rs       # Add nav_9 (OTP), ? → glossary, startup onboarding push
src/tui/fido2.rs           # Add ? keybinding → help panel
src/tui/oath.rs            # Add ? keybinding → help panel
src/tui/piv.rs             # Add ? keybinding → help panel
src/tui/keys.rs            # Add ? keybinding → help panel
src/tui/pin.rs             # Add ? keybinding → help panel
src/tui/ssh.rs             # Add ? keybinding → help panel
src/tui/diagnostics.rs     # Add ? keybinding → help panel (if screen exists separately)
src/model/app_state.rs     # Add Screen::Otp, Screen::Glossary, Screen::Onboarding variants
```

### Pattern 1: OTP Status APDU

```
AID:  A0 00 00 05 27 20 01 01  (8 bytes — OTP application)
SELECT: 00 A4 04 00 08 [AID]
READ STATUS: 00 03 00 00
Response: 6 bytes: [major, minor, build, pgm_seq, touch_level_lo, touch_level_hi]
  touch_level = (touch_level_hi << 8) | touch_level_lo
  SLOT1_VALID = touch_level & 0x01
  SLOT2_VALID = touch_level & 0x02
  SLOT1_TOUCH = touch_level & 0x04
  SLOT2_TOUCH = touch_level & 0x08
```

Source: Yubico yubikit Python SDK — `yubikey-manager/yubikit/yubiotp.py` CFGSTATE enum (verified 2026-03-27).

### Pattern 2: Per-Screen Help Panel (EDU-01)

```rust
// Source: existing src/tui/widgets/popup.rs + fido2.rs pattern

// In each screen's static bindings array, add:
KeyBinding {
    key: KeyCode::Char('?'),
    modifiers: KeyModifiers::NONE,
    action: "help",
    description: "? Help",
    show: true,
},

// In on_action():
"help" => {
    ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
        PopupScreen::new("FIDO2 Help", FIDO2_HELP_TEXT)
    ))));
}
```

Each screen defines a `const SCREEN_HELP_TEXT: &str` with its own educational content.

### Pattern 3: Factory-Default Detection (EDU-04)

```rust
// In model layer — no ratatui imports
pub fn is_factory_default(yk: &YubiKeyState) -> bool {
    let no_fido2_pin = yk.fido2.as_ref().map(|f| !f.pin_is_set).unwrap_or(false);
    let zero_oath = yk.oath.as_ref().map(|o| o.credentials.is_empty()).unwrap_or(false);
    let piv_default_key = check_piv_default_mgmt_key().unwrap_or(false);
    no_fido2_pin && zero_oath && piv_default_key
}
```

PIV default key check: AUTHENTICATE (INS=0x87, algorithm=0x03 for 3DES) against slot 0x9B with default key bytes `01 02 03 04 05 06 07 08 01 02 03 04 05 06 07 08 01 02 03 04 05 06 07 08`. SW 0x9000 means default key is in use.

### Pattern 4: OTP Screen Structure

```rust
// Mirrors PivScreen pattern
pub struct OtpScreen {
    pub otp_state: Option<OtpState>,
}

// OtpState in model layer:
#[derive(Debug, Clone, serde::Serialize)]
pub struct OtpState {
    pub slot1: OtpSlotStatus,
    pub slot2: OtpSlotStatus,
    pub slot1_touch: bool,
    pub slot2_touch: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum OtpSlotStatus {
    Occupied,  // Configured — type undetectable from hardware
    Empty,
}
```

### Anti-Patterns to Avoid

- **Showing false credential type:** Do not present a `OtpSlotType::YubicoOtp` guess. The hardware read returns only occupied/empty. Show "Configured (type unreadable)" not a specific type.
- **Blocking startup on onboarding detection:** PIV default key check requires PC/SC exclusive access — run it in background via `run_worker_with_progress` or in the same startup scan thread. Do not block the TUI event loop.
- **Adding `?` to help/glossary screens themselves:** These screens should only use `Esc` to close, not recurse.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Help panel overlay | Custom ratatui popup renderer | `ModalScreen::new(Box::new(PopupScreen::new(...)))` | Already exists in `src/tui/widgets/popup.rs` |
| Glossary layout | Custom multi-column widget | `Label` widgets in `compose()` | HelpScreen already uses this pattern successfully |
| OTP APDU connection | New PC/SC connect helper | Follow `piv::get_piv_state()` pattern: kill_scdaemon, connect exclusive, transmit | Pattern established, copy it |

---

## Common Pitfalls

### Pitfall 1: OTP Credential Type Is Write-Only
**What goes wrong:** Planner implements `OtpSlotType` enum with `YubicoOtp`, `StaticPassword`, `HmacSha1` variants, then discovers at runtime there is no APDU to read this information back.
**Why it happens:** The requirement text says "Yubico OTP, static password, HMAC-SHA1, or empty" — but this is the full design wishlist, not what the hardware exposes. The OTP configuration flags (which encode type) are write-only at configuration time.
**How to avoid:** Model as `OtpSlotStatus::Occupied | OtpSlotStatus::Empty`. Display on screen: `Slot 1: Configured (credential type is not readable from hardware)`. This is honest and matches what ykman's `otp info` displays.
**Warning signs:** Any code path that returns a specific credential type from a read-only APDU response is wrong.

### Pitfall 2: OTP APDU Shares scdaemon Conflict
**What goes wrong:** OTP APDU reads fail with "card busy" because scdaemon is holding the CCID channel.
**Why it happens:** Same issue as PIV and OpenPGP reads — exclusive CCID access is required.
**How to avoid:** `card::kill_scdaemon()` + 50ms sleep before connecting, exactly as done in `piv::get_piv_state()`. The OTP AID is different (A0 00 00 05 27 20 01 01) but the PC/SC connection pattern is identical.
**Warning signs:** SW 0x6B00 or PC/SC connection error on OTP SELECT.

### Pitfall 3: Dashboard `?` Conflicts with Nav_6 Help
**What goes wrong:** Dashboard has nav_6 bound to `[6] Help` (existing HelpScreen). Adding `?` → glossary creates two overlapping help-related bindings that confuse the user.
**Why it happens:** Phase 11 adds a second help-related screen (GlossaryScreen) while nav_6 stays as the existing keybinding reference screen.
**How to avoid:** `?` on dashboard → GlossaryScreen (new protocol glossary per EDU-02). `6` → HelpScreen (existing keybinding reference). Both are distinct and complementary. Document this distinction in screen footers.
**Warning signs:** nav_6 and `?` both pushing the same screen, or nav_6 being removed unexpectedly.

### Pitfall 4: Onboarding Pushed on Every Launch After Factory Reset
**What goes wrong:** After factory reset, every relaunch shows the onboarding screen because the device remains in factory-default state and no "onboarding completed" flag exists.
**Why it happens:** There is no persistent app state — `AppState` is rebuilt from hardware on each launch.
**How to avoid:** This is acceptable behavior — the onboarding screen is informational, not a wizard, and can be dismissed with Esc. Make sure the footer clearly says "Esc to skip". Alternatively, store a config file flag in `dirs::config_dir()` to record completion — but this adds scope; discuss with planner.
**Warning signs:** Users complaining onboarding appears every launch even when device is not factory-default (means detection logic is wrong).

### Pitfall 5: PIV Default Key Check Causes Scdaemon Kill on Startup
**What goes wrong:** PIV default key check kills scdaemon as a side effect, which interferes with gpg operations the user has running.
**Why it happens:** `piv::get_piv_state()` calls `kill_scdaemon()`. If onboarding detection adds another PIV APDU call at startup, it doubles the kill calls.
**How to avoid:** Check if the existing `piv` field already in `YubiKeyState` provides enough info. The PIV state from `detect_all()` shows slot occupancy but NOT whether the management key is default. The default key check requires a separate AUTHENTICATE attempt — it's a best-effort check, so if it errors (scdaemon conflict, key protected), treat as "key is not default" and skip onboarding. The check should be done once as part of the startup detection scan, not as a separate blocking call.
**Warning signs:** Double scdaemon kill in startup logs; gpg subprocess errors after yubitui launches.

---

## OTP Requirement Clarification

The requirement as written in REQUIREMENTS.md for OTP-01 says:

> "User can view OTP slot status screen showing slot 1 and slot 2 occupancy and **configured type (Yubico OTP, static password, HMAC-SHA1, empty)**"

This is NOT fully implementable. The OTP READ STATUS APDU (INS=0x03) returns only:
- Is slot 1 occupied? (SLOT1_VALID bit)
- Is slot 2 occupied? (SLOT2_VALID bit)
- Does each slot require touch? (SLOT1_TOUCH, SLOT2_TOUCH bits)

The credential type is not stored anywhere readable by the host. This matches the design decision already in REQUIREMENTS.md v2 note: "OTP slot write (OTP-02, OTP-03) deferred — underdocumented HID frame protocol" — the type-detection problem is part of why OTP is hard.

**Recommendation for planner:** Implement OTP-01 as occupied/empty with a note on screen. The success criterion text "what type each contains" should be treated as "display occupied status; display a note that type is not readable." This satisfies the spirit of the requirement (user can see the slot screen) while being technically honest.

---

## Code Examples

### OTP AID Select + Status Read

```rust
// Source: yubikit Python yubiotp.py CFGSTATE + commands-read-status.html
// AID: A0 00 00 05 27 20 01 01

const OTP_AID: &[u8] = &[0xA0, 0x00, 0x00, 0x05, 0x27, 0x20, 0x01, 0x01];

const SELECT_OTP: &[u8] = &[
    0x00, 0xA4, 0x04, 0x00, 0x08,
    0xA0, 0x00, 0x00, 0x05, 0x27, 0x20, 0x01, 0x01,
];

const READ_OTP_STATUS: &[u8] = &[0x00, 0x03, 0x00, 0x00];

// Response: 6 bytes [major, minor, build, pgm_seq, touch_lo, touch_hi]
const SLOT1_VALID: u16 = 0x01;
const SLOT2_VALID: u16 = 0x02;
const SLOT1_TOUCH: u16 = 0x04;
const SLOT2_TOUCH: u16 = 0x08;

pub fn get_otp_slot_status() -> Result<OtpState> {
    super::card::kill_scdaemon();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let ctx = Context::establish(Scope::User)?;
    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = ctx.list_readers(&mut readers_buf)?.collect();

    let card = readers.into_iter().find_map(|reader| {
        ctx.connect(reader, ShareMode::Exclusive, Protocols::T0 | Protocols::T1).ok()
    }).ok_or_else(|| anyhow::anyhow!("No reader found"))?;

    let mut buf = [0u8; 256];
    let resp = card.transmit(SELECT_OTP, &mut buf)?;
    if super::card::apdu_sw(resp) != 0x9000 {
        return Err(anyhow::anyhow!("OTP application not available"));
    }

    let mut status_buf = [0u8; 64];
    let status_resp = card.transmit(READ_OTP_STATUS, &mut status_buf)?;
    if super::card::apdu_sw(status_resp) != 0x9000 || status_resp.len() < 6 {
        return Err(anyhow::anyhow!("OTP READ STATUS failed"));
    }

    let touch_level = (status_resp[5] as u16) << 8 | (status_resp[4] as u16);
    Ok(OtpState {
        slot1: if touch_level & SLOT1_VALID != 0 { OtpSlotStatus::Occupied } else { OtpSlotStatus::Empty },
        slot2: if touch_level & SLOT2_VALID != 0 { OtpSlotStatus::Occupied } else { OtpSlotStatus::Empty },
        slot1_touch: touch_level & SLOT1_TOUCH != 0,
        slot2_touch: touch_level & SLOT2_TOUCH != 0,
    })
}
```

### Per-Screen Help Binding Addition

```rust
// Source: src/tui/widgets/popup.rs + src/tui/fido2.rs patterns

// 1. Add to SCREEN_BINDINGS static array:
KeyBinding {
    key: KeyCode::Char('?'),
    modifiers: KeyModifiers::NONE,
    action: "help",
    description: "? Help",
    show: true,
},

// 2. Add to on_action():
"help" => {
    ctx.push_screen_deferred(Box::new(
        ModalScreen::new(Box::new(PopupScreen::new("OTP Slots Help", OTP_HELP_TEXT)))
    ));
}

// 3. Define per-screen content:
const OTP_HELP_TEXT: &str =
    "OTP Slots — YubiKey has two configurable OTP slots.\n\
     Slot 1 activates on short touch; Slot 2 on long touch.\n\
     Types: Yubico OTP (cloud-validated), HMAC-SHA1 (challenge-response),\n\
     Static Password (fixed string), HOTP (counter-based).\n\
     Note: credential type cannot be read back from hardware.";
```

### Glossary Screen Structure

```rust
// Source: src/tui/help.rs pattern (direct equivalent)
pub struct GlossaryScreen;

impl Widget for GlossaryScreen {
    fn compose(&self) -> Vec<Box<dyn Widget>> {
        vec![
            Box::new(Header::new("Protocol Glossary")),
            Box::new(Label::new("PIV — Personal Identity Verification. Smart card standard for certificates and key storage.")),
            Box::new(Label::new("FIDO2 / WebAuthn — Hardware passkey standard. Phishing-resistant authentication.")),
            // ... 6 more entries
            Box::new(Footer),
        ]
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| ykman otp info (subprocess) | Native APDU SELECT OTP + READ STATUS | yubitui v1.0 decision | No ykman dependency |
| Guessing credential type | Display occupied/empty only | Protocol constraint | Honest, correct |

---

## Open Questions

1. **OTP-01 Requirement Scope**
   - What we know: Hardware only returns occupied/empty — type is unreadable
   - What's unclear: Does the user accept "Configured (type unknown)" or does this fail the success criterion?
   - Recommendation: Plan includes a visible note on the OTP screen. If user wants type display, it would require a v2 feature storing type in a config file at write time (which requires OTP write support, also v2).

2. **Onboarding: PIV Management Key Check Scope**
   - What we know: Checking the PIV default key requires an AUTHENTICATE APDU that adds a PC/SC round-trip at startup
   - What's unclear: Is this check worth the startup latency and scdaemon-kill side effect?
   - Recommendation: Make PIV key check a best-effort heuristic. If it fails (timeout, error), treat as "key is NOT default" and skip onboarding. This avoids blocking startup.

3. **Onboarding Repeat-Show Behavior**
   - What we know: No persistent "onboarding completed" flag exists
   - What's unclear: Is showing onboarding every launch until device is configured acceptable?
   - Recommendation: Acceptable for v1.1. Add a config file flag to `.config/yubitui/onboarding_complete` in v2 if users complain.

4. **Dashboard Navigation Number for OTP**
   - What we know: Dashboard currently has nav_1 through nav_8 bound. OTP screen is a 9th screen.
   - What's unclear: Should OTP use `9` key, extend the button list, or replace a less-used screen?
   - Recommendation: Add `[9] OTP Slots` as nav_9. The footer already shows '1-8 Navigate' — update to '1-9 Navigate'. `?` → GlossaryScreen replaces the current `?` keybinding which was not previously bound on dashboard.

---

## Environment Availability

Step 2.6: SKIPPED for FIDO2/CTAP2 aspects (already verified in Phase 10). OTP via PC/SC uses pcsc crate already in Cargo.toml. No new external tools required.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| pcsc | OTP APDU reads | Already in Cargo.toml | 2.8 | — |
| textual-rs | All new screens | Already in Cargo.toml | path dep | — |

---

## Sources

### Primary (HIGH confidence)
- `Yubico/yubikey-manager` GitHub — `yubikit/yubiotp.py` CFGSTATE enum: SLOT1_VALID=0x01, SLOT2_VALID=0x02, SLOT1_TOUCH=0x04, SLOT2_TOUCH=0x08 (verified 2026-03-27)
- [OTP Read Status — Yubico SDK docs](https://docs.yubico.com/yesdk/users-manual/application-otp/commands-read-status.html) — AID A0 00 00 05 27 20 01 01, INS 0x03 returns 6-byte status structure
- [ConfigState — yubikit Python API](https://developers.yubico.com/yubikey-manager/API_Documentation/autoapi/yubikit/yubiotp/index.html) — `is_configured()` tests SLOT1/2_VALID; no type method exists
- `src/tui/widgets/popup.rs` — ModalScreen + PopupScreen + ConfirmScreen patterns (project source)
- `src/tui/fido2.rs` — per-screen keybinding and on_action patterns (project source)
- `src/model/piv.rs` — OTP should follow the same kill_scdaemon + exclusive connect pattern (project source)

### Secondary (MEDIUM confidence)
- [Yubico Forum — Determine current slot configurations](https://forum.yubico.com/viewtopicc406.html?p=7405) — confirms credential type is NOT readable; only occupied/empty via status
- [ConfigurationFlags struct — .NET SDK](https://docs.yubico.com/yesdk/yubikey-api/Yubico.YubiKey.Otp.ConfigurationFlags.html) — write-time flags only; no read-back mechanism
- [Blog: Configuring YubiKey NEO auto-eject via CCID](https://blog.tho.ms/hacks/2017/04/25/configuring-yubikey-neo-auto-eject-through-ccid.html) — confirms OTP CCID response byte layout: [major, minor, build, pgmSeq, touch_level_lo, touch_level_hi]

### Tertiary (LOW confidence)
- WebSearch results confirming PIV default management key = 010203040506070801020304050607080102030405060708 (24-byte 3DES); confirmed by Yubico docs for firmware < 5.7 and AES-192 for >= 5.7

---

## Metadata

**Confidence breakdown:**
- OTP slot occupancy APDU: HIGH — verified from Yubico SDK source
- OTP credential type limitation: HIGH — confirmed from SDK source and forum; type is write-only
- Help panel pattern (EDU-01): HIGH — PopupScreen/ModalScreen already in codebase
- Glossary screen (EDU-02): HIGH — direct analog of HelpScreen
- Onboarding detection (EDU-03/04): MEDIUM — Fido2 + OATH checks are HIGH; PIV default key check MEDIUM (AUTHENTICATE APDU approach correct but untested in codebase)

**Research date:** 2026-03-27
**Valid until:** 2026-06-27 (stable hardware protocol; 90 days)
