---
created: 2026-03-26T04:53:36.875Z
title: Fix wrong model detection when management AID unavailable
area: ui
files:
  - src/yubikey/detection.rs:82-90
  - src/yubikey/card.rs:280-318
---

## Problem

When `get_device_info()` returns `None` (management AID not supported or query fails silently),
the fallback path at `detection.rs:89` calls `detect_model_from_version(&openpgp_version)` where
`openpgp_version` is extracted from GET DATA 0x4F AID bytes 6–7.

Those bytes are the **OpenPGP card application spec version**, not the YubiKey firmware version:
- YubiKey NEO: OpenPGP app v2.0 → aid[6]=2 → `Model::Unknown` (bad, but at least not wrong)
- YubiKey 4/5: OpenPGP app v3.4 → aid[6]=3 → `Model::YubiKeyNeo` ← **wrong model**

User sees "Device: YubiKey NEO USB-A | FW: 3.4.0" for a YubiKey 4 or 5 whose management AID
query is failing. The "FW: 3.4.0" is also wrong — it's the OpenPGP spec version, not firmware.

## Solution

Two complementary fixes:
1. **Rename/clarify** `openpgp_version` in detection.rs to `openpgp_spec_version` so future code
   doesn't treat it as firmware.
2. **Fallback model logic**: when management AID fails and OpenPGP spec major ≥ 3, return
   `Model::Unknown` rather than `Model::YubiKeyNeo` — the NEO uses OpenPGP 2.x, so major≥3
   cannot be a NEO.
3. **Investigate** why `get_device_info()` returns `None` for what appears to be a YubiKey 5 —
   check if SELECT_MGMT is failing because of reader/protocol issues or a bug in the APDU.
