---
phase: 03-advanced-yubikey-features
verified: 2026-03-24T00:00:00Z
status: passed
score: 10/10 must-haves verified
gaps: []
human_verification:
  - test: "Touch policy set flow — drop-to-terminal"
    expected: "Pressing 't' on keys screen, selecting a slot and policy, drops to raw terminal for Admin PIN entry, then returns to TUI with updated touch policy displayed"
    why_human: "Requires a connected YubiKey and interactive Admin PIN entry; cannot be exercised in a headless check"
  - test: "Attestation popup with real key"
    expected: "Pressing 'a' on keys screen shows a PEM certificate popup for on-device-generated keys, or a clear error for imported keys"
    why_human: "Requires a connected YubiKey with a key loaded in the SIG slot"
  - test: "Multi-key Tab switcher"
    expected: "With 2+ YubiKeys connected, 'Key X/Y (Tab to switch)' appears on dashboard; Tab increments the active key and re-displays that key's data"
    why_human: "Requires 2+ physical YubiKeys; single-key path is exercised but multi-key indicator is conditional"
---

# Phase 03: Advanced YubiKey Features — Verification Report

**Phase Goal:** Power-user features and release readiness. Done when: `cargo test` passes with meaningful coverage, CI matrix is green, touch policy and attestation work.
**Verified:** 2026-03-24
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #  | Truth                                                                    | Status     | Evidence                                                                     |
|----|--------------------------------------------------------------------------|------------|------------------------------------------------------------------------------|
| 1  | `cargo test` passes with 30+ unit tests                                  | VERIFIED   | 36 tests pass (0 failed) across 7 test modules                               |
| 2  | Parser functions are `pub`                                               | VERIFIED   | All 5 functions confirmed `pub fn` at expected line numbers                  |
| 3  | `touch_policy.rs` has TouchPolicy enum, parse_touch_policies, set_touch_policy | VERIFIED | Lines 7, 77, 144 in touch_policy.rs                                   |
| 4  | `attestation.rs` has get_attestation_cert, parse_attestation_result      | VERIFIED   | Lines 22, 39 in attestation.rs                                               |
| 5  | `detection.rs` has list_connected_serials, detect_all_yubikey_states     | VERIFIED   | Lines 18, 33 in detection.rs                                                 |
| 6  | `app.rs` has `yubikey_states: Vec<YubiKeyState>` and multi-key support   | VERIFIED   | Lines 36-37; yubikey_count, selected_yubikey_idx accessors present           |
| 7  | CI matrix tests Linux, macOS, Windows                                    | VERIFIED   | rust.yml has 3-OS matrix with `fail-fast: false`                             |
| 8  | Release workflow triggers on `v*` tags                                   | VERIFIED   | release.yml has `tags: - "v*"`, builds all 3 OS, uploads artifacts          |
| 9  | Touch policy set flow exists in keys.rs (SetTouchPolicy screens)         | VERIFIED   | KeyScreen variants SetTouchPolicy/Select/Confirm plus IRREVERSIBLE warning   |
| 10 | Attestation popup exists in keys.rs                                      | VERIFIED   | render_attestation_popup, attestation_popup field, 'a' key handler in app.rs |

**Score:** 10/10 truths verified

---

### Required Artifacts

