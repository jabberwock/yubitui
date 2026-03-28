# Phase 10: FIDO2 Screen - Research

**Researched:** 2026-03-27
**Domain:** CTAP2/HID protocol over `ctap-hid-fido2` crate; textual-rs widget pattern; reset timing workflow
**Confidence:** MEDIUM-HIGH (crate API verified at HIGH via docs.rs; reset gap confirmed; Windows behavior confirmed from crate README)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Researcher evaluates available Rust CTAP2 crates on stability and security grounds. No ykman. Native Rust library is fully acceptable.
- **D-02:** Researcher must spike credential enumeration (`credentialManagement` command) and credential deletion specifically. Device info and PIN ops are lower risk.
- **D-03:** Screen opens and shows device info immediately (no PIN required). Below info section, credentials are loaded via an inline PIN prompt.
- **D-04:** If no FIDO2 PIN configured yet, credential section shows "No PIN configured — press S to set one." Setting PIN flows directly into PIN entry then loads credentials automatically.
- **D-05:** If PIN set but user cancels/skips PIN entry, show "Credentials locked — press P to authenticate" as placeholder.
- **D-06:** Single `Fido2Screen` widget — no push_screen for sub-views. Layout: Header, device info section, passkeys section, footer.
- **D-07:** Follows PivScreen pattern exactly — `compose()` returns full widget tree, `on_action()` handles keybindings, model layer in `src/model/fido2.rs` with zero ratatui imports.
- **D-08:** Keybindings: `S` set/change PIN, `D` delete selected credential, `R` reset FIDO2 applet, `Esc` back to dashboard, Up/Down/j/k navigate credentials.
- **D-09:** Dashboard navigation: key `8` and "[8] FIDO2 / Security Key" button, following nav_7 OATH pattern.
- **D-10:** After irreversibility confirmation dialog, show a dedicated reset guidance screen with countdown, device reconnect polling, and outcome handling.
- **D-11:** The 10-second timing constraint and its reason must be explained clearly on the guidance screen.
- **D-12:** Windows admin privilege detection at point of FIDO2 operation attempt — inline message, no persistent banner. Claude's discretion on exact UX.

### Claude's Discretion

- Mock data structure for `--mock` mode (what fields `Fido2State` contains)
- Exact CBOR/CTAP2 command sequencing (researcher/planner will determine from crate API or spec)
- Error handling for card busy, timeout, and auth failure states

### Deferred Ideas (OUT OF SCOPE)

- Fingerprint management (FIDO-08) — Bio YubiKey only, deferred to v2
- Enable/disable YubiKey applications (FIDO-09) — requires Management Key auth, deferred to v2
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| FIDO-01 | User can view FIDO2 info screen: firmware version, supported algorithms, PIN status, PIN retry count | `get_info()` returns `Info` struct with `firmware_version`, `algorithms`, `options` (includes `clientPin` boolean); `get_pin_retries()` returns `i32` |
| FIDO-02 | User can set a FIDO2 PIN when none is configured | `set_new_pin(pin: &str)` on `FidoKeyHid` |
| FIDO-03 | User can change an existing FIDO2 PIN | `change_pin(current_pin: &str, new_pin: &str)` on `FidoKeyHid` |
| FIDO-04 | User can view list of resident FIDO2 credentials (passkeys) | `credential_management_enumerate_rps()` + `credential_management_enumerate_credentials()` — HIGH confidence (verified at docs.rs) |
| FIDO-05 | User can delete a specific resident credential with confirmation dialog | `credential_management_delete_credential(pin, pkcd: PublicKeyCredentialDescriptor)` — HIGH confidence |
| FIDO-06 | User can reset FIDO2 applet with warning and 10s timing window | Reset NOT in ctap-hid-fido2 — must hand-roll raw CTAP HID frame for command 0x07. See Pitfall 1. |
| FIDO-07 | On Windows, user sees a clear message when FIDO2 operations require admin privileges | Detect via `Err` on `FidoKeyHidFactory::create()` containing "access" or via OS gate; Windows HID requires admin per crate README |
</phase_requirements>

---

## Summary

