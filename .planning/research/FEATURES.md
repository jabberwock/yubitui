# Feature Research

**Domain:** YubiKey TUI management tool (v1.1 — new capabilities)
**Researched:** 2026-03-26
**Confidence:** HIGH (Yubico official docs verified; Rust crate status verified)

---

## Context

This research covers what Yubico Authenticator does that yubitui v1.0 does not. v1.0 already ships:
PIV screen, PIN management, key import/generate, SSH wizard, touch policy, attestation, diagnostics,
multi-key Tab switching, and all reads via native PC/SC APDUs.

The question is: which of Yubico Authenticator's remaining features should v1.1 build, in what order,
and at what complexity?

---

## Yubico Authenticator Feature Map (Verified Against Official Docs)

Source: https://docs.yubico.com/software/yubikey/tools/authenticator/auth-guide/ykauth-intro.html
and https://docs.yubico.com/software/yubikey/tools/ykman/FIDO_Commands.html

### OATH (TOTP/HOTP)
- List all stored credentials (CALCULATE ALL APDU `0xa4`, AID `A0:00:00:05:27:21:01`)
- Generate code for one credential (CALCULATE APDU `0xa2`)
- Add credential via QR scan or manual entry (PUT APDU `0x01`)
- Delete credential (DELETE APDU `0x02`)
- Rename credential
- Password-protect the OATH application (SET CODE `0x03` + VALIDATE `0xa3`)
- Pin/favorite specific accounts for quick access
- Touch-required flag per credential
- Credential fields: issuer, account name, type (TOTP/HOTP), algorithm (SHA-1/SHA-256/SHA-512), digits (6/7/8), period (15/30/60s for TOTP), counter (HOTP)
- Reset OATH application (RESET APDU `0x04`)

### FIDO2 / WebAuthn
- List resident/discoverable credentials (up to 25 on YubiKey 5)
- Delete individual credential (irreversible)
- Set FIDO2 PIN (first-time)
- Change FIDO2 PIN
- View PIN retry count
- Force-change PIN flag
- Set minimum PIN length
- Reset FIDO2 application (erases all credentials + PIN; must happen within 10s of insert)
- Toggle always-UV (user verification requirement)
- Enable Enterprise Attestation (pre-configured keys only)
- Fingerprint management (YubiKey Bio series only): add, list, delete, rename

### OTP Slots (Yubico OTP Application)
Two configurable slots (short press = slot 1, long press = slot 2). Per-slot operations:
- Configure as Yubico OTP (44-char unique string using secret key + device fields)
- Configure as static password (up to 38 chars, never changes)
- Configure as HMAC-SHA1 challenge-response (YubiKey hashes challenge with secret)
- Configure as OATH HOTP (6 or 8 digit counter-based code via HMAC-SHA1)
- Swap slot 1 and slot 2 configurations
- Delete slot configuration (irreversible)
- View current slot occupancy (empty vs. configured)

### PIV (already in v1.0 — gaps vs. Yubico Authenticator)
yubitui v1.0 has: slot occupancy, key import, key generate, touch policy, attestation
What Yubico Authenticator has that we lack:
- Certificate viewer (decode and display X.509 certificate metadata: subject, issuer, validity, SANs)
- Certificate import from file (PEM/DER/P12)
- Certificate delete from slot
- PIV Management Key change (separate from user/admin PIN)
- Full PIN/PUK/Management Key management in one place

