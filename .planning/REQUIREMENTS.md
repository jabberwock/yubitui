# Requirements: yubitui v1.2

**Defined:** 2026-03-29
**Core Value:** Zero-friction YubiKey management — detect problems automatically, guide users through fixes, expose all operations without requiring memorization of CLI incantations.

## v1.2 Requirements

### Provisioning Wizards

- [ ] **WIZARD-01**: User can launch an "Initial YubiKey Setup" wizard from the dashboard that guides them through: FIDO2 PIN setup, first OATH account, PIV/SSH key configuration — each step shows current state and can be skipped
- [ ] **WIZARD-02**: User can launch a "Set Up SSH Key with Touch Policy" wizard that: generates or imports an OpenPGP key to the SIG/AUT slot, sets touch policy, exports the SSH public key, and provides shell configuration instructions — in a single guided flow
- [ ] **WIZARD-03**: Provisioning wizards surface relevant touch policy choices upfront (no touch, touch, cached touch) with plain-language descriptions before any irreversible operation
- [ ] **WIZARD-04**: Dashboard shows a nav affordance hint (1–9 keys) so new users can discover all screens without reading documentation
- [ ] **WIZARD-05**: Each wizard step shows the current device state (e.g. "FIDO2 PIN: not set") so users can see what they're changing before committing

### OATH Improvements

- [ ] **OATH-07**: User can import an OATH account by pasting an otpauth:// URI — issuer, account, secret, and algorithm are pre-filled from the URI; user confirms before adding
- [ ] **OATH-08**: User can set an OATH application password when none is configured — subsequent OATH operations (codes, add, delete) prompt for the password only when the applet requires it (SW 0x6982)
- [ ] **OATH-09**: User can change an existing OATH application password after authenticating with the current password
- [ ] **OATH-10**: User can remove the OATH application password (reset to unprotected) after authenticating with the current password

### PIV Management Key

- [ ] **PIV-03**: User can change the PIV management key from the default (3DES 0x0102...0x080102) to a user-chosen value — supports both 3DES and AES-128/192/256 key types per YubiKey 5.7+ MANAGE SECURITY OBJECT APDU
- [ ] **PIV-04**: User is warned when the PIV management key is at factory default — a banner or badge on the PIV screen indicates the security risk and links to the change workflow
- [ ] **PIV-05**: PIV management key change requires the current management key for authentication before allowing the change — if the current key is the factory default, a simpler "I know it's default" confirmation flow is offered

## v2 Requirements (Deferred)

### OTP Slot Write

- **OTP-02**: User can configure OTP slot 1 or 2 (Yubico OTP, static password, HMAC-SHA1) — deferred due to underdocumented HID frame protocol and access code complexity
- **OTP-03**: User can delete/wipe an OTP slot configuration

### Advanced

- **FIDO2-FINGER**: FIDO2 fingerprint management (Bio series only) — requires CTAP2.1 bio enrollment
- **BACKUP-01**: Backup/restore workflows — deferred
- **TAURI-01**: Tauri GUI layer consuming src/model/ — future milestone

## Out of Scope

- OTP slot write — underdocumented HID protocol; access code management complexity
- FIDO2 fingerprint enrollment — Bio series only, niche hardware
- Cloud backup/restore — security boundary
- GUI (non-TUI) — Tauri milestone TBD

## Traceability

| REQ-ID | Phase | Status |
|--------|-------|--------|
| WIZARD-01 | TBD | Pending |
| WIZARD-02 | TBD | Pending |
| WIZARD-03 | TBD | Pending |
| WIZARD-04 | TBD | Pending |
| WIZARD-05 | TBD | Pending |
| OATH-07 | TBD | Pending |
| OATH-08 | TBD | Pending |
| OATH-09 | TBD | Pending |
| OATH-10 | TBD | Pending |
| PIV-03 | TBD | Pending |
| PIV-04 | TBD | Pending |
| PIV-05 | TBD | Pending |
