---
status: partial
phase: 12-yubikey-slot-delete-workflow
source: [12-01-SUMMARY.md, 12-02-SUMMARY.md, 12-03-SUMMARY.md]
started: 2026-03-29T01:55:00Z
updated: 2026-03-29T06:45:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Delete action label visible in KeysScreen
expected: Open yubitui, navigate to OpenPGP Keys screen. The action list shows '  d  Delete Key Slot' among the available actions (alongside g Generate, i Import, etc.)
result: blocked
blocked_by: hardware
reason: "Only the first screen (dashboard) recognized the YubiKey — subsequent screens including KeysScreen did not read card data. Hardware access issue prevents testing delete flows."

### 2. Empty slot guard — OpenPGP
expected: On the Keys screen, navigate to a slot with NO key (e.g. Signature if empty). Press 'd'. A popup appears saying "No Key — No key in the [slot] slot to delete." Pressing Esc or Enter closes it and returns to KeysScreen.
result: blocked
blocked_by: hardware
reason: "YubiKey not accessible beyond dashboard screen — see Test 1."

### 3. OpenPGP delete flow — PIN entry screen
expected: On the Keys screen, navigate to a slot WITH a key. Press 'd'. A "Delete [Slot] Key" PIN input screen appears showing '> _'. Typing characters updates the display in real time (e.g. '> ••••'). Pressing Esc dismisses the screen and returns to KeysScreen.
result: blocked
blocked_by: hardware
reason: "YubiKey not accessible beyond dashboard screen — see Test 1."

### 4. OpenPGP delete flow — wrong PIN rejected
expected: On the Keys screen, navigate to an occupied slot, press 'd', enter a WRONG Admin PIN, press Enter. An error popup appears showing "Delete failed: ..." (with retry count or blocked message). The key is NOT deleted.
result: blocked
blocked_by: hardware
reason: "YubiKey not accessible beyond dashboard screen — see Test 1."

### 5. OpenPGP delete flow — correct PIN deletes key
expected: Navigate to an occupied OpenPGP slot, press 'd', enter the correct Admin PIN, press Enter. A confirmation screen appears. Confirm. A "Success — [Slot] key deleted." popup appears. After closing, the slot shows as empty.
result: blocked
blocked_by: hardware
reason: "YubiKey not accessible beyond dashboard screen — see Test 1."

### 6. PIV slot navigation
expected: Navigate to the PIV screen. Up/Down arrow keys (or j/k) move a '>' cursor between the four slot rows (9a Authentication, 9c Signature, 9d Key Management, 9e Card Auth). The footer shows 'D to delete'.
result: blocked
blocked_by: hardware
reason: "YubiKey not accessible beyond dashboard screen — see Test 1."

### 7. Empty PIV slot guard
expected: On the PIV screen, navigate to an EMPTY slot, press 'D'. A popup appears: "Empty Slot — No certificate or key to delete in this slot." Closing it returns to PivScreen.
result: blocked
blocked_by: hardware
reason: "YubiKey not accessible beyond dashboard screen — see Test 1."

### 8. PIV delete flow — management key input screen
expected: On the PIV screen, navigate to an OCCUPIED slot, press 'D'. A "Delete PIV Slot — Management Key" input screen appears. Pressing Esc dismisses it. Pressing Enter with empty input uses the default key.
result: blocked
blocked_by: hardware
reason: "YubiKey not accessible beyond dashboard screen — see Test 1."

### 9. PIV delete flow — confirmation screen content
expected: After entering a management key (or accepting default), a firmware-gated confirmation screen appears. On firmware >= 5.7: "Both certificate and key will be deleted." On firmware < 5.7: "Certificate will be deleted. Key cannot be removed on firmware X.Y.Z."
result: blocked
blocked_by: hardware
reason: "YubiKey not accessible beyond dashboard screen — see Test 1."

### 10. Sub-screen rendering — no bleed-through
expected: Navigate to any Keys sub-screen (k Key Attributes, a Attestation, e SSH Public Key, d Delete). The sub-screen completely fills the terminal — no underlying KeysScreen content visible through lines 7–24. Footer is not doubled.
result: blocked
blocked_by: hardware
reason: "YubiKey not accessible beyond dashboard screen — see Test 1."

## Summary

total: 10
passed: 0
issues: 0
pending: 0
skipped: 0
blocked: 10

## Gaps

- truth: "YubiKey detected on dashboard but card data not accessible in subsequent screens (KeysScreen, PivScreen, etc.)"
  status: failed
  reason: "User reported: Only the first screen even recognized the yubikey"
  severity: blocker
  test: 1
  artifacts: []
  missing: ["Card session scoping investigation — exclusive access may be released after dashboard read, subsequent screens fail to re-acquire"]