The `ctap-hid-fido2` crate (v3.5.9, actively maintained as of 2026-03-15) is the right choice for this phase. It is the only mature Rust CTAP2 crate with stable credential management support. The `FidoKeyHid` struct exposes all needed operations directly: `get_info()`, `get_pin_retries()`, `set_new_pin()`, `change_pin()`, and the full `credential_management_*` family. These are all HIGH confidence — verified via docs.rs and the crate source on 2026-03-27.

The single critical gap: **`ctap-hid-fido2` does not expose `authenticatorReset` (command 0x07)**. The ctapcli tool does not expose it either, and the source tree confirms no reset module exists. The reset must be implemented by hand as a raw CTAPHID_CBOR frame. This is a compact implementation — the CTAP HID reset frame is a single CBOR payload of `{1: 0x07}` sent over the hidapi channel the crate already opens — but it requires wrapping `hidapi` directly for this one operation. The 10-second timing window is a hard CTAP2 spec requirement: the device must receive the reset command within 10 seconds of power-on (USB plug-in). The typical pattern is: send reset, receive `CTAP2_ERR_NOT_ALLOWED` (0x2E), instruct user to unplug/replug, poll for device reconnect, then resend.

For the reset countdown UX (D-10), `textual-rs`'s `run_worker_with_progress` API is the correct mechanism: spawn a worker that loops with `tokio::time::sleep(Duration::from_secs(1))` sending progress ticks, and the widget receives `WorkerProgress<u8>` messages that drive the countdown display.

**Primary recommendation:** Use `ctap-hid-fido2 = "3.5.9"` for all CTAP2 operations; hand-roll the reset CTAPHID_CBOR frame using the `hidapi` crate (already a transitive dependency of ctap-hid-fido2) for command 0x07.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ctap-hid-fido2 | 3.5.9 | CTAP2 over HID: info, PIN, credential management | Only mature Rust CTAP2 crate with credentialManagement; actively maintained |
| hidapi | (transitive dep of ctap-hid-fido2) | Raw HID access for authenticatorReset | ctap-hid-fido2 already pulls it in; reuse for reset frame |
| chrono | 0.4 (already in Cargo.toml) | Elapsed-time calculation in reset countdown | Already used by OATH screen for timestamp |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tokio | 1.40 (already in Cargo.toml) | Async worker for reset countdown polling | run_worker_with_progress for 1s tick loop |
| sha2 | 0.10 (already in Cargo.toml) | SHA-256 rpid_hash computation for enumerate_credentials | rpid_hash parameter is SHA-256(rp.id.as_bytes()) |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| ctap-hid-fido2 | webauthn-authenticator-rs | Much heavier crate, browser-oriented, not hardware management |
| ctap-hid-fido2 | ArdaXi/ctap (unmaintained) | Last commit 2019; no credentialManagement support |
| ctap-hid-fido2 | fido-key-manager | CLI tool wrapper over ctap-hid-fido2 anyway |
| run_worker_with_progress | std::thread + channel | Would work but not idiomatic in textual-rs; workers auto-cancel on unmount |

**Installation:**
```bash
# Add to Cargo.toml [dependencies]:
ctap-hid-fido2 = "3.5.9"
# hidapi is already a transitive dep — no direct addition needed unless you
# need to call hidapi::HidApi directly for the reset frame.
# If direct access is needed:
# hidapi = "2"
```

**Version verification:** Confirmed `3.5.9` via `cargo search ctap-hid-fido2` and crates.io API on 2026-03-27. Published 2026-03-15.

---

## Architecture Patterns

### Recommended Project Structure

```
src/
├── model/
│   └── fido2.rs          # Fido2State, Fido2Credential, Fido2PinStatus; get_fido2_state(),
│                         # set_pin(), change_pin(), enumerate_credentials(),
│                         # delete_credential(), reset_fido2() — zero ratatui imports
├── tui/
│   └── fido2.rs          # Fido2Screen, PinInputState, ResetGuidanceScreen,
│                         # DeleteCredentialScreen — all textual-rs widgets
└── model/mock.rs         # Extend mock_yubikey_states() with fido2: Some(Fido2State{...})
```

### Pattern 1: FidoKeyHid Connection