| Artifact                              | Expected                                              | Status      | Details                                                          |
|---------------------------------------|-------------------------------------------------------|-------------|------------------------------------------------------------------|
| `src/yubikey/openpgp.rs`              | `pub fn parse_card_status` + `#[cfg(test)]`           | VERIFIED    | pub fn at line 43; 4 tests present                               |
| `src/yubikey/pin.rs`                  | `pub fn parse_pin_status` + `#[cfg(test)]`            | VERIFIED    | pub fn at line 34; 7 tests present                               |
| `src/yubikey/piv.rs`                  | `pub fn parse_piv_info` + `#[cfg(test)]`              | VERIFIED    | pub fn at line 35; 2 tests present                               |
| `src/yubikey/key_operations.rs`       | `pub fn parse_ykman_openpgp_info` + `#[cfg(test)]`    | VERIFIED    | pub fn at line 39; 3 tests present                               |
| `src/yubikey/detection.rs`            | `pub fn detect_model_from_version` + `#[cfg(test)]`   | VERIFIED    | pub fn at line 161; 8 tests present (4 model + 4 serial)         |
| `src/yubikey/touch_policy.rs`         | TouchPolicy enum, parse_touch_policies, set_touch_policy | VERIFIED | All present; 7 tests in test module                              |
| `src/yubikey/attestation.rs`          | get_attestation_cert, parse_attestation_result        | VERIFIED    | All present; 5 tests in test module                              |
| `src/yubikey/mod.rs`                  | `pub mod touch_policy` + `pub mod attestation`        | VERIFIED    | Lines 8 and 10                                                   |
| `src/yubikey/mod.rs`                  | `touch_policies: Option<touch_policy::TouchPolicies>` | VERIFIED    | Line 149 in YubiKeyState struct                                  |
| `src/yubikey/mod.rs`                  | `fn detect_all` on YubiKeyState                       | VERIFIED    | Line 158                                                         |
| `src/app.rs`                          | `yubikey_states: Vec<YubiKeyState>`                   | VERIFIED    | Line 36                                                          |
| `src/app.rs`                          | `selected_yubikey_idx: usize`                         | VERIFIED    | Line 37                                                          |
| `src/app.rs`                          | `fn yubikey_count`, `fn selected_yubikey_idx`         | VERIFIED    | Lines 887-891                                                    |
| `src/app.rs`                          | `execute_touch_policy_set`                            | VERIFIED    | Line 832                                                         |
| `src/app.rs`                          | `KeyCode::Tab` in Dashboard handler                   | VERIFIED    | Lines 602-605                                                    |
| `src/ui/keys.rs`                      | SetTouchPolicy, SetTouchPolicyConfirm, IRREVERSIBLE   | VERIFIED    | Lines 17-19 (enum variants), line 601 (IRREVERSIBLE text)        |
| `src/ui/keys.rs`                      | `attestation_popup: Option<String>`                   | VERIFIED    | Line 31                                                          |
| `src/ui/keys.rs`                      | `touch_slot_index: usize`                             | VERIFIED    | Line 29                                                          |
| `src/ui/dashboard.rs`                 | "Tab to switch" multi-key indicator                   | VERIFIED    | Lines 35-39                                                      |
| `.github/workflows/rust.yml`          | 3-OS matrix (ubuntu, macos, windows) + clippy         | VERIFIED    | matrix.include with 3 entries; clippy step present               |
| `.github/workflows/release.yml`       | Triggers on `v*` tags, builds all 3 OS                | VERIFIED    | tags: "v*"; 3 OS in matrix; upload-artifact; action-gh-release   |
| `src/ui/keys.rs`                      | Safe fingerprint slicing via `.get(..16).unwrap_or`   | VERIFIED    | Lines 145, 158, 171 — all 3 occurrences use safe `.get()`        |

---

### Key Link Verification

| From                         | To                              | Via                                          | Status   | Details                                                          |
|------------------------------|---------------------------------|----------------------------------------------|----------|------------------------------------------------------------------|
| `detection.rs`               | `ykman list --serials`          | `list_connected_serials` calls `["list", "--serials"]` | WIRED | Line 23 in detection.rs                                    |
| `app.rs`                     | `detection.rs`                  | refresh calls `YubiKeyState::detect_all()`   | WIRED    | detect_all invoked at 5+ refresh points in app.rs                |
| `keys.rs`                    | `touch_policy.rs`               | displays TouchPolicies, wires set through app | WIRED   | Lines 183-203 render tp; app.rs line 843 calls set_touch_policy  |
| `keys.rs`                    | `attestation.rs`                | 'a' key calls get_attestation_cert, PEM shown in popup | WIRED | app.rs line 265; render_attestation_popup at keys.rs line 614 |
| `touch_policy.rs`            | `ykman set-touch`               | set_touch_policy builds command with --device + --force | WIRED | Lines 138-167 in touch_policy.rs                       |
| `attestation.rs`             | `ykman openpgp keys attest`     | get_attestation_cert calls `attest <slot> -` | WIRED    | Line 32 in attestation.rs                                        |
| `rust.yml`                   | `cargo test`                    | runs tests without device-tests feature      | WIRED    | `cargo test --verbose` present; no `device-tests` in file        |
| `release.yml`                | `cargo build --release`         | builds release binary per OS                 | WIRED    | `cargo build --release --verbose` in release.yml                 |