### General / Management
- Toggle individual YubiKey applications on/off (OATH, FIDO2, OTP, PIV, OpenPGP, etc.)
- Factory reset of individual applications (each app has independent reset)
- Change YubiKey label (device label shown in apps)
- Change YubiKey color/theme (cosmetic, Yubico Authenticator-specific)
- Multi-key switching (already in v1.0)

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist when they switch from Yubico Authenticator or ykman to yubitui.
Missing these = "it's missing basic stuff."

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| TOTP code generation (list + calculate) | Core reason many users touch Yubico Authenticator daily | MEDIUM | Native OATH APDU — SELECT AID `A0:00:00:05:27:21:01`, CALCULATE ALL `0xa4`. TOTP requires system clock for 30s window display. |
| Add TOTP/HOTP account (manual entry) | Without add, the list is read-only and useless | MEDIUM | PUT APDU `0x01`. QR scan impossible in TUI (no camera). Manual entry of issuer + secret covers 95% of real use. |
| Delete TOTP/HOTP account | Users accumulate stale accounts; deletion is essential | LOW | DELETE APDU `0x02`. Needs irreversibility warning (same pattern as touch policy). |
| FIDO2 PIN set/change | FIDO2 keys without PIN are insecure; users need to configure this | MEDIUM | Requires libfido2 or ctap-hid-fido2 crate (HID, not PC/SC). Architecture decision: separate HID channel from existing PC/SC. |
| FIDO2 resident credential list | Users need to audit what passkeys are on-key | MEDIUM | Requires PIN-authenticated CTAP2 session. ctap-hid-fido2 v3.5.9 (maintained, Mar 2026) supports this. |
| FIDO2 credential delete | Users accumulate stale passkeys; deletion is essential | MEDIUM | Irreversible. Requires credential ID from enumerate step. |
| FIDO2 reset | Locked-out users' only recovery path | MEDIUM | Must happen within 10s of insert. Touch confirmation required. Irreversible — needs hard warning. |
| OTP slot view (current config) | Users need to know what slots contain before changing | LOW | READ STATUS APDU. Already partially done in v1.0 for OTP-adjacent operations. |
| PIV Management Key change | Security best practice; factory default is well-known | LOW | Already have PIN management flow — same pattern for management key. |
| PIV certificate view (decode X.509) | Users import certificates; they need to verify what's there | MEDIUM | Requires X.509 parser (x509-parser Rust crate). Display subject, issuer, expiry, SANs. |

### Differentiators (Competitive Advantage)

Features that make yubitui better than Yubico Authenticator for power users and developers.
These align with yubitui's Core Value: zero-friction, guided, all-in-one.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| In-TUI protocol education (PIV, FIDO2, OATH, OTP, OpenPGP, SSH) | Yubico Authenticator has no explanation of what protocols are or when to use them. yubitui can be the tool that teaches you while you use it. | MEDIUM | Dedicated ? keybinding per screen opens a modal with plain-English explanation + use-case examples. Content is static text — low risk, high value. |
| New user onboarding flow | Yubico Authenticator assumes you know what you're doing. yubitui can greet first-timers: detect empty/factory-state key, suggest what to set up first. | HIGH | Requires "is this key configured?" heuristics: PIN is default? No FIDO2 PIN? No OATH accounts? Guide user through initialization checklist. |
| Tmux-based E2E test suite | No comparable tool has reproducible TUI integration tests. Signals professional quality. | HIGH | Entirely internal — affects developer experience, not end users. Makes v1.1 reliable enough for other users to build on. |
| Model/View separation (Tauri-ready) | Architecture that enables future GUI without rewriting logic | HIGH | Internal — enables future milestones, not visible to users now. |
| TOTP countdown timer in TUI | Show time remaining before code expires (30s window). Yubico Authenticator on desktop does this; most TUI tools don't. | LOW | Derive from `SystemTime::now()` modulo period. Ratatui progress bar or numeric display. |
| OATH application password protection | Adds PIN-gating to TOTP secrets on key | HIGH | Requires HMAC challenge/response auth to OATH AID before operations. Complex state management. |
| OTP slot configure (write, not just read) | ykman handles this; few TUI tools do. Static password manager on hardware. | HIGH | Requires OTP configure APDU (complex binary format with slot config flags). HIGH risk: misconfiguration locks out slot. |
| Application toggle (enable/disable per app) | Power users managing enterprise YubiKeys | HIGH | Requires Management Key authentication. Enterprise use case. Niche but no TUI competitor has it. |
| FIDO2 fingerprint management (Bio series) | Unique to YubiKey Bio — no other TUI covers this | HIGH | ctap-hid-fido2 supports fingerprint add/list/delete. Niche (Bio series only). |