`FidoKeyHid` is constructed with `FidoKeyHidFactory::create(&LibCfg::init())`. This is a synchronous call. On Windows, failure here (with an "access" error message) indicates insufficient privileges. The connection is NOT kept alive across calls — create fresh per operation (matches how OATH/PIV do card connections).

```rust
// Source: docs.rs/ctap-hid-fido2/latest — FidoKeyHidFactory::create
use ctap_hid_fido2::{FidoKeyHidFactory, LibCfg};

pub fn get_fido2_device() -> anyhow::Result<ctap_hid_fido2::FidoKeyHid> {
    FidoKeyHidFactory::create(&LibCfg::init())
        .map_err(|e| anyhow::anyhow!("FIDO2 device error: {}", e))
}
```

### Pattern 2: get_info for FIDO-01

```rust
// Source: docs.rs/ctap-hid-fido2/latest — FidoKeyHid::get_info
let device = get_fido2_device()?;
let info = device.get_info()?;

// info.firmware_version: Option<u32>  — packed major/minor/patch
// info.algorithms: Vec<String>         — e.g. ["ES256", "EdDSA"]
// info.options: Vec<(String, bool)>    — includes ("clientPin", true/false)
// PIN set: info.options.iter().any(|(k,v)| k == "clientPin" && *v)
// PIN retries: separate call:
let retries = device.get_pin_retries()?;  // -> i32
```

### Pattern 3: Credential Enumeration (D-02 spike result)

Credential enumeration requires two steps: enumerate RPs, then for each RP enumerate credentials using the RP's `rpid_hash`.

```rust
// Source: docs.rs/ctap-hid-fido2/latest — credential_management_* methods
use ctap_hid_fido2::fidokey::credential_management::credential_management_params::{Rp, Credential};

let device = get_fido2_device()?;
let rps: Vec<Rp> = device.credential_management_enumerate_rps(Some(pin))?;
// Rp fields:
//   rp.public_key_credential_rp_entity.id: String   — "github.com"
//   rp.public_key_credential_rp_entity.name: String — "GitHub"
//   rp.rpid_hash: Vec<u8>                           — SHA-256 of rp_id (for next call)

let mut all_credentials: Vec<Credential> = Vec::new();
for rp in &rps {
    let creds = device.credential_management_enumerate_credentials(
        Some(pin),
        &rp.rpid_hash,
    )?;
    // Credential fields:
    //   cred.public_key_credential_user_entity — user display name, user id
    //   cred.public_key_credential_descriptor  — needed for delete
    all_credentials.extend(creds);
}
```

### Pattern 4: Delete Credential (FIDO-05)

```rust
// Source: docs.rs/ctap-hid-fido2/latest — credential_management_delete_credential
let pkcd = credential.public_key_credential_descriptor.clone();
device.credential_management_delete_credential(Some(pin), pkcd)?;
```

### Pattern 5: Set/Change PIN (FIDO-02, FIDO-03)

```rust
// Set new PIN (no existing PIN):
device.set_new_pin(new_pin)?;

// Change existing PIN:
device.change_pin(current_pin, new_pin)?;
```

### Pattern 6: authenticatorReset — Hand-Rolled CTAP HID Frame

`ctap-hid-fido2` does NOT expose reset. The reset must be sent as a raw CTAPHID_CBOR frame. The approach is:

