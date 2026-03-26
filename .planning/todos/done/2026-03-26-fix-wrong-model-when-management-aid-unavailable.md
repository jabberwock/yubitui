---
created: 2026-03-26T04:53:36.875Z
title: Fix wrong model detection when management AID unavailable
area: ui
files:
  - src/yubikey/detection.rs:82-90
  - src/yubikey/card.rs:280-318
---

## Problem

**Hardware confirmed:** YubiKey 5C NFC (`Yubico YubiKey OTP FIDO CCID` reader name) shows as
"YubiKey NEO USB-A | FW: 3.4.0". The fix in commit `8d3234c` did not resolve it.

The root cause has two layers:

**Layer 1 — management AID data is not usable:**
`get_device_info()` likely returns `Some(DeviceInfo { firmware: None, form_factor_byte: None, serial: None })`.
Either:
- GET_DEVICE_INFO APDU (`00 1D 00 00 00`) succeeds (SW=9000) but the response TLV has an outer
  wrapper that `tlv_find` doesn't unwrap before searching for tags 0x05/0x04/0x02, OR
- SELECT_MGMT succeeds but GET_DEVICE_INFO fails silently (SW≠9000 → return None → same path)

**Layer 2 — bad fallback when firmware is None:**
In `detection.rs:83`, `di.firmware.clone().unwrap_or(openpgp_version)` falls back to
`openpgp_version` when `firmware` is None. But `openpgp_version` is the OpenPGP *spec* version
(3.4 for YubiKey 5), NOT the hardware firmware. `model_from_form_factor(ff_byte=0, fw_major=3)`
→ `(3, _, _) => (Model::YubiKeyNeo, FormFactor::UsbA)`.

The FW version shown (3.4.0) is also wrong for the same reason.

## Investigation needed

1. Add `tracing::debug!` to `get_device_info()` to log what SELECT_MGMT and GET_DEVICE_INFO
   actually return (SW code and response length). Run with `RUST_LOG=debug` to see.
2. Check if GET_DEVICE_INFO response has an outer TLV container tag (e.g. `0x71`) that needs to
   be unwrapped before searching for inner tags. ykman's `Tlv.parse_dict` skips this automatically
   via its iterator; our `tlv_find` walks flat TLV and may skip over the inner tags.
3. Check if the actual firmware tag in the GET_DEVICE_INFO response uses a different tag byte than
   `0x05` (verify against ykman source: `yubikit/management.py` DEVICE_INFO_TAG enum).

## Solution

1. **Fix GET_DEVICE_INFO TLV parsing**: if the response has an outer container, unwrap it first
   before calling `tlv_find` for inner tags.
2. **Fix fallback**: when `di.firmware` is `None` (management AID returned no parseable firmware),
   do NOT fall back to `openpgp_version`. Instead return `Model::Unknown` and display "Unknown".
3. **Fix fallback model logic**: when management AID fully fails (None) and OpenPGP spec major ≥ 3,
   return `Model::Unknown` — the NEO uses OpenPGP 2.x, so major≥3 is not a NEO.
