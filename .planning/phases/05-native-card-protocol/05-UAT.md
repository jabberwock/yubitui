---
status: complete
phase: 05-native-card-protocol
source: [05-01-SUMMARY.md, 05-02-SUMMARY.md]
started: 2026-03-25T00:00:00Z
updated: 2026-03-26T00:00:00Z
---

## Current Test

[testing complete]

## Tests

### 1. YubiKey Detection Without ykman
expected: Insert a YubiKey and run the app. Dashboard detects and displays the YubiKey (serial, firmware) via native PC/SC. App works even without ykman installed.
result: pass

### 2. Card State Display (PIN Counters, Fingerprints)
expected: Dashboard shows PIN retry counters (User PIN, Reset Code, Admin PIN), key fingerprints per slot, and algorithm type — all read via native APDUs, not gpg --card-status output parsing.
result: skipped
reason: User jumped to report key import failure

### 2b. Key Import to Card (keytocard)
expected: Importing a generated key to all three slots (SIG, ENC, AUT) completes successfully. If a step fails, the error clearly distinguishes the cause (wrong Admin PIN vs card disconnected) so the user knows what to fix.
result: issue
reported: "still can't import keys and it's not clear why. No idea if the admin pin is bad or what. Error: Card removed -- reinsert and retry / Smartcard operation failed / ENC slot import failed / Slots filled: SIG — ENC — AUT —"
severity: blocker

### 3. Touch Policy Display
expected: Key attributes screen shows touch policy per slot (Signature, Decryption, Authentication, Attestation). Values should be accurate — e.g. "On", "Off", "Fixed", "Cached".
result: issue
reported: "Key Attributes screen ([K]) shows [empty] for all slots. Touch policies are visible in the main Key Management view but not in the dedicated Key Attributes subscreen."
severity: major

### 4. Admin PIN Prompt for Touch Policy Set
expected: When you navigate to set a touch policy on a slot, the app prompts for the Admin PIN before executing. Entering the correct Admin PIN lets the operation proceed; wrong PIN shows an error.
result: pass
note: Admin PIN dialog appears correctly. Minor visual artifact: background shows Import Key screen content bleeding through the modal.

### 5. PIV Slot Detection
expected: Navigating to the PIV/certificates section shows which PIV slots are occupied vs empty, read via native PC/SC (not ykman piv info).
result: issue
reported: "No PIV screen exists in the UI. src/yubikey/piv.rs has PivState/SlotInfo types but all marked #[allow(dead_code)] — feature not connected to any screen."
severity: major

### 6. SSH Setup Status Accuracy
expected: Opening the SSH Setup Wizard (key 5 or menu item), the status panel immediately shows accurate ✅/❌ for: (1) SSH support in gpg-agent.conf, (2) SSH_AUTH_SOCK in shell, (3) GPG agent running. These reflect the actual current state, not stale defaults.
result: issue
reported: "On initial load, gpg-agent.conf showed ❌ even though enable-ssh-support was already present. Status only corrected to ✅ after running an action (which triggered a re-read). Initial read is stale/incorrect."
severity: major

### 7. SSH Config Write ("Configure Shell" action)
expected: Selecting [2] Configure Shell and pressing Enter writes an `export SSH_AUTH_SOCK=...` line to ~/.zshrc (or ~/.bashrc). After completing, the "SSH_AUTH_SOCK configured in shell" indicator updates to ✅.
result: pass
note: Correctly detected existing entry and showed idempotent message "SSH_AUTH_SOCK already configured in /Users/michael/.zshrc". Indicator updated to ✅.

### 8. Enable SSH Support in gpg-agent.conf
expected: Selecting [1] Enable SSH support and pressing Enter adds `enable-ssh-support` to ~/.gnupg/gpg-agent.conf. Status indicator immediately updates to ✅ after the action completes.
result: pass
note: Correctly detected existing entry and showed "SSH support already enabled". No duplicate write.

## Extended Screen Tests (E2E via tmux)

### E1. System Check screen
expected: All diagnostics show accurate real-time status (pcscd, GPG agent version+socket, scdaemon, SSH agent socket).
result: pass
note: All 4 items ✅ with accurate detail. PC/SC: macOS CryptoTokenKit. GPG: 2.4.9. SSH socket confirmed.

