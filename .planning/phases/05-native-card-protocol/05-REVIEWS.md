---
phase: 5
reviewers: [gemini]
reviewed_at: 2026-03-26T03:08:13Z
plans_reviewed: [05-01-PLAN.md, 05-02-PLAN.md, 05-03-PLAN.md]
note: Claude (current runtime) skipped for independence — reviewed by Gemini only.
---

# Cross-AI Plan Review — Phase 5

## Gemini Review

# Phase 5: Native Card Protocol — Implementation Plan Review

This review covers the three-wave plan to transition `yubitui` from CLI-based operations (`ykman`, `gpg --card-status`) to a native PC/SC implementation using the `pcsc` crate.

## 1. Summary
The implementation plan is a technically sound and highly detailed roadmap for removing external CLI dependencies. By centralizing hardware primitives in a dedicated `card.rs` module and adopting an "Exclusive Connection" strategy, the plan addresses the most common pitfalls of PC/SC development (resource contention and proprietary vendor extensions). The phased approach—primitives first, then features, then cleanup—minimizes the risk of breaking existing functionality while ensuring that the core logic remains testable via the established parser/transport separation.

---

## 2. Strengths
- **Surgical Abstraction:** Creating `card.rs` as a primitive layer follows best practices, ensuring that high-level modules like `detection.rs` and `pin.rs` don't need to manage raw APDU construction or reader handles directly.
- **Resource Management:** The "D-04" decision to kill `scdaemon` and use `ShareMode::Exclusive` is the correct (albeit aggressive) way to ensure reliable card access on systems where GPG is running.
- **Verification-First Design:** Leveraging the existing pattern of separating parsers from I/O allows for 14+ unit tests using mock data, which is critical since CI environments often lack physical smart cards.
- **UX-Centric Error Handling:** Mapping hex Status Words (SW) to plain English (D-15/D-17) maintains the "intuitive" goal of `yubitui`, preventing the TUI from feeling like a low-level debugger.
- **Proprietary Knowledge:** The plan correctly identifies YubiKey-specific DOs (0xD6-0xD9 for touch, 0xFB for attestation) and OpenPGP AID structures, showing deep alignment with the hardware's actual behavior.

---

## 3. Concerns
- **Performance Thrashing (MEDIUM):** Decision D-04 specifies killing `scdaemon` before *every* operation. If a high-level UI action (like "Refresh Dashboard") triggers 10+ individual `get_data` calls, restarting the scdaemon lifecycle 10 times will cause significant lag.
- **Extended Length APDUs (MEDIUM):** Attestation certificates and large OpenPGP data objects (like URLs or fingerprints) can exceed the 256-byte limit of standard short APDUs. If the plan doesn't account for Extended Length APDUs (supported by YubiKey 4/5) or APDU chaining, fetching large data will fail.
- **scdaemon Race Conditions (LOW):** On some Linux/macOS systems, `gpgconf --kill scdaemon` returns before the background process has fully released the PC/SC context. A "Card Busy" error can still occur if the next `connect` call happens too quickly.
- **KDF "Bail Out" (LOW):** The plan to bail out if KDF is enabled (GET DATA 0xF9) is a safe conservative move, but it should be clearly messaged to the user as a "Current Limitation" rather than a generic error.

---

## 4. Suggestions
- **Transaction Scoping:** Instead of "kill scdaemon per APDU," implement a "Session" or "Transaction" scope. Kill `scdaemon` once, perform all required reads (SELECT + multiple GET DATAs), then drop the card handle.
- **Extended Length Support:** In `connect_to_openpgp_card`, ensure the `pcsc` transmit call is prepared to handle buffers larger than 256 bytes. YubiKeys usually support Extended Length, which is much faster than chaining for certificates.
- **Minor Delay after Kill:** Add a small, configurable delay (e.g., 50ms-100ms) after the `kill_scdaemon` subprocess call to allow the OS to clean up the card handle before `yubitui` attempts an exclusive lock.
- **Trace SW Codes:** Ensure `apdu_error_message` includes the hex code in the `tracing::debug!` log even if the UI shows English, to aid in remote troubleshooting for users with fringe firmware versions.

---

## 5. Risk Assessment
**Risk Level: MEDIUM**

**Justification:**
The primary risk is not logic, but **environment**. Moving to native PC/SC makes the app sensitive to specific `pcscd` versions, reader drivers, and OS-level smart card locking behaviors that `gpg` and `ykman` have already solved over years of edge-case handling. However, the plan's adherence to the "Exclusive" pattern and the use of proprietary APDUs for touch/attestation is the only way to achieve the stated "No CLI Deps" goal. The MEDIUM rating reflects the inherent fragility of direct hardware communication across Linux, macOS, and Windows.

## 6. Final Verdict
The plans are **Approved for Implementation** with the recommendation to pay close attention to **Connection Scoping** (to prevent performance lag) and **Extended Length APDUs** (to ensure large data like certificates can be read).

---

## Consensus Summary

Single reviewer (Gemini) — no cross-reviewer consensus required. Key findings:

### Agreed Strengths
- card.rs primitive layer is the right abstraction boundary
- Exclusive connection + scdaemon kill is the correct PC/SC pattern
- Parser/IO separation enables unit tests without hardware
- SW → English error mapping keeps UX goals intact

### Top Concerns (by severity)

**MEDIUM: Performance — kill scdaemon per operation**
Plan 01 Task 2 calls `connect_to_openpgp_card()` independently from each module (pin.rs, openpgp.rs, key_operations.rs). A single dashboard refresh could kill/restart scdaemon 5-10 times. The existing `detect_all_yubikey_states()` in Plan 01 correctly batches reads through one connection — but the individual module functions (get_pin_status, get_openpgp_state, get_key_attributes) each reconnect independently. Consider whether callers always use the batched path or if there are code paths that call these individually.

**MEDIUM: Extended Length APDUs for attestation certs**
Attestation DER certificates are typically 1-2KB. The pcsc crate's `transmit()` uses a caller-provided buffer. The plan specifies a 4096-byte buffer for attestation (Plan 02 Task 2), which should be sufficient — but the standard `get_data()` helper uses a 1024-byte buffer. Verify 1024 bytes is adequate for DO 0x6E (Application Related Data) which contains fingerprints + algorithm attrs for all keys. If a card has URLs or large cardholder data, 0x6E could be close to 1KB.

**LOW: scdaemon race condition on Linux**
Brief delay (50-100ms) after `kill_scdaemon()` may be needed on some systems before the exclusive connect attempt.

**LOW: KDF bail-out UX**
The KDF check in set_touch_policy bails with a message about needing ykman. This is correct behavior but should be surfaced as a known limitation in the UI, not a generic error.

### Divergent Views
N/A — single reviewer.

---

*To incorporate feedback into planning:*
```
/gsd:plan-phase 5 --reviews
```
