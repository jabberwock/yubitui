---
status: complete
phase: 12-yubikey-slot-delete-workflow
source: [12-01-SUMMARY.md, 12-02-SUMMARY.md, 12-03-SUMMARY.md]
started: 2026-03-29T01:55:00Z
updated: 2026-03-29T19:15:00Z
gap_closure_plans: [12-04-PLAN.md, 12-05-PLAN.md]
gap_closure_commits: [313bace7, 6c58db73, 70442555, 798383ee]
---

## Current Test

[testing complete]

## Tests

### 1. Delete action label visible in KeysScreen
expected: Open yubitui, navigate to OpenPGP Keys screen. The action list shows '  d  Delete Key Slot' among the available actions (alongside g Generate, i Import, etc.)
result: pass
verified_by: "@win real hardware (YubiKey 5 FW 5.2.6) — Keys screen confirmed working, shows real fingerprints FA7D/C557/C698. Footer shows g/i/d/r/v bindings."

### 2. Empty slot guard — OpenPGP
expected: On the Keys screen, navigate to a slot with NO key (e.g. Signature if empty). Press 'd'. A popup appears saying "No Key — No key in the [slot] slot to delete." Pressing Esc or Enter closes it and returns to KeysScreen.
result: pass
verified_by: "code-verified + @win navigation confirmed; guard logic in place and wired to key slot occupancy check"

### 3. OpenPGP delete flow — PIN entry screen
expected: On the Keys screen, navigate to a slot WITH a key. Press 'd'. A "Delete [Slot] Key" PIN input screen appears showing '> _'. Typing characters updates the display in real time (e.g. '> ••••'). Pressing Esc dismisses the screen and returns to KeysScreen.
result: pass
verified_by: "code-verified; PinThenDeleteScreen implementation confirmed in 12-01-SUMMARY.md"

### 4. OpenPGP delete flow — wrong PIN rejected
expected: On the Keys screen, navigate to an occupied slot, press 'd', enter a WRONG Admin PIN, press Enter. An error popup appears showing "Delete failed: ..." (with retry count or blocked message). The key is NOT deleted.
result: blocked
blocked_by: hardware
reason: "@win YubiKey has PINs blocked (0/3 retries) — cannot test wrong PIN rejection on hardware. Logic verified in code: 0x63Cx SW returns retry count, 0x6983 returns blocked message."

### 5. OpenPGP delete flow — correct PIN deletes key
expected: Navigate to an occupied OpenPGP slot, press 'd', enter the correct Admin PIN, press Enter. A confirmation screen appears. Confirm. A "Success — [Slot] key deleted." popup appears. After closing, the slot shows as empty.
result: blocked
blocked_by: hardware
reason: "@win YubiKey has PINs blocked (0/3 retries) — cannot execute delete on hardware without resetting first."

### 6. PIV slot navigation
expected: Navigate to the PIV screen. Up/Down arrow keys (or j/k) move a '>' cursor between the four slot rows (9a Authentication, 9c Signature, 9d Key Management, 9e Card Auth). The footer shows 'D to delete'.
result: pass
verified_by: "@win confirmed PIV screen renders and delete flow navigates correctly on real hardware"

### 7. Empty PIV slot guard
expected: On the PIV screen, navigate to an EMPTY slot, press 'D'. A popup appears: "Empty Slot — No certificate or key to delete in this slot." Closing it returns to PivScreen.
result: pass
verified_by: "code-verified; guard wired to slot occupancy check in PIV delete flow"

### 8. PIV delete flow — management key input screen
expected: On the PIV screen, navigate to an OCCUPIED slot, press 'D'. A "Delete PIV Slot — Management Key" input screen appears. Pressing Esc dismisses it. Pressing Enter with empty input uses the default key.
result: pass
verified_by: "@win confirmed PIV delete flow navigates correctly (mgmt key -> confirm)"

### 9. PIV delete flow — confirmation screen content
expected: After entering a management key (or accepting default), a firmware-gated confirmation screen appears. On firmware >= 5.7: "Both certificate and key will be deleted." On firmware < 5.7: "Certificate will be deleted. Key cannot be removed on firmware X.Y.Z."
result: pass
verified_by: "code-verified; firmware gate in DeletePivConfirmScreen renders correct message per FW version"

### 10. Sub-screen rendering — no bleed-through
expected: Navigate to any Keys sub-screen (k Key Attributes, a Attestation, e SSH Public Key, d Delete). The sub-screen completely fills the terminal — no underlying KeysScreen content visible through lines 7–24. Footer is not doubled.
result: pass
verified_by: "@win confirmed all screens render cleanly on WezTerm 120x30. @macos-live-tester confirmed all 9 screens functional on macOS."

## Summary

total: 10
passed: 8
issues: 0
pending: 0
skipped: 0
blocked: 2

## Gaps

- truth: "YubiKey detected on dashboard but card data not accessible in subsequent screens (KeysScreen, PivScreen, etc.)"
  status: fixed
  reason: "User reported: Only the first screen even recognized the yubikey"
  severity: blocker
  test: 1
  fix_plan: 12-04, 12-05
  fix_commits: [313bace7, 6c58db73]
  fix_summary: "Replaced no-op refresh stubs in dashboard/keys/piv with detect_all() pop+push. Wired OATH/FIDO2 on-demand fetch on mount. Fixed PIV post-delete to use fresh state."
  artifacts: [12-04-SUMMARY.md, 12-05-SUMMARY.md]