### E2. Dashboard — refresh, help, menu
expected: [R] refreshes data, [?] shows help overlay, [m] shows nav menu overlay.
result: pass
note: All three work correctly. Help screen lists all keybindings. Menu shows 5 nav options. Refresh updates state.

### E3. PIN Management — Change User PIN dialog
expected: [C] opens multi-field modal with Current PIN / New PIN / Confirm New PIN, Tab/Enter to navigate fields, Esc to cancel.
result: pass

### E4. PIN Management — Unblock User PIN Wizard
expected: Shows current retry counters. Options gated by available retries — [1] only shown if Reset Code retries > 0.
result: pass
note: Reset Code retries showing 0/3 so only [2] Unblock with Admin PIN visible. Correct gating behavior.

### E5. Key Management — [G] Generate Key Wizard
expected: Step 1/5 algorithm picker shows Ed25519 (recommended), RSA 2048, RSA 4096 with descriptions.
result: pass

### E6. Key Management — [S] SSH Pubkey Popup
expected: Shows SSH public key or clear message if auth slot is empty.
result: pass
note: Shows "No authentication key found on card. Import or generate a key first." Correct.

### E7. Key Management — [V] View Full Card Status
expected: Shows detailed card info (fingerprints, attributes, cardholder name, PIN counters) after confirming.
result: issue
reported: "After pressing Enter on confirmation screen, returns silently to main Key Management view. No card detail is displayed."
severity: major

### E8. Key Management — [E] Export SSH Public Key
expected: Exports auth key in SSH format and displays it, or shows clear error if no auth key present.
result: issue
reported: "With no auth key set, silently returns to main view after confirming. Should show error like [S] does ('No authentication key found')."
severity: minor

### E9. Key Management — [A] Attestation error persistence
expected: Attestation error message should clear when navigating to other actions.
result: issue
reported: "Attestation error 'Security condition not met' persisted and appeared on the [V] View Card Status confirmation screen. Stale message state not cleared between actions."
severity: minor

## Summary

total: 17
passed: 9
issues: 7
pending: 0
skipped: 1

## Gaps

- truth: "Key import to card must clearly report why it failed (wrong Admin PIN vs card disconnected)"
  status: failed
  reason: "User reported: still can't import keys and it's not clear why. No idea if the admin pin is bad or what. Error: Card removed -- reinsert and retry / Smartcard operation failed / ENC slot import failed / Slots filled: SIG — ENC — AUT —"
  severity: blocker
  test: 2b
  artifacts: []
  missing: []

- truth: "Key Attributes screen ([K]) should display touch policy per slot (On/Off/Fixed/Cached)"
  status: failed
  reason: "Key Attributes screen shows [empty] for all slots. Touch policies only appear in main Key Management view."
  severity: major
  test: 3
  artifacts: []
  missing: []

- truth: "PIV/certificates section shows which PIV slots are occupied vs empty"
  status: failed
  reason: "No PIV screen in UI. piv.rs exists with dead_code types but is not wired to any screen or navigation."
  severity: major
  test: 5
  artifacts: [src/yubikey/piv.rs]
  missing: [PIV screen, navigation entry]

- truth: "SSH Setup Wizard shows accurate status on initial load without requiring user action"
  status: failed
  reason: "gpg-agent.conf status showed ❌ on initial load even though enable-ssh-support was present. Re-read only triggered by running an action."
  severity: major
  test: 6
  artifacts: []
  missing: [correct initial status read in SSH setup screen]

- truth: "[V] View full card status displays detailed card information after confirmation"
  status: failed
  reason: "After pressing Enter on confirmation screen, returns silently to main Key Management view. No card detail is displayed."
  severity: major
  test: E7
  artifacts: []
  missing: []

- truth: "[E] Export SSH public key shows error when no auth key is present"
  status: failed
  reason: "With no auth key set, silently returns to main view after confirming instead of showing an error message."
  severity: minor
  test: E8
  artifacts: []
  missing: []

- truth: "Action error messages are cleared when navigating to a different action"
  status: failed
  reason: "Attestation error persisted and appeared on the View Card Status confirmation screen."
  severity: minor
  test: E9
  artifacts: []
  missing: []
