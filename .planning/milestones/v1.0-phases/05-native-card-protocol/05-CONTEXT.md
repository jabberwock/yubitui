# Phase 5: Native Card Protocol - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Replace all ykman CLI invocations with native Rust PC/SC APDU implementations.
gpg and gpgconf remain for keyring operations (key gen, import, SSH pubkey export, agent management).
The app must work on a clean system with only pcscd/PC/SC installed — no ykman binary required.

Out of scope: replacing gpg keyring operations (--gen-key, --edit-key, --export-ssh-key).
Out of scope: FIDO2/OTP/management key operations.

</domain>

<decisions>
## Implementation Decisions

### Crate strategy
- **D-01:** `pcsc` raw only — all card communication via hand-written APDUs. No `openpgp-card`, no `yubikey` crate. We reference ykman's open source Python implementation for the exact APDU byte sequences.
- **D-02:** Remove `openpgp-card`, `card-backend-pcsc`, and `yubikey` crates from Cargo.toml — they were added in anticipation of Phase 5 but won't be used given the pcsc-raw decision.
- **D-03:** `pcsc = "2.8"` stays — already used for factory reset, now extended to all card operations.

### scdaemon coexistence
- **D-04:** Before every native PC/SC operation: `gpgconf --kill scdaemon` to release the card channel, then connect with `ShareMode::Exclusive`. scdaemon restarts automatically on the next gpg call. This is the pattern already established for factory reset.
- **D-05:** No explicit scdaemon restart after the operation — lazy restart on next gpg call is sufficient.

### Card read replacement (replacing gpg --card-status)
- **D-06:** Replace all `gpg --card-status` calls used for card state reads with direct PC/SC GET DATA APDUs. This eliminates the class of output-parsing bugs (e.g. the PIN retry counter field-swap bug from this session).
- **D-07:** Key GET DATA DOs to implement:
  - `00 CA 00 C4 00` — PW Status Bytes (PIN retry counters: PW1 byte 4, RC byte 5, PW3 byte 6)
  - `00 CA 00 6E 00` — Application Related Data (AID, fingerprints, key info)
  - `00 CA 00 65 00` — Cardholder Related Data (name, language)
  - `00 CA 00 5F 50 00` — URL of public key
  - `00 CA 00 5E 00` — Login data
- **D-08:** Serial number is extracted from the OpenPGP AID select response (bytes 10–13 big-endian after `D2 76 00 01 24 01 [version] [mfr] [serial4]`).

### Touch policy (YubiKey proprietary extension)
- **D-09:** Read touch policy via GET DATA per slot:
  - `00 CA 00 D6 00` — Signature key touch policy
  - `00 CA 00 D7 00` — Decryption key touch policy
  - `00 CA 00 D8 00` — Authentication key touch policy
  - `00 CA 00 D9 00` — Attestation key touch policy
- **D-10:** Set touch policy via PUT DATA:
  - `00 DA 00 D6 01 [policy]` — Signature slot
  - `00 DA 00 D7 01 [policy]` — Decryption slot
  - `00 DA 00 D8 01 [policy]` — Authentication slot
  - Policy byte: `00`=off, `01`=on, `02`=fixed, `03`=cached, `04`=cached-fixed
  - Requires Admin PIN verified in current session before PUT DATA is accepted.

### Device detection (replacing ykman list --serials)
- **D-11:** Enumerate connected YubiKeys via PC/SC reader list. For each reader, SELECT the OpenPGP AID (`D2 76 00 01 24 01`). On success, extract serial from AID response bytes. This replaces `ykman list --serials`.
- **D-12:** Key attributes currently read via `ykman openpgp info` (key type, fingerprint, touch policy per slot) are replaced with GET DATA 0x6E (application related data) + touch policy DOs per slot.

### PIV scope
- **D-13:** Implement native PIV info read via PC/SC:
  - SELECT PIV AID: `00 A4 04 00 09 A0 00 00 03 08 00 00 10 00 01`
  - GET DATA per slot: `00 CB 3F FF [len] 5C [len] [slot-tag]`
  - PIV slot tags: `9A`=Authentication, `9C`=Signing, `9D`=Key Management, `9E`=Card Auth
- **D-14:** PIV read is best-effort — if SELECT PIV fails (no PIV application, older YubiKey), return empty slot list rather than an error.

### Plan sequencing
- **Plan 1:** Device detection + card state reads. Replace `ykman list --serials`, `ykman openpgp info` (key attributes), and all `gpg --card-status` calls with native PC/SC reads. This is the foundation everything else builds on.
- **Plan 2:** Touch policy + PIV info. Replace `ykman openpgp keys set-touch`, `ykman openpgp info` (touch policy), and `ykman piv info` with native APDUs.
- **Plan 3:** Cleanup. Remove `find_ykman()`, the `ykman` feature detection, unused crates from Cargo.toml. Verify no ykman references remain.

### Error UX for regular users
- **D-15:** All APDU status word errors translate to plain English with an action: e.g. "Card not found — make sure your YubiKey is inserted", "Operation failed — try removing and reinserting your YubiKey". No raw SW codes shown in the UI.
- **D-16:** SW codes go to the debug log (`tracing::debug!`) for power-user diagnostics.
- **D-17:** Build a shared `apdu_error_message(sw: u16, context: &str) -> String` helper that maps common SWs to user messages. Referenced from all PC/SC operation sites.