### Anti-Features (Explicitly Avoid)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| QR code scanning for TOTP add | Yubico Authenticator uses QR scan as primary TOTP add method | TUI has no camera access. Attempting to parse QR from terminal screenshots or clipboard is fragile and platform-specific. | Manual entry with issuer + secret + type fields. Works everywhere. Cover 95% of use cases. |
| YubiKey color/label theming | Yubico Authenticator lets you personalize key label and color | Purely cosmetic. Adds no security or productivity value. Requires Management Key auth for label write. | Show device serial + firmware version as identity. Skip cosmetic customization. |
| Cloud backup of TOTP secrets | Users worry about losing the key | Exporting TOTP secrets from hardware defeats the security model. Secrets stored in YubiKey secure element cannot be extracted by design. | Guide users to keep backup YubiKey enrolled in same services. Document recovery flow. |
| OTP slot Yubico OTP programming | Looks like table stakes | Yubico OTP validation requires Yubico's cloud validation service. Configuring this without understanding the ecosystem causes broken auth. Users who need this use ykman. | Show current slot occupancy. Document that slot reprogramming for Yubico OTP requires cloud key registration. |
| Web-based feature set (passkey registration) | Users want to "register" a passkey from the TUI | WebAuthn registration requires a browser + relying party. The TUI cannot substitute for a browser FIDO2 flow. | Show existing resident credentials. Guide users to do registration in browser, then return to yubitui to audit/manage. |
| OpenPGP card reset from TUI | Power feature, seems in-scope | Reset deletes all GPG keys. GPG key material may be unrecoverable. This is a catastrophic irreversible operation requiring gpg tooling for full confirmation flow. | Show GPG key status (already in v1.0). Document manual `gpg --card-edit` reset procedure. |
| FIDO U2F (legacy) management | U2F keys are stored server-side, not on the YubiKey | U2F credentials are non-resident — there is nothing to list or delete on-device. The UI would be empty/misleading. | Explain that U2F registrations are managed at the service (website) level, not on the key. |

---

## Feature Dependencies

```
[OATH List Credentials]
    └──requires──> [OATH SELECT AID + CALCULATE ALL APDU]
                       └──requires──> [PC/SC CCID channel already in v1.0]

[OATH Add Credential]
    └──requires──> [OATH SELECT AID + PUT APDU]
    └──requires──> [OATH List Credentials] (to show added account)

[OATH Password Protection]
    └──requires──> [OATH List Credentials]
    └──requires──> [SET CODE + VALIDATE APDU]
    └──requires──> [OATH Add/Delete] (all ops need auth)

[FIDO2 Credential List]
    └──requires──> [ctap-hid-fido2 or libfido2 HID channel] -- NEW dependency, not in v1.0
    └──requires──> [FIDO2 PIN set] (enumerate requires authenticated session)

[FIDO2 Credential Delete]
    └──requires──> [FIDO2 Credential List] (need credential ID)
    └──requires──> [FIDO2 PIN set]

[FIDO2 Reset]
    └──requires──> [10s timing window after insert]
    └──requires──> [touch confirmation on key]
    -- does NOT require PIN (resets it)

[FIDO2 Fingerprint Management]
    └──requires──> [FIDO2 PIN set]
    └──requires──> [YubiKey Bio series hardware] (not available on standard 5 series)

[PIV Certificate View]
    └──requires──> [x509-parser Rust crate]
    └──enhances──> [PIV screen already in v1.0]

[PIV Management Key Change]
    └──enhances──> [PIN management already in v1.0]

[OTP Slot View]
    └──requires──> [OTP READ STATUS APDU]
    -- independent of existing v1.0 features

[OTP Slot Configure]
    └──requires──> [OTP Slot View]
    └──requires──> [OTP CONFIGURE APDU (complex binary slot config)]

[In-TUI Education]
    -- independent of all feature implementations
    -- can be built as static content modals at any time

[New User Onboarding]
    └──requires──> [OATH List] (detect empty OATH)
    └──requires──> [FIDO2 info] (detect no FIDO2 PIN)
    └──enhances──> [In-TUI Education] (onboarding surfaces explanations)

[TOTP Countdown Timer]
    └──requires──> [OATH List Credentials]
    └──enhances──> [OATH code display]

[Model/View Separation]
    -- architectural prerequisite, no feature dependency
    -- enables: cleaner code for all new features; Tauri-ready

[Tmux E2E Tests]
    -- infrastructure prerequisite
    -- should precede any new feature implementation for TDD discipline
```

### Dependency Notes

