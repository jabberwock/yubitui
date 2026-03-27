---
status: complete
phase: 05-native-card-protocol
scope: SSH key management (SSH Wizard + Key Management SSH flows)
started: 2026-03-26T00:00:00Z
updated: 2026-03-26T00:00:00Z
tester: claude (tmux)
---

## Current Test

[testing complete]

## Tests

### 1. SSH Wizard — Initial Load Status Accuracy
expected: Opening SSH Setup Wizard (key 5) immediately shows correct ✅/❌ for all three indicators without requiring user action.
result: pass
note: All 3 show ✅ on first load. Previously-reported stale status bug (05-UAT test 6) is fixed.

### 2. SSH Wizard — [4] Export SSH pubkey, no auth key
expected: With no auth key on card, [4] Export shows an error explaining the key is missing (not silent).
result: pass
note: Returns to main with "Status: Error: No authentication key found on card. Import or generate a key first."

### 3. SSH Wizard — [5] Test Connection, form input
expected: [5] Test Connection shows username and hostname fields. Tab switches focus. Typing works in each field.
result: pass
note: Username field focused on entry, Tab switches to hostname, input accepted in both.

### 4. SSH Wizard — stale message bleed across sub-screens
expected: Each action sub-screen starts with no message from previous actions.
result: pass
note: Fixed — ssh_state.message cleared on each navigation arm. [4]→[5] and [1]→[3] transitions show clean screens.

### 5. SSH Wizard — [1] Enable SSH support, idempotent
expected: Pressing Enter when already enabled shows a message, not a duplicate write.
result: pass
note: Shows "SSH support already enabled". No duplicate write.

### 6. SSH Wizard — [3] Restart GPG Agent
expected: Agent restarts successfully and ✅ status remains.
result: pass
note: Shows "GPG agent restarted successfully". All 3 indicators remain ✅ after restart.

### 7. Key Management — [S] SSH pubkey popup, no auth key
expected: With no auth key, [S] shows a popup explaining the key is missing.
result: pass
note: Fixed — popup shows message once. Removed key_state.message assignment in [S] error arm; popup handles None case directly.

### 8. Key Management — [E] Export SSH pubkey, no auth key
expected: With no auth key, confirming [E] Export shows an error (not silent return to main).
result: pass
note: Now routes to SSH pubkey popup showing the error. Previously-reported bug (05-UAT E8) is fixed.

## Summary

total: 8
passed: 8
issues: 0
pending: 0