---

### Data-Flow Trace (Level 4)

| Artifact             | Data Variable      | Source                                           | Produces Real Data | Status   |
|----------------------|--------------------|--------------------------------------------------|--------------------|----------|
| `src/ui/keys.rs`     | `yk.touch_policies`| `detect_yubikey_state()` calls ykman openpgp info, parses via `parse_touch_policies` | Yes (when ykman available) | FLOWING  |
| `src/ui/dashboard.rs`| `yubikey_count()`  | `yubikey_states.len()` derived from `detect_all_yubikey_states()` | Yes | FLOWING  |
| `src/ui/keys.rs`     | `attestation_popup`| `get_attestation_cert()` calls ykman attest, captures stdout | Yes (when key on-device) | FLOWING |

---

### Behavioral Spot-Checks

| Behavior                            | Command                                     | Result          | Status  |
|-------------------------------------|---------------------------------------------|-----------------|---------|
| cargo test passes with 30+ tests    | `cargo test`                                | 36 passed, 0 failed | PASS |
| Fingerprint safe slicing in keys.rs | grep for `fingerprint[..16]`                | 0 matches       | PASS    |
| No device-tests in CI               | grep for `device-tests` in workflows        | 0 matches       | PASS    |
| release.yml triggers on v* tags     | grep for `tags:` + `v*`                     | Found           | PASS    |

---

### Requirements Coverage

No explicit requirement IDs were declared for this phase (requirements: [] in all plans). All deliverables are tracked via must_haves in plan frontmatter and verified above.

---

### Anti-Patterns Found

No blocker anti-patterns found. The codebase was scanned for TODO/FIXME, placeholder returns, and empty handlers in the files modified by this phase. One expected `TODO` comment appears in detection.rs as a documented future optimization (single ykman call for touch policies); this is informational and does not block functionality.

| File                          | Line | Pattern                                    | Severity | Impact                                |
|-------------------------------|------|--------------------------------------------|----------|---------------------------------------|
| `src/yubikey/detection.rs`    | ~37  | `// TODO: optimize to single call`         | Info     | Touch policy populated via second ykman call; works correctly, just suboptimal |

---

### Human Verification Required

#### 1. Touch Policy Set Flow

**Test:** Run `cargo run`. Navigate to the Keys screen. Press `t` to enter touch policy selection. Select a slot (e.g., Signature). Select a non-irreversible policy (e.g., On). Confirm the terminal drops out of raw mode and prompts for Admin PIN.
**Expected:** TUI suspends, ykman prompts for Admin PIN in the raw terminal, returns to TUI after entry, and touch policy row updates.
**Why human:** Requires a connected YubiKey and interactive Admin PIN entry.

#### 2. IRREVERSIBLE Warning for Fixed Policy

**Test:** Press `t`, select a slot, then select "Fixed" or "Cached-Fixed".
**Expected:** A confirmation screen appears with the word "IRREVERSIBLE" and requires pressing `y` to proceed. Any other key cancels without change.
**Why human:** Requires hardware; the warning render path is verified in code but the interaction flow needs confirming.

#### 3. Attestation Popup

**Test:** Press `a` on the Keys screen with a YubiKey connected.
**Expected:** Either a PEM certificate popup appears (for on-device-generated keys) or a clear error message (for imported keys, e.g., "Attestation failed").
**Why human:** Requires a connected YubiKey with a key in the SIG slot.

#### 4. Multi-Key Tab Switching

**Test:** Connect 2+ YubiKeys. Run `cargo run`. Observe the Dashboard.
**Expected:** "Key 1/2 (Tab to switch)" indicator appears. Pressing Tab increments to "Key 2/2" and refreshes key data.
**Why human:** Requires 2+ physical YubiKeys.

---

### Gaps Summary

No gaps found. All 10 observable truths are verified in the codebase. The 36-test suite passes cleanly. All workflow files are correct. All UI integration points are wired. Human verification items listed above are behavioral confirmations requiring hardware, not code correctness issues.

---

_Verified: 2026-03-24_
_Verifier: Claude (gsd-verifier)_