- **FIDO2 requires HID channel, not PC/SC:** yubitui v1.0 is entirely PC/SC (CCID). FIDO2 management uses USB HID via CTAP2 protocol. These are separate OS-level interfaces. ctap-hid-fido2 v3.5.9 (maintained March 2026) provides this in Rust. Adding this crate is the critical new dependency for all FIDO2 features.
- **OATH is pure PC/SC CCID:** OATH uses the same PC/SC channel as PIV/OpenPGP. SELECT AID switches to OATH applet. No new transport dependency — builds directly on v1.0's card.rs.
- **OTP slot config is HID, not PC/SC:** OTP slot programming uses the keyboard/HID interface (YubiKey presents as USB HID keyboard). This is a second transport — same family as FIDO2 HID but different protocol. Complex to add without ctap-hid-fido2 or direct HID.
- **In-TUI Education is zero-dependency:** Static text modals. Can be built in any phase, by any developer, without hardware.
- **Model/View separation should precede new feature screens:** Writing new OATH/FIDO2 screens into the current monolithic architecture will create debt. Architectural separation first makes subsequent feature additions clean.

---

## v1.1 Scope Definition

### Build in v1.1 (This Milestone)

Ordered by dependency chain and user value.

- [ ] **Mouse support fix** — v1.0 partial implementation is broken; this is a stated v1.1 requirement from PROJECT.md
- [ ] **Model/View architectural separation** — prerequisite for clean feature additions; enables Tauri future
- [ ] **Tmux E2E test suite** — TDD infrastructure; should be in place before new features ship
- [ ] **In-TUI protocol education** — zero dependencies; highest value-to-effort ratio; differentiates from all competitors
- [ ] **OATH list + code generation** — table stakes; PC/SC CCID, no new transport
- [ ] **OATH add account (manual entry)** — table stakes; extends OATH list
- [ ] **OATH delete account** — table stakes; extends OATH list
- [ ] **FIDO2 PIN set/change** — table stakes; requires ctap-hid-fido2 crate addition
- [ ] **FIDO2 resident credential list** — table stakes; requires PIN-auth FIDO2 session
- [ ] **FIDO2 credential delete** — table stakes; requires credential list
- [ ] **FIDO2 reset** — table stakes; recovery path for locked users
- [ ] **OTP slot view** — table stakes (read-only); PC/SC OTP STATUS APDU
- [ ] **New user onboarding flow** — differentiator; requires OATH + FIDO2 detection
- [ ] **TOTP countdown timer** — low complexity differentiator; built on OATH list

### Defer to v1.2+

- [ ] **PIV certificate view (X.509 decode)** — requires x509-parser; useful but not blocking
- [ ] **PIV Management Key change** — useful; low complexity; can wait
- [ ] **OATH application password protection** — complex state; niche use case
- [ ] **OTP slot configure (write)** — high risk; complex APDU; power user only
- [ ] **Application toggle (enable/disable)** — enterprise niche; complex auth
- [ ] **FIDO2 fingerprint management** — Bio series only; niche hardware
- [ ] **YubiHSM Auth** — highly specialized; not general-purpose user need

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Mouse support fix | HIGH | LOW | P1 |
| Model/View separation | HIGH (arch) | HIGH | P1 |
| Tmux E2E test suite | HIGH (quality) | HIGH | P1 |
| In-TUI protocol education | HIGH | LOW | P1 |
| OATH list + code generation | HIGH | MEDIUM | P1 |
| OATH add account | HIGH | MEDIUM | P1 |
| OATH delete account | HIGH | LOW | P1 |
| FIDO2 PIN set/change | HIGH | MEDIUM | P1 |
| FIDO2 credential list | HIGH | MEDIUM | P1 |
| FIDO2 credential delete | HIGH | LOW | P1 |
| FIDO2 reset | MEDIUM | MEDIUM | P1 |
| OTP slot view | MEDIUM | LOW | P1 |
| New user onboarding | HIGH | HIGH | P1 |
| TOTP countdown timer | MEDIUM | LOW | P2 |
| PIV cert view (X.509) | MEDIUM | MEDIUM | P2 |
| PIV management key change | MEDIUM | LOW | P2 |
| OATH password protection | LOW | HIGH | P3 |
| OTP slot configure (write) | LOW | HIGH | P3 |
| Application toggle | LOW | HIGH | P3 |
| FIDO2 fingerprints (Bio) | LOW | HIGH | P3 |

---

## In-TUI Education: UX Patterns

**What good contextual education looks like in a TUI (verified sources):**

Effective TUI help is:
- **Triggered by `?` keybinding** on any screen — not auto-shown, not intrusive
- **Context-specific** — the PIV screen's `?` explains PIV; the FIDO2 screen's `?` explains FIDO2
- **Plain English with one concrete example** — "PIV is a smart card standard. Your company badge probably uses PIV."
- **Rendered as modal overlay** — same popup widget pattern already in v1.0 (`render_popup`)
- **Dismissable immediately** — Esc or any key closes

