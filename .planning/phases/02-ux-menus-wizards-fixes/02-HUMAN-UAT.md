---
status: partial
phase: 02-ux-menus-wizards-fixes
source: [02-VERIFICATION.md]
started: 2026-03-24T00:00:00Z
updated: 2026-03-24T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Dashboard context menu visual overlay and mouse interaction
expected: Press `m` or `Enter` on Dashboard — a popup context menu appears centered on screen with 5 navigation options (Diagnostics, Key Management, PIN Management, SSH Setup Wizard, Help). Up/Down arrows move the highlight. Enter navigates to selected screen. Esc closes without navigating. Mouse scroll moves selection.
result: [pending]

### 2. PIN wizard retry counter display and path gating
expected: Press `4` for PIN Management, then `u`. The wizard check screen shows retry counters (User PIN, Admin PIN, Reset Code) with color coding (green/yellow/red). Recovery options appear based on available retries — option 1 for reset code if retries > 0, option 2 for admin PIN if retries > 0, option 3 for factory reset only when both are 0 and ykman is installed.
result: [pending]

### 3. Key attributes display (ykman path and graceful fallback)
expected: Press `3` for Key Management, then `a`. If ykman is installed: shows algorithm type (e.g., ed25519, RSA2048) per slot (SIG/ENC/AUT). If ykman is not installed: shows "ykman required" message without crashing.
result: [pending]

### 4. SSH pubkey popup overlay
expected: Press `3` for Key Management, then `s`. If an authentication key exists: shows the SSH public key in a popup overlay with copy instructions for GitHub/servers. If no auth key: shows "No authentication key found" message. Esc closes the popup.
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps
