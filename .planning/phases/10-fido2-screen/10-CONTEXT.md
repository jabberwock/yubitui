# Phase 10: FIDO2 Screen - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the FIDO2 screen: device info display, resident credential (passkey) listing with PIN-gated access, PIN set/change flows, and a guided reset workflow with timing window support. Navigation wired from the dashboard.

New capabilities NOT in scope: fingerprint management (FIDO-08), enable/disable applications (FIDO-09), FIDO2 attestation.

</domain>

<decisions>
## Implementation Decisions

### CTAP2 Transport
- **D-01:** The researcher evaluates available Rust CTAP2 crates (e.g. `ctap-hid-fido2`, `fido-hid-rs`) vs rolling native HID frames on **stability and security grounds** — not speed or convenience. The most stable and secure option wins regardless of implementation effort. The roadmap's no-ykman rule prohibits shelling out to external tools; a native Rust library is fully acceptable.
- **D-02:** The researcher must specifically spike credential enumeration (`credentialManagement` command) and credential deletion — the roadmap flags MEDIUM confidence on these operations specifically. Device info and PIN operations are lower risk.

### Credential Loading Flow
- **D-03:** Screen opens and shows device info immediately (no PIN required for info). Below the info section, credentials are loaded via an inline PIN prompt — user enters their FIDO2 PIN in the credentials area to unlock the list.
- **D-04:** If no FIDO2 PIN is configured yet, the credential section shows "No PIN configured — press S to set one." Setting the PIN flows directly into PIN entry and then loads credentials automatically.
- **D-05:** If a PIN is set but user cancels or skips PIN entry, show "Credentials locked — press P to authenticate" as placeholder.

### Screen Structure
- **D-06:** Single `Fido2Screen` widget — no push_screen for sub-views. Layout:
  1. Header("FIDO2 / Security Key")
  2. Device info section: firmware version, supported algorithms, PIN status, PIN retry count
  3. Passkeys section: scrollable credential list (RP ID + user display name per row), inline PIN prompt when locked
  4. Footer with keybindings
- **D-07:** Follows the PivScreen pattern exactly — `compose()` returns the full widget tree, `on_action()` handles keybindings, model layer in `src/model/fido2.rs` with zero ratatui imports.
- **D-08:** Keybindings on the main screen: `S` set/change PIN, `D` delete selected credential, `R` reset FIDO2 applet, `Esc` back to dashboard, Up/Down/j/k navigate credentials.
- **D-09:** Dashboard navigation: key `8` and a "[8] FIDO2 / Security Key" button (following the OATH pattern from phase 09 which used `7`).

### Reset UX
- **D-10:** After the irreversibility confirmation dialog, show a dedicated reset guidance screen:
  - Instruction text: "Unplug your YubiKey now, then replug it within 10 seconds."
  - Live countdown timer (10 → 0 seconds)
  - TUI polls for device reconnect; when detected within the window, sends the reset command automatically
  - If the window expires before replug: show "Window expired — please try again" and return to the FIDO2 screen
  - If reset succeeds: show success message and return to dashboard (FIDO2 applet now factory-default)
- **D-11:** The 10-second timing constraint and its reason ("FIDO2 protocol requires reset within 10s of power-on") must be explained clearly on the guidance screen — FIDO-06 requirement.

### Windows Admin Privilege (FIDO-07)
- **D-12:** Claude's discretion — detect at the point of a FIDO2 operation attempt and show an inline message explaining why elevated privileges are needed and what to do. No persistent banner.

### Claude's Discretion
- Mock data structure for `--mock` mode (what fields Fido2State contains)
- Exact CBOR/CTAP2 command sequencing (researcher/planner will determine from crate API or spec)
- Error handling for card busy, timeout, and auth failure states

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Prior phase decisions (locked patterns)
- `.planning/phases/08-textual-rs-migration/08-CONTEXT.md` — D-01 through D-10: textual-rs widget pattern, model boundary, Pilot test approach, Footer keybindings
- `.planning/phases/09-oath-totp-screen/09-02-SUMMARY.md` — OathScreen as the closest structural template (credential list with selection, keybindings, compose pattern)

### Codebase integration points
- `src/model/mod.rs` — `YubiKeyState::supports_fido2()` — use this to gate FIDO2 screen access
- `src/model/mock.rs` — mock data source; extend with `Fido2State` for `--mock` mode
- `src/tui/dashboard.rs` — nav_7 pattern to follow for wiring nav_8 to Fido2Screen
- `src/tui/piv.rs` — structural template for the new Fido2Screen widget

### Requirements
- `.planning/REQUIREMENTS.md` §FIDO2 — FIDO-01 through FIDO-07 (all must be satisfied)

### CTAP2 / HID (researcher must evaluate)
- `ctap-hid-fido2` crate — primary candidate; researcher must verify `credentialManagement` support
- CTAP2 spec §6.8 authenticatorCredentialManagement — credential enumeration and deletion protocol

</canonical_refs>

<specifics>
## Specific References

- **Layout reference**: The user approved this mockup during discussion:
  ```
   FIDO2 / Security Key
   ──────────────────────────────────
   Firmware: 5.4.3
   Algorithms: ES256, EdDSA
   PIN: Set (3 retries remaining)

   Passkeys (2)
   > github.com    user@example.com
     google.com    user@gmail.com


   [S] Set PIN  [D] Delete  [R] Reset  [Esc] Back
  ```
- **Transport decision**: User explicitly said "go with the most stable and secure approach, whether that's handrolled or not" — researcher has full latitude to recommend either, based on quality not convenience.
- **Reset timing**: The 10-second window is a hard CTAP2 protocol requirement, not a UX choice. The TUI must explain it clearly and handle expiry gracefully.

</specifics>

<deferred>
## Deferred Ideas

- Fingerprint management (FIDO-08) — Bio YubiKey only, deferred to v2
- Enable/disable YubiKey applications (FIDO-09) — requires Management Key auth, deferred to v2
</deferred>