**Anti-pattern:** Showing education to repeat users on every visit. Solution: Only show onboarding once (first launch heuristic), then only on `?` request.

**Protocol education content scope for v1.1:**
- PIV — what it is, when to use it, slots 9a/9c/9d/9e explained
- FIDO2/WebAuthn — what passkeys are, resident vs non-resident, PIN requirement
- OATH (TOTP/HOTP) — how time-based codes work, why hardware storage beats software apps
- OTP slots — Yubico OTP vs static password vs challenge-response, use cases
- OpenPGP — sign vs encrypt vs auth key, relationship to SSH
- SSH — how gpg-agent SSH works, why hardware key is better than file-based key

---

## New User Onboarding Flow

**Trigger heuristics** (detect "new user" state without asking):
- FIDO2 application has no PIN set (default state)
- OATH has 0 credentials
- PIV slots all use factory default management key (default: 010203040506070801020304050607080102030405060708)
- OpenPGP key slots are empty

**Flow design** (from best practices research):
1. Welcome screen on first launch: "Your YubiKey is connected. Let's set it up."
2. Show detected state: "No FIDO2 PIN · No TOTP accounts · PIV using factory defaults"
3. Checklist-style: user picks what to configure first (not forced linear)
4. Each step surfaces the relevant screen with pre-loaded context
5. Completion: "Your key is configured. Here's what you can do next."

**Anti-pattern:** Forcing all steps before allowing normal use. Users must be able to skip onboarding and come back.

---

## Competitor Feature Analysis

| Feature | Yubico Authenticator | ykman CLI | yubitui v1.0 | yubitui v1.1 Plan |
|---------|---------------------|-----------|--------------|-------------------|
| OATH TOTP list + generate | Yes (primary use case) | Yes | No | Build |
| OATH add account | Yes (QR + manual) | Yes | No | Build (manual only) |
| FIDO2 PIN management | Yes | Yes | No | Build |
| FIDO2 credential list/delete | Yes | Yes | No | Build |
| OTP slot view | Yes | Yes | No | Build (read-only) |
| OTP slot configure (write) | Yes | Yes | No | Defer |
| PIV key generate/import | Yes | Yes | Yes (v1.0) | Already done |
| PIV slot occupancy | Yes | Yes | Yes (v1.0) | Already done |
| PIV cert view | Yes | Yes | No | Defer to v1.2 |
| Touch policy | Yes | Yes | Yes (v1.0) | Already done |
| Attestation | Yes | Yes | Yes (v1.0) | Already done |
| Protocol education | No | No | No | Build (differentiator) |
| New user onboarding | No | No | No | Build (differentiator) |
| Tmux E2E tests | n/a | n/a | No | Build (quality) |
| Native PC/SC (no ykman) | No (uses ykman/libfido2) | No (is ykman) | Yes (v1.0) | Maintain |

---

## Sources

- Yubico Authenticator Overview: https://docs.yubico.com/software/yubikey/tools/authenticator/auth-guide/ykauth-intro.html
- OATH Commands and APDUs: https://docs.yubico.com/yesdk/users-manual/application-oath/oath-commands.html
- OATH Credentials Overview: https://docs.yubico.com/yesdk/users-manual/application-oath/oath-credentials.html
- FIDO2 Credential Management: https://docs.yubico.com/yesdk/users-manual/application-fido2/fido2-cred-mgmt.html
- FIDO2 Reset: https://docs.yubico.com/yesdk/users-manual/application-fido2/fido2-reset.html
- ykman FIDO Commands: https://docs.yubico.com/software/yubikey/tools/ykman/FIDO_Commands.html
- OTP Slot Configuration: https://docs.yubico.com/software/yubikey/tools/authenticator/auth-guide/yubico-otp.html
- YubiKey Protocols and Applications: https://docs.yubico.com/hardware/yubikey/yk-tech-manual/yk5-apps.html
- ctap-hid-fido2 Rust crate (v3.5.9, Mar 2026): https://lib.rs/crates/ctap-hid-fido2
- Contextual Help UX Patterns: https://www.chameleon.io/blog/contextual-help-ux

---

*Feature research for: yubitui v1.1 new capabilities*
*Researched: 2026-03-26*