### gpg operations that remain unchanged
- `gpg --batch --gen-key` — key generation (writes to keyring, card stub transfer via keytocard)
- `gpg --edit-key` — key import to card
- `gpg --export-ssh-key` — SSH public key export from keyring
- `gpgconf --list-dirs` — gnupg home path resolution
- `gpgconf --kill scdaemon` — card channel release (used as a utility before PC/SC ops)
- `gpg-agent` — SSH agent functionality

### Claude's Discretion
- Exact module structure for the PC/SC APDU layer (single `src/yubikey/card.rs` or split by concern)
- Whether to define APDU constants as named `const` values or inline byte arrays with comments
- Retry logic on transient card errors (card removed mid-op)

</decisions>

<specifics>
## Specific Ideas

- "ykman is open source, so" — reference ykman's Python source directly for APDU sequences. Don't reverse-engineer; read the source: `yubikit/core/smartcard/`, `yubikit/openpgp.py`, `yubikit/piv.py`.
- The app is targeting regular users, not just power users. Error messages must be human-readable and actionable, not raw hex codes.
- The PIN retry counter field-swap bug (discovered this session) is the canonical example of why replacing gpg output parsing with direct card reads is the right call.
- Factory reset via APDU is already implemented and working (established this session) — Plan 3 extends and consolidates that pattern.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### ykman source (APDU reference — read for exact byte sequences)
- ykman/yubikit OpenPGP: https://github.com/Yubico/yubikey-manager/blob/main/yubikit/openpgp.py — touch policy DOs, PIN verify, card data reads
- ykman/yubikit PIV: https://github.com/Yubico/yubikey-manager/blob/main/yubikit/piv.py — PIV AID, slot tags, GET DATA format
- ykman/yubikit core APDU: https://github.com/Yubico/yubikey-manager/blob/main/yubikit/core/smartcard/__init__.py — APDU construction patterns

### OpenPGP card spec (data object reference)
- OpenPGP Application on Smart Card Specification v3.4 — Section 4 (Data Objects), Section 7 (Commands). DOs 0x6E, 0xC4, 0x65, 0xD6–0xD9 are the primary targets.

### Existing key source files (read before planning)
- `src/yubikey/detection.rs` — current ykman list + openpgp info calls to replace
- `src/yubikey/touch_policy.rs` — current ykman set-touch call to replace
- `src/yubikey/key_operations.rs` — get_key_attributes() and get_ssh_public_key_text() (card-status reads to replace)
- `src/yubikey/pin.rs` — get_pin_status() / parse_pin_status() — replace with GET DATA 0xC4
- `src/yubikey/openpgp.rs` — get_openpgp_state() — replace with GET DATA 0x6E + 0x65
- `src/yubikey/piv.rs` — get_piv_info() — replace with native PIV APDUs
- `src/yubikey/pin_operations.rs` — factory_reset_openpgp() — already native PC/SC, extend pattern to other ops
- `src/yubikey/pin_operations.rs` §find_ykman — remove entirely in Plan 3

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `factory_reset_openpgp()` in `src/yubikey/pin_operations.rs` — established the kill-scdaemon + exclusive-connect + SELECT + APDU + transmit pattern. This is the template for all Phase 5 PC/SC ops.
- `apdu_sw()` helper in `pin_operations.rs` — extracts 2-byte status word from response slice. Reuse everywhere.
- `find_ykman()` in `pin_operations.rs` — to be deleted in Plan 3.

### Established Patterns
- PC/SC pattern: `gpgconf --kill scdaemon` → `Context::establish` → `list_readers` → `connect(Exclusive)` → `SELECT AID` → transmit APDUs → interpret SW.
- Error handling: `anyhow::bail!` with human-readable messages. SW codes go to `tracing::debug!` only.
- Parser separation: parse functions take `&str`/`&[u8]` and are unit-testable without hardware. Established in Phase 3, continue here.

### Integration Points
- `detect_all_yubikey_states()` in `detection.rs` — replaces ykman serial list with PC/SC reader enumeration
- `get_pin_status()` in `pin.rs` — replaces gpg --card-status parse with GET DATA 0xC4
- `get_openpgp_state()` in `openpgp.rs` — replaces gpg --card-status parse with GET DATA 0x6E
- `get_key_attributes()` in `key_operations.rs` — replaces ykman openpgp info parse
- `set_touch_policy()` in `touch_policy.rs` — replaces ykman openpgp keys set-touch
- `get_piv_info()` in `piv.rs` — replaces ykman piv info

</code_context>

<deferred>
## Deferred Ideas

- Replace gpg keyring operations (gen-key, edit-key, export-ssh-key) with native Rust cryptography — future milestone, significant scope
- FIDO2/WebAuthn status display — backlog
- OTP slot management — backlog
- Management key operations — backlog

</deferred>

---

*Phase: 05-native-card-protocol*
*Context gathered: 2026-03-26*