1. Open the FIDO HID device via `hidapi::HidApi` (or reuse ctap-hid-fido2's exposed `HidParam` to find the device path).
2. Perform CTAPHID channel initialization (CTAPHID_INIT, send 8-byte nonce, receive 4-byte channel ID).
3. Send CTAPHID_CBOR with command byte `0x07` (authenticatorReset), no parameters.
4. Read response — `0x00` = success, `0x2E` (CTAP2_ERR_NOT_ALLOWED) = outside 10s window.

The alternative (simpler to implement for phase scope): use `ctap-hid-fido2`'s `FidoKeyHidFactory` to get device params, then separately call `hidapi` for the raw frame. This avoids duplicating HID enumeration logic.

**Protocol reference (verified):**
- CTAPHID_CBOR command byte: `0x10` (HID layer wrapping CTAP2)
- authenticatorReset command byte inside CBOR payload: `0x07`
- Timing: must arrive within 10 seconds of device power-on (USB insertion)
- Error `0x2E` (CTAP2_ERR_NOT_ALLOWED): window expired, user must replug

### Pattern 7: Reset Countdown UX with run_worker_with_progress

```rust
// Inside ResetGuidanceScreen::on_action("start_countdown", ctx):
// ctx.run_worker_with_progress(own_id, |progress_tx| {
//     Box::pin(async move {
//         for seconds_remaining in (0..=10).rev() {
//             let _ = progress_tx.send(seconds_remaining as u8);
//             if seconds_remaining > 0 {
//                 tokio::time::sleep(Duration::from_secs(1)).await;
//             }
//         }
//         // Return final status (expired)
//         ResetCountdownResult::Expired
//     })
// });
//
// In on_event: handle WorkerProgress<u8> to update displayed countdown
// In on_event: handle WorkerResult<ResetCountdownResult> for final state
```

Note: The device reconnect polling (when device is replugged, try the reset command) can be combined with the countdown loop — each second, attempt `get_fidokey_devices()` and if device is found AND counter is still within window, send the reset frame.

### Pattern 8: Windows Admin Privilege Detection

The `ctap-hid-fido2` README explicitly states: "security key via HID cannot be accessed unless the executing exe has administrator privileges" on Windows. Detection:

```rust
// Attempt device open — if it fails on Windows, check if the error
// message indicates access denial:
match FidoKeyHidFactory::create(&LibCfg::init()) {
    Err(e) => {
        let msg = e.to_string().to_lowercase();
        let is_access_denied = msg.contains("access") || msg.contains("permission");
        if cfg!(target_os = "windows") && is_access_denied {
            // surface FIDO-07 message
        }
    }
    Ok(device) => { /* proceed */ }
}
```

### Anti-Patterns to Avoid

- **Keeping FidoKeyHid alive across operations:** Do not store it in widget state. Create fresh per operation (matches OATH/PIV approach with fresh PC/SC connections each call).
- **Calling credential_management_* without PIN:** These calls require PIN authentication. Passing `None` will fail on YubiKeys with PIN set.
- **Computing rpid_hash manually when enumerate_rps already provides it:** The `Rp` struct from `enumerate_rps` includes `rpid_hash` — no need to SHA-256 the RP ID yourself.
- **Storing `PublicKeyCredentialDescriptor` as string:** It contains a credential ID byte vector. Store the whole struct (it implements Clone) or serialize with serde.
- **Blocking the TUI thread for the reset countdown:** Must use `run_worker_with_progress`, not `std::thread::sleep` in a widget method.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CTAP2 HID communication | Raw USB HID framing, CTAP2 packet assembly, channel management | `ctap-hid-fido2` | Complex multi-packet HID framing, CBOR encoding, CTAP2 error handling; crate has 3.5.x release history |
| CTAP2 PIN protocol | Elliptic-curve PIN hash, PIN token exchange | `ctap-hid-fido2` `set_new_pin()`/`change_pin()` | PIN protocol uses ECDH key exchange; non-trivial to implement correctly |
| RP ID hash | SHA-256(rp_id) | Use `rp.rpid_hash` from `enumerate_rps()` result | The crate returns it directly; no hashing needed |
| Credential list TUI state | Custom tree structure | `Vec<(Rp, Vec<Credential>)>` flattened into `Vec<Fido2Credential>` in Fido2State | Simple flat list sufficient for display; selection by index |

**Exception — must hand-roll:**
- `authenticatorReset` (0x07): No crate exposes this. Hand-roll a 3-step CTAPHID frame: INIT, then CTAPHID_CBOR{0x07}, then read response. This is ~60-80 lines of code using `hidapi` directly.

---

## Runtime State Inventory

> Phase 10 is a new screen addition (no rename/refactor). Runtime state inventory not applicable.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust/cargo | Build | ✓ | cargo 1.92.0 | — |
| ctap-hid-fido2 | FIDO2 HID operations | ✓ (to be added to Cargo.toml) | 3.5.9 | — |
| hidapi (C library) | ctap-hid-fido2 transitive dep on Linux | macOS/Windows: bundled | — | On Linux: `apt install libusb-1.0-0-dev libudev-dev` |
| Physical YubiKey | Device testing | hardware-gated | — | `--mock` flag for all TUI tests |

**Missing dependencies with no fallback:**
- None that block implementation. Reset requires a physical device for integration testing; mock covers all TUI tests.

**Missing dependencies with fallback:**
- Linux users need `libusb` system packages — documented in ctap-hid-fido2 README. Not a blocker for macOS dev machine.

---

## Common Pitfalls

### Pitfall 1: authenticatorReset Is Missing from ctap-hid-fido2
**What goes wrong:** Planning tasks that call `device.reset()` — this method does not exist. Builds fail.
**Why it happens:** The crate omits reset, likely because it's a destructive one-shot operation rarely needed in library clients.
**How to avoid:** Plan a dedicated Wave 0 or Wave 1 task: `src/model/fido2_reset.rs` — a raw HID frame sender using `hidapi::HidApi` that performs CTAPHID_INIT + CTAPHID_CBOR{0x07}. The reset function is small (~80 lines) and self-contained.
**Warning signs:** Any task description that says "call reset on FidoKeyHid" is wrong.

### Pitfall 2: Credential Enumeration Requires Two-Step Protocol
**What goes wrong:** Calling `credential_management_enumerate_credentials(pin, ???)` without first calling `enumerate_rps()` — the `rpid_hash` parameter is not a raw RP ID string, it's the SHA-256 hash returned by `enumerate_rps()`.
**Why it happens:** The API looks similar to a flat list but is actually a two-level hierarchy (RPs → credentials per RP).
**How to avoid:** Always call `enumerate_rps()` first, collect `Vec<Rp>`, then loop calling `enumerate_credentials(pin, &rp.rpid_hash)` for each RP.
**Warning signs:** Trying to compute rpid_hash from a string RP ID — unnecessary, the struct already contains it.

### Pitfall 3: get_info Does Not Separate PIN "Set" vs "Configured"
**What goes wrong:** `info.options` contains `("clientPin", bool)` where `true` means a PIN has been set. But this boolean is nested inside a `Vec<(String, bool)>`. Treating all options as flat fields will miss it.
**Why it happens:** The `Info` struct's options field is a generic key-value vector, not typed fields per option.
**How to avoid:** Parse explicitly: `info.options.iter().find(|(k,_)| k == "clientPin").map(|(_,v)| *v).unwrap_or(false)`.
**Warning signs:** Assuming `info.pin_set` or similar typed field exists — it does not.

### Pitfall 4: Reset 10-Second Window Timing
**What goes wrong:** Sending reset command before user unplugs/replugs (wrong order), or waiting too long after replug.
**Why it happens:** The CTAP2 spec requires the command arrive within 10 seconds of device power-on. "Power-on" means USB insertion. If the device is already plugged in, it must be physically removed and reinserted.
**How to avoid:** Implement D-10 exactly: after confirmation dialog, prompt user to unplug, start 10-second countdown, poll for device reconnect, send reset immediately on reconnect detection.
**Warning signs:** Attempting to send reset to an already-running device — will always return `CTAP2_ERR_NOT_ALLOWED`.

### Pitfall 5: YubiKeyState Has No fido2 Field Yet
**What goes wrong:** Accessing `yk.fido2` in model/mod.rs fails to compile — the field doesn't exist yet.
**Why it happens:** Phase 9 added `oath`, but FIDO2 state was not added to `YubiKeyState`.
**How to avoid:** Wave 0 of the plan must add `fido2: Option<Fido2State>` to `YubiKeyState` in `src/model/mod.rs` AND extend `mock_yubikey_states()` with fixture data. This must come before any TUI work.
**Warning signs:** Any plan that writes `Fido2Screen` before `YubiKeyState.fido2` is in model/mod.rs.

### Pitfall 6: credentialManagement Requires CTAP 2.1 Capability
**What goes wrong:** `credential_management_enumerate_rps()` returns an error on older YubiKeys that only support CTAP 2.0.
**Why it happens:** `authenticatorCredentialManagement` is a CTAP 2.1 extension. YubiKey 5 series supports it, but the `Info.options` map may show it under `"credMgmt"` or `"credentialMgmtPreview"` keys.
**How to avoid:** Before calling enumerate_rps, check `info.options` for `"credMgmt": true` OR `"credentialMgmtPreview": true`. If neither, surface a "Passkey management requires CTAP 2.1" message. In mock mode, add `credMgmt: true` to mock options.
**Warning signs:** Error from `enumerate_rps` on real hardware without checking capabilities first.

---

## Code Examples

Verified patterns from docs.rs and crate source:

### Complete Fido2State fetch (model layer)
```rust
// Source: docs.rs/ctap-hid-fido2/latest — verified 2026-03-27
use ctap_hid_fido2::{FidoKeyHidFactory, LibCfg};

pub fn get_fido2_state(pin: Option<&str>) -> anyhow::Result<Fido2State> {
    let device = FidoKeyHidFactory::create(&LibCfg::init())
        .map_err(|e| anyhow::anyhow!("FIDO2 device: {}", e))?;

    let info = device.get_info()?;
    let pin_retry_count = device.get_pin_retries()?;
    let pin_is_set = info.options.iter()
        .find(|(k, _)| k == "clientPin")
        .map(|(_, v)| *v)
        .unwrap_or(false);
    let supports_cred_mgmt = info.options.iter()
        .any(|(k, v)| (k == "credMgmt" || k == "credentialMgmtPreview") && *v);

    let algorithms: Vec<String> = info.algorithms.clone();
    let firmware_version = info.firmware_version;

    let credentials = if pin_is_set && supports_cred_mgmt {
        if let Some(pin) = pin {
            let rps = device.credential_management_enumerate_rps(Some(pin))?;
            let mut creds = Vec::new();
            for rp in &rps {
                let rp_creds = device.credential_management_enumerate_credentials(
                    Some(pin), &rp.rpid_hash
                )?;
                for c in rp_creds {
                    creds.push(Fido2Credential {
                        rp_id: rp.public_key_credential_rp_entity.id.clone(),
                        user_name: c.public_key_credential_user_entity.name.clone()
                            .unwrap_or_default(),
                        credential_id: c.public_key_credential_descriptor.id.clone(),
                    });
                }
            }
            Some(creds)
        } else {
            None // PIN required but not provided — show locked state
        }
    } else {
        Some(vec![]) // No PIN set, or no credMgmt support
    };

    Ok(Fido2State {
        firmware_version,
        algorithms,
        pin_is_set,
        pin_retry_count: pin_retry_count as u8,
        credentials,
        supports_cred_mgmt,
    })
}
```

### Fido2State model type
```rust
// src/model/fido2.rs — zero ratatui imports (INFRA-03/04)
#[derive(Debug, Clone, serde::Serialize)]
pub struct Fido2State {
    pub firmware_version: Option<u32>,   // from info.firmware_version
    pub algorithms: Vec<String>,          // e.g. ["ES256", "EdDSA"]
    pub pin_is_set: bool,                 // from options["clientPin"]
    pub pin_retry_count: u8,              // from get_pin_retries()
    pub credentials: Option<Vec<Fido2Credential>>, // None = locked (PIN not provided)
    pub supports_cred_mgmt: bool,         // from options["credMgmt"]
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Fido2Credential {
    pub rp_id: String,          // e.g. "github.com"
    pub user_name: String,      // user display name
    pub credential_id: Vec<u8>, // needed for delete
}
```

### Mock fixture extension
```rust
// src/model/mock.rs — extend YubiKeyState in mock_yubikey_states()
fido2: Some(fido2::Fido2State {
    firmware_version: Some(0x050403), // 5.4.3 packed
    algorithms: vec!["ES256".to_string(), "EdDSA".to_string()],
    pin_is_set: true,
    pin_retry_count: 8,
    credentials: Some(vec![
        fido2::Fido2Credential {
            rp_id: "github.com".to_string(),
            user_name: "user@example.com".to_string(),
            credential_id: vec![0x01, 0x02, 0x03],
        },
        fido2::Fido2Credential {
            rp_id: "google.com".to_string(),
            user_name: "user@gmail.com".to_string(),
            credential_id: vec![0x04, 0x05, 0x06],
        },
    ]),
    supports_cred_mgmt: true,
}),
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No credentialManagement in CTAP 2.0 | credentialManagement in CTAP 2.1 (YubiKey 5 firmware 5.2+) | ~2019 with CTAP 2.1 spec | Must gate credential list behind capability check |
| ykman CLI for FIDO2 ops | Native CTAP2 crate | This project's core constraint | All operations via ctap-hid-fido2 |
| Separate PIN token negotiation | ctap-hid-fido2 handles PIN token automatically via `pin: Option<&str>` | Since ctap-hid-fido2 v2.x | No manual PIN token exchange needed |

**Deprecated/outdated:**
- `fido-hid-rs`: Not found as a standalone crate — search returned no results. Likely never published or was internal.
- `ctap` (ArdaXi/ctap): Last commit 2019, unmaintained, no credentialManagement support.

---

## Open Questions

1. **hidapi version for direct reset frame access**
   - What we know: `hidapi` is a transitive dep of ctap-hid-fido2; exact version in ctap-hid-fido2's Cargo.toml not confirmed
   - What's unclear: Whether yubitui needs to declare `hidapi` as a direct dep (with matching version) or can access the transitive dep via ctap-hid-fido2's re-exports
   - Recommendation: In plan Wave 0, add `hidapi = "2"` as a direct dependency to avoid version mismatch. If ctap-hid-fido2 re-exports `HidApi`, use that.

2. **firmware_version format in Info struct**
   - What we know: `info.firmware_version` exists as `Option<u32>` (confirmed by source inspection)
   - What's unclear: Whether it's packed `(major << 16 | minor << 8 | patch)` or a raw version integer
   - Recommendation: Display as hex (e.g., `0x050403`) in the model and let the TUI format it as "5.4.3" by parsing major/minor/patch. Cross-check against `YubiKeyInfo.version` which is already known for the mock fixture.

3. **PublicKeyCredentialUserEntity fields**
   - What we know: Contains `name: Option<String>` (user name) and `id` (user handle bytes)
   - What's unclear: Whether `display_name: Option<String>` is a separate field
   - Recommendation: The `Fido2Credential.user_name` field in the model should use `user_entity.name.clone().unwrap_or_else(|| user_entity.display_name.clone().unwrap_or_default())` — defensive fallback chain.

---

## Sources

### Primary (HIGH confidence)
- docs.rs/ctap-hid-fido2/latest/ctap_hid_fido2/fidokey/struct.FidoKeyHid.html — full method list verified 2026-03-27
- raw.githubusercontent.com/gebogebogebo/ctap-hid-fido2/master/src/fidokey/credential_management/mod.rs — credential management function signatures
- raw.githubusercontent.com/gebogebogebo/ctap-hid-fido2/master/src/fidokey/get_info/mod.rs — Info struct fields
- /Users/michael/code/textual-rs/crates/textual-rs/src/widget/context.rs — `run_worker_with_progress` API
- /Users/michael/code/textual-rs/crates/textual-rs/src/worker.rs — WorkerResult/WorkerProgress types

### Secondary (MEDIUM confidence)
- github.com/gebogebogebo/ctap-hid-fido2/blob/master/README.md — Windows admin privilege requirement (stated in README)
- docs.yubico.com/yesdk/users-manual/application-fido2/apdu/reset.html — reset timing specification (CTAP command 0x07, 10s window, error codes)
- crates.io/crates/ctap-hid-fido2 — version 3.5.9 confirmed, published 2026-03-15

### Tertiary (LOW confidence)
- CTAP spec (fidoalliance.org) — authenticatorReset §6.6 command byte 0x07 (not directly fetched from authoritative source, but corroborated by Yubico docs above)

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — docs.rs verified, version from crates.io API
- Credential management API: HIGH — source code inspected
- Reset implementation: MEDIUM — no reset in crate confirmed (verified via source tree search); raw HID approach is well-understood but not tested against real device
- Architecture: HIGH — follows established OathScreen/PivScreen patterns in codebase
- Pitfalls: HIGH — all confirmed by direct inspection

**Research date:** 2026-03-27
**Valid until:** 2026-06-27 (ctap-hid-fido2 is actively developed; re-verify reset API before that date)
