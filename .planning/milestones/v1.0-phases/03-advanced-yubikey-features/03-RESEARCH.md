# Phase 3: Advanced YubiKey Features - Research

**Researched:** 2026-03-24 (updated with fresh codebase audit)
**Domain:** YubiKey CLI integration (ykman), Rust unit testing, GitHub Actions CI matrix, binary release distribution
**Confidence:** HIGH

---

## Summary

Phase 3 adds power-user features (touch policy, multi-key support, attestation) and ships the project as a properly tested, CI-validated release. The codebase is in a strong state after Phase 2: `cargo build`, `cargo test`, and `cargo clippy -- -D warnings` all pass cleanly. Confirmed: zero unit tests exist (`running 0 tests`). All YubiKey operations are channelled through `ykman` and `gpg` CLI calls — the testing strategy focuses on the pure-Rust parsers (which don't touch hardware) rather than command execution.

All three Phase 3 features rely on `ykman openpgp` subcommands. Touch policies are already present in the `ykman openpgp info` output that `parse_ykman_openpgp_info` already partially parses — extending the parser to capture the `Touch policies:` section avoids a second ykman call. `ykman list --serials` gives one decimal serial per line for multi-key enumeration. `ykman openpgp keys attest <slot> -` streams a PEM certificate to stdout. The `find_ykman()` utility in `pin_operations.rs` handles PATH + Windows well-known path and must be reused by all new code.

The existing CI workflow (`rust.yml`) is Linux-only, has no `cargo clippy` step, no macOS or Windows runners, and no release job. Expanding to a 3-OS matrix requires adding conditional `libpcsclite-dev` installation for Linux only — macOS and Windows do not need extra PCSC installation steps.

**Primary recommendation:** Implement in this order — (1) unit tests for all parsers, (2) touch policy backend + UI, (3) multi-key detection + switcher UI, (4) attestation display + CI matrix + release workflow.

---

## Project Constraints (from CLAUDE.md)

CLAUDE.md does not exist in the repository root. Constraints are carried from STATE.md decisions and project conventions observed in code.

**Standing rules from STATE.md:**
- Cross-platform requirement is non-negotiable (Linux/macOS/Windows)
- Security rules: no sensitive values in logs, no shell injection, no hardcoded paths
- Always run `cargo clippy -- -D warnings` before committing
- Never store Admin PIN in App state between operations
- Serial number must not appear in logs (observed in `detection.rs` comment)

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.29 | TUI rendering | Already in project |
| crossterm | 0.28 | Terminal events | Already in project |
| anyhow | 1.0 | Error handling | Already in project |
| ykman CLI | 5.0.1 (dev machine) | YubiKey management commands | Yubico-official tool; used throughout |

### Supporting (already present in Cargo.toml)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| mockall | 0.13 | Mock traits in unit tests | Available if needed for trait mocking |
| tempfile | 3.13 | Temp files in tests | Available if needed for file-based tests |
| assert_cmd | 2.0 | Test CLI invocations | Available for integration-level command tests |
| predicates | 3.1 | Assertion helpers | Available alongside assert_cmd |

### No New Dependencies Required

All Phase 3 features are implementable with the existing dependency set. No new crates need to be added.

**Verified versions:**
```bash
cargo test     # Passed: 0 tests, 0 failures (confirmed 2026-03-24)
cargo clippy   # Passed: 0 warnings (confirmed 2026-03-24)
rustc          # 1.94.0 (from Cargo.toml rust-version = "1.75", actual toolchain is 1.94.0)
ykman          # 5.0.1 on development machine
```

---

## Current Codebase State (Critical for Planning)

### Parser Functions That Need `pub` Visibility for Tests

These functions exist but are **private** — they must be made `pub` before they can be unit-tested:

| Module | Private Function | Status | Action |
|--------|-----------------|--------|--------|
| `src/yubikey/openpgp.rs` | `parse_card_status(output: &str)` | private `fn` | Change to `pub fn` |
| `src/yubikey/pin.rs` | `parse_pin_status(output: &str)` | private `fn` | Change to `pub fn` |
| `src/yubikey/piv.rs` | `parse_piv_info(output: &str)` | private `fn` | Change to `pub fn` |
| `src/yubikey/key_operations.rs` | `parse_ykman_openpgp_info(output: &str)` | private `fn` | Change to `pub fn` |

These are all pure `&str -> Struct` parsers with no I/O dependencies. Making them `pub` is the only code change needed to enable tests.

### Detection Architecture Gap

`detection.rs::detect_yubikeys()` currently uses `gpg --card-status --with-colons` to discover keys and **discards all but the first** (see comment: "For now, just use the first detected key"). Multi-key support requires:
1. Adding `list_connected_serials()` using `ykman list --serials` as a parallel enumeration path
2. Passing `--device <serial>` to all subsequent `ykman` calls
3. Keeping `gpg --card-status` for the OpenPGP state of the selected key (GPG sees only the first card; for multi-key, ykman becomes authoritative)

### `find_ykman()` Location

`find_ykman()` is defined in `src/yubikey/pin_operations.rs`. All new ykman-calling code must import it from there — do not duplicate this function.

### App Struct

`App` in `src/app.rs` currently holds:
```rust
yubikey_state: Option<YubiKeyState>,  // single key
```
Multi-key support requires adding:
```rust
yubikey_states: Vec<YubiKeyState>,
selected_yubikey_idx: usize,
```
The existing `yubikey_state` field can be kept as a derived accessor (`fn yubikey_state() -> Option<&YubiKeyState>`) to avoid breaking all existing render call sites.

### Touch Policy — Integration Point

`key_operations.rs::get_key_attributes()` already calls `ykman openpgp info` and parses SIG/ENC/AUT slot algorithms and fingerprints. The same `ykman openpgp info` output includes the `Touch policies:` section. The natural extension is to add a `touch_policies: TouchPolicies` field to `KeyAttributes` and extend `parse_ykman_openpgp_info` to populate it — zero additional ykman invocations needed.

### Existing UI Patterns Available to Reuse

- **Popup overlay:** `src/ui/widgets/popup::render_popup()` — used for SSH pubkey popup (Plan 02-03). Reuse for attestation PEM display.
- **Destructive confirmation:** Double-Y confirmation pattern from `PinScreen::UnblockWizardFactoryReset` in `app.rs`. Reuse for `fixed`/`cached-fixed` touch policy.
- **Drop-to-terminal:** `execute_pin_operation()` / `execute_key_operation()` pattern in `app.rs` (disable raw mode, leave alternate screen, run command, restore). Reuse for touch policy set (requires Admin PIN prompt).
- **KeyScreen enum:** `src/ui/keys.rs::KeyScreen` — add `TouchPolicy` and `Attestation` variants here.

### CI Workflow Current State

`.github/workflows/rust.yml` has:
- Single job, `ubuntu-latest` only
- Installs `libpcsclite-dev` (correct)
- Runs `cargo build --verbose` and `cargo test --verbose`
- Missing: `cargo clippy -- -D warnings`, macOS runner, Windows runner, release job

---

## Architecture Patterns

### Recommended Project Structure (additions only)

```
src/
├── yubikey/
│   ├── touch_policy.rs    # NEW: TouchPolicies struct, parse_touch_policies(), set_touch_policy()
│   ├── attestation.rs     # NEW: get_attestation_cert(slot, serial)
│   ├── detection.rs       # MODIFY: add list_connected_serials(), pass serial to state fetch
│   ├── key_operations.rs  # MODIFY: extend KeyAttributes with touch_policies field, make parse_ykman_openpgp_info pub
│   ├── openpgp.rs         # MODIFY: make parse_card_status pub
│   ├── pin.rs             # MODIFY: make parse_pin_status pub
│   └── piv.rs             # MODIFY: make parse_piv_info pub
├── ui/
│   ├── keys.rs            # MODIFY: add TouchPolicy + Attestation KeyScreen variants, render functions
│   └── dashboard.rs       # MODIFY: add multi-key indicator + Tab cycling
└── app.rs                 # MODIFY: yubikey_states Vec + selected_yubikey_idx, key switcher handler

tests/
└── (inline #[cfg(test)] modules in each modified source file)
```

No separate `tests/` integration test files are needed — inline `#[cfg(test)]` modules in each parser module are idiomatic for unit tests in Rust and avoid crate visibility issues.

### Pattern 1: Inline unit tests in parser modules

**What:** Tests live inside `#[cfg(test)]` blocks at the bottom of the same file as the function under test. No separate `tests/` directory required for unit tests.

**When to use:** For all pure-Rust parser functions.

**Example:**
```rust
// src/yubikey/pin.rs — at bottom of file

pub fn parse_pin_status(output: &str) -> Result<PinStatus> { ... }  // now pub

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pin_status_healthy() {
        let output = "PIN retry counter : 3 3 0\n";
        let result = parse_pin_status(output).unwrap();
        assert_eq!(result.user_pin_retries, 3);
        assert!(!result.user_pin_blocked);
    }

    #[test]
    fn test_parse_pin_status_user_blocked() {
        let output = "PIN retry counter : 0 3 0\n";
        let result = parse_pin_status(output).unwrap();
        assert_eq!(result.user_pin_retries, 0);
        assert!(result.user_pin_blocked);
        assert!(!result.admin_pin_blocked);
    }

    #[test]
    fn test_parse_pin_status_missing_line() {
        // gpg output without counter line — should return defaults
        let output = "General key info..: [none]\n";
        let result = parse_pin_status(output).unwrap();
        assert_eq!(result.user_pin_retries, 3);  // default
    }
}
```

### Pattern 2: Touch policy — extend existing KeyAttributes

**What:** `ykman openpgp info` already outputs touch policies in the same response as slot info. Extend `parse_ykman_openpgp_info` rather than writing a separate function.

**ykman openpgp info touch policy output (verified on ykman 5.0.1):**
```
Touch policies:
  Signature key:      Off
  Encryption key:     Off
  Authentication key: Off
  Attestation key:    Off
```

**Valid slots for set-touch:** `sig`, `enc`, `aut`, `att`
**Valid policies:** `on`, `off`, `fixed`, `cached`, `cached-fixed`
**Setting touch policy requires Admin PIN:**
```
ykman openpgp keys set-touch <slot> <policy> --admin-pin <pin> [--force]
```
Without `--force`, ykman prompts interactively. The UI should drop to terminal for Admin PIN entry (reusing the existing pattern) and pass `--force`.

**CRITICAL:** Setting `fixed` or `cached-fixed` is irreversible without deleting the private key. The UI must show a destructive-action confirmation (matching factory reset double-Y from Phase 2) before proceeding.

**New types to add to `touch_policy.rs`:**
```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TouchPolicies {
    pub signature: TouchPolicy,
    pub encryption: TouchPolicy,
    pub authentication: TouchPolicy,
    pub attestation: TouchPolicy,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum TouchPolicy {
    #[default]
    Off,
    On,
    Fixed,
    Cached,
    CachedFixed,
    Unknown(String),
}

impl TouchPolicy {
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "off" => Self::Off,
            "on" => Self::On,
            "fixed" => Self::Fixed,
            "cached" => Self::Cached,
            "cached-fixed" => Self::CachedFixed,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn is_irreversible(&self) -> bool {
        matches!(self, Self::Fixed | Self::CachedFixed)
    }
}
```

### Pattern 3: Multi-key detection via ykman list --serials

**What:** Enumerate connected YubiKeys by serial, then pass `--device <serial>` to subsequent ykman subcommands.

**Verified:** `ykman list --serials` outputs one decimal integer per line. Example: `13390292\n`.

**Implementation sketch (new function in `detection.rs`):**
```rust
// Uses find_ykman() from pin_operations — import it
pub fn list_connected_serials() -> Result<Vec<u32>> {
    let ykman = crate::yubikey::pin_operations::find_ykman()?;
    let output = Command::new(ykman)
        .args(["list", "--serials"])
        .output()?;
    if !output.status.success() {
        return Ok(vec![]);  // no keys or ykman unavailable
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().filter_map(|l| l.trim().parse::<u32>().ok()).collect())
}
```

**Note:** `gpg --card-status` only sees the first card. For multi-key display, ykman is authoritative. Keep `gpg --card-status` for PIN status and OpenPGP state of the selected key (it will address the first card, which is fine for single-key use; for multi-key, pass serial via ykman for state queries instead).

### Pattern 4: App multi-key state

**What:** Extend `App` to hold multiple YubiKey states without breaking existing render call sites.

```rust
pub struct App {
    // existing fields unchanged...
    yubikey_state: Option<YubiKeyState>,   // KEEP as derived view of selected key
    // NEW:
    yubikey_states: Vec<YubiKeyState>,
    selected_yubikey_idx: usize,
}
```

On startup: populate `yubikey_states` from `list_connected_serials()`. Derive `yubikey_state` as `yubikey_states.get(selected_yubikey_idx)`. `Tab` on Dashboard cycles `selected_yubikey_idx`.

### Pattern 5: Attestation via popup overlay

**What:** `ykman openpgp keys attest <slot> -` writes PEM to stdout. Display in `render_popup()`.

**Command (verified):**
```rust
pub fn get_attestation_cert(slot: &str, serial: Option<u32>) -> Result<String> {
    let ykman = crate::yubikey::pin_operations::find_ykman()?;
    let mut cmd = Command::new(&ykman);
    if let Some(s) = serial {
        cmd.args(["--device", &s.to_string()]);
    }
    cmd.args(["openpgp", "keys", "attest", slot, "-"]);
    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Attestation failed for slot {}: {}", slot, stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
```

**Failure modes (verified):**
- Slot empty (no key): command exits non-zero
- Key was imported, not generated on-device: command exits non-zero with clear error
- PIN blocked: command exits non-zero

**Display:** Raw PEM in a scrollable popup (same `render_popup()` used for SSH pubkey). No X.509 parsing — raw PEM is sufficient for v1.0 and avoids adding an `x509-parser` dependency.

**Slots:** `sig`, `enc`, `aut` (not `att` — the attestation key cannot attest itself)

### Anti-Patterns to Avoid

- **Making parse functions` pub(crate)` instead of `pub`:** Tests in `#[cfg(test)]` within the same module can access private functions. But using `pub` is cleaner and enables future integration tests if needed.
- **Storing Admin PIN in App state:** Prompt once via drop-to-terminal; pass as `--admin-pin`; discard. Never store in any struct field.
- **Passing `fixed`/`cached-fixed` without destructive warning:** Must show double-confirmation overlay matching the factory reset pattern.
- **Calling `ykman list --serials` on every keypress:** Cache the list; refresh only on `r` (refresh) or startup.
- **Using `gpg --card-status` for multi-key enumeration:** GPG sees only the first card. Use `ykman list --serials` for enumeration.
- **Testing with live hardware in CI:** All parser tests must use fixture strings. Gate any live-hardware test behind `#[cfg(feature = "device-tests")]`. The CI workflow must NOT pass `--features device-tests`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Multi-key enumeration | Custom USB scanning | `ykman list --serials` | Handles USB, NFC, reader abstraction |
| Touch policy management | Direct APDU commands | `ykman openpgp keys set-touch` | Handles all policy variants and error cases |
| Attestation certificate generation | OpenPGP APDU sequence | `ykman openpgp keys attest` | Yubico-specific extension; undocumented APDU |
| X.509 cert parsing | Custom PEM parser | Skip (show raw PEM) | No dependency needed; raw PEM is clear enough |
| Cross-platform binary builds | Custom build scripts | GitHub Actions native runners | Handles toolchain install automatically |

**Key insight:** This codebase deliberately externalises all YubiKey protocol complexity to `ykman` and `gpg`. Phase 3 must stay consistent — there is no reason to introduce `yubikey` crate APDU calls when `ykman` CLI equivalents exist for all three features.

---

## Common Pitfalls

### Pitfall 1: pcsclite on GitHub Actions macOS and Windows

**What goes wrong:** CI fails with "could not find library pcsclite" on non-Linux runners.

**Why it happens:** The `pcsc` and `card-backend-pcsc` crates link against system pcsclite. Linux needs explicit install. macOS and Windows do not.

**How to avoid:**
- Linux: keep `sudo apt-get install -y libpcsclite-dev`
- macOS: no extra step; `pcsc` crate links to `PCSC.framework` automatically
- Windows: no extra step; `pcsc` crate links to `winscard.lib` which ships with Windows

**Warning signs:** CI errors mentioning `pkg-config`, `libpcsclite`, or link failures on the `pcsc` crate.

### Pitfall 2: cargo test in CI without a YubiKey hangs or fails

**What goes wrong:** Any test calling `ykman` or `gpg --card-status` will fail (tool absent), return empty output (no device), or hang.

**Why it happens:** CI runners have no physical YubiKey.

**How to avoid:** All tests must use pre-captured string fixtures. Gate any live-hardware test behind `#[cfg(feature = "device-tests")]`. The `device-tests` feature is already defined in `Cargo.toml`. The CI `cargo test` command must NOT include `--features device-tests`.

**Warning signs:** CI test suite hangs; `gpg --card-status` returns error in test output.

### Pitfall 3: Fixed/CachedFixed touch policy is irreversible without key deletion

**What goes wrong:** User accidentally sets `fixed` touch policy and cannot change it without deleting their private key.

**Why it happens:** `ykman` does not warn about irreversibility when `--force` is passed.

**How to avoid:** Check `TouchPolicy::is_irreversible()` before spawning ykman. If true, show double-confirmation overlay (matching factory reset pattern in `app.rs`) before proceeding.

### Pitfall 4: ykman --device requires plain integer string, not quoted

**What goes wrong:** `cmd.args(["--device", "\"13390292\""])` fails with "invalid integer".

**Why it happens:** `Command::args()` does not shell-expand — quotes become part of the argument value.

**How to avoid:** Pass `serial.to_string()` directly: `cmd.args(["--device", &serial.to_string()])`.

### Pitfall 5: Release workflow artifact naming collisions

**What goes wrong:** All three OS jobs upload a file named `yubitui` and artifact names collide.

**Why it happens:** Default artifact names are not OS-scoped.

**How to avoid:** Use matrix variable in artifact name: `yubitui-${{ matrix.artifact_name }}`. Define `artifact_name` per matrix entry: `yubitui-linux-amd64`, `yubitui-macos-amd64`, `yubitui-windows-amd64.exe`.

### Pitfall 6: Windows binary extension in release upload paths

**What goes wrong:** Release workflow tries to upload `target/release/yubitui` on Windows but the file is `target/release/yubitui.exe`.

**How to avoid:** Define `binary_extension: ""` for Linux/macOS and `binary_extension: ".exe"` for Windows in the matrix. Reference as `yubitui${{ matrix.binary_extension }}` in upload path.

### Pitfall 7: CI workflow missing cargo clippy step

**What goes wrong:** Clippy regressions accumulate silently since the existing CI does not run clippy.

**How to avoid:** Add `cargo clippy -- -D warnings` to the CI job. It already passes locally — add it to CI as part of the Phase 3 workflow update.

### Pitfall 8: Fingerprint slice panic in keys.rs

**What goes wrong:** `src/ui/keys.rs` line 97 does `&sig.fingerprint[..16]` — if a fingerprint is shorter than 16 characters (e.g., empty string from a parsing edge case), this panics.

**How to avoid:** When writing parser tests, include test cases for empty/short fingerprints. The render code should use `sig.fingerprint.get(..16).unwrap_or(&sig.fingerprint)` defensively.

---

## Code Examples

Verified patterns from codebase audit + direct ykman 5.0.1 testing:

### Making parse_pin_status testable

```rust
// src/yubikey/pin.rs — change fn to pub fn

pub fn parse_pin_status(output: &str) -> Result<PinStatus> {  // was private
    // ... existing implementation unchanged ...
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pin_status_all_healthy() {
        let output = "PIN retry counter : 3 3 0\n";
        let r = parse_pin_status(output).unwrap();
        assert_eq!(r.user_pin_retries, 3);
        assert_eq!(r.admin_pin_retries, 3);
        assert_eq!(r.reset_code_retries, 0);
        assert!(!r.user_pin_blocked);
        assert!(!r.admin_pin_blocked);
    }

    #[test]
    fn test_parse_pin_status_user_blocked() {
        let output = "PIN retry counter : 0 3 0\n";
        let r = parse_pin_status(output).unwrap();
        assert_eq!(r.user_pin_retries, 0);
        assert!(r.user_pin_blocked);
        assert!(!r.admin_pin_blocked);
    }

    #[test]
    fn test_parse_pin_status_no_counter_line() {
        // gpg output without counter line -> defaults
        let output = "General key info..: [none]\nVersion ..........: 3.4\n";
        let r = parse_pin_status(output).unwrap();
        assert_eq!(r.user_pin_retries, 3);  // default
        assert!(!r.user_pin_blocked);
    }
}
```

### Parsing touch policies from ykman openpgp info output

```rust
// src/yubikey/touch_policy.rs

pub fn parse_touch_policies(output: &str) -> TouchPolicies {
    let mut policies = TouchPolicies::default();
    let mut in_touch_section = false;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Touch policies:") {
            in_touch_section = true;
            continue;
        }
        if in_touch_section {
            if trimmed.starts_with("Signature key:") {
                policies.signature = TouchPolicy::from_str(
                    trimmed.split(':').nth(1).unwrap_or("")
                );
            } else if trimmed.starts_with("Encryption key:") {
                policies.encryption = TouchPolicy::from_str(
                    trimmed.split(':').nth(1).unwrap_or("")
                );
            } else if trimmed.starts_with("Authentication key:") {
                policies.authentication = TouchPolicy::from_str(
                    trimmed.split(':').nth(1).unwrap_or("")
                );
            } else if trimmed.starts_with("Attestation key:") {
                policies.attestation = TouchPolicy::from_str(
                    trimmed.split(':').nth(1).unwrap_or("")
                );
            } else if !trimmed.is_empty() && !trimmed.contains(':') {
                in_touch_section = false;
            }
        }
    }
    policies
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_touch_policies_all_off() {
        let output = "Touch policies:\n  Signature key:      Off\n  Encryption key:     Off\n  Authentication key: Off\n  Attestation key:    Off\n";
        let p = parse_touch_policies(output);
        assert_eq!(p.signature, TouchPolicy::Off);
        assert_eq!(p.encryption, TouchPolicy::Off);
        assert!(!p.signature.is_irreversible());
    }

    #[test]
    fn test_parse_touch_policies_sig_fixed() {
        let output = "Touch policies:\n  Signature key:      Fixed\n  Encryption key:     On\n  Authentication key: Cached\n  Attestation key:    Off\n";
        let p = parse_touch_policies(output);
        assert_eq!(p.signature, TouchPolicy::Fixed);
        assert!(p.signature.is_irreversible());
        assert_eq!(p.encryption, TouchPolicy::On);
        assert_eq!(p.authentication, TouchPolicy::Cached);
    }

    #[test]
    fn test_parse_touch_policies_no_section() {
        // Output without touch policies section -> all default (Off)
        let output = "OpenPGP version: 3.4\n";
        let p = parse_touch_policies(output);
        assert_eq!(p.signature, TouchPolicy::Off);
    }
}
```

### CI matrix with 3 OS runners

```yaml
# .github/workflows/rust.yml (replacement)

name: Rust

on:
  push:
    branches: [ "main" ]
  pull_request:
    branches: [ "main" ]

env:
  CARGO_TERM_COLOR: always

jobs:
  build-and-test:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            binary_extension: ""
            artifact_name: yubitui-linux-amd64
          - os: macos-latest
            binary_extension: ""
            artifact_name: yubitui-macos-amd64
          - os: windows-latest
            binary_extension: ".exe"
            artifact_name: yubitui-windows-amd64.exe
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Install system dependencies (Linux only)
        if: runner.os == 'Linux'
        run: sudo apt-get update && sudo apt-get install -y libpcsclite-dev

      - name: Build
        run: cargo build --release --verbose

      - name: Run tests (no device)
        run: cargo test --verbose

      - name: Clippy
        run: cargo clippy -- -D warnings

      - name: Upload binary
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact_name }}
          path: target/release/yubitui${{ matrix.binary_extension }}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Linux-only CI | 3-OS matrix CI | Phase 3 | Catches Windows/macOS regressions |
| Zero unit tests | Parser unit tests (inline `#[cfg(test)]`) | Phase 3 | Makes future refactoring safe |
| Single-key detection via gpg | Multi-key enumeration via ykman list | Phase 3 | Supports users with multiple YubiKeys |
| Touch policy: not displayed | Touch policy: displayed + settable | Phase 3 | Power-user feature |
| No attestation | Attestation PEM via popup | Phase 3 | Verify on-device key generation |

**Deprecated/outdated:**
- `gpg --card-status` for multi-key detection: GPG only accesses the first card seen. `ykman list --serials` is the correct approach for enumerating multiple connected YubiKeys.
- No clippy in CI: the existing `rust.yml` omits clippy — Phase 3 adds it.

---

## Open Questions

1. **Touch policy — Admin PIN entry strategy**
   - What we know: `ykman openpgp keys set-touch` requires `--admin-pin TEXT` or interactive prompt. Drop-to-terminal is the established pattern.
   - Recommendation: Drop to alternate screen (matching `execute_pin_operation`), let ykman prompt interactively with `--force`. This avoids building a PIN input widget and reuses the established pattern.

2. **Attestation PEM — raw display vs. parsed**
   - What we know: PEM certs are ~800 bytes of base64. The `render_popup()` widget supports wrapping text.
   - Recommendation: Show raw PEM in scrollable popup. Do not add `x509-parser` dependency for v1.0. The user can copy and parse externally if needed.

3. **Multi-key UI — scope of switcher**
   - What we know: Dashboard is the natural home for a key switcher. Other screens operate on the currently selected key.
   - Recommendation: `Tab` on Dashboard cycles through `yubikey_states`. Dashboard header shows current serial + model. Other screens inherit the selected key's state.

4. **Fingerprint slice safety in keys.rs**
   - What we know: `src/ui/keys.rs` line 97 does `&sig.fingerprint[..16]` — can panic if fingerprint is shorter than 16 chars.
   - Recommendation: Fix defensively in Plan 03-01 when making parsers public and adding tests — add a test case for short/empty fingerprints that will catch this.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| ykman CLI | Touch policy, attestation, multi-key, existing features | Yes | 5.0.1 | Show "ykman not found" error |
| gpg CLI | Card status, PIN ops, OpenPGP state | Yes | present | No fallback |
| libpcsclite-dev | CI Linux build | CI-only | apt-get | Install in workflow (already there) |
| PCSC.framework | CI macOS build | macOS-native | auto | No install needed |
| winscard.lib | CI Windows build | Windows-native | auto | No install needed |
| cargo/rustc | Build + test | Yes | 1.94.0 | — |
| GitHub Actions ubuntu-latest | CI | CI runner | — | — |
| GitHub Actions macos-latest | CI (new) | CI runner | — | — |
| GitHub Actions windows-latest | CI (new) | CI runner | — | — |

**Missing dependencies with no fallback:** None — all runtime dependencies are available or auto-provided by the OS.

**CI-specific notes:**
- `ubuntu-latest` requires manual `libpcsclite-dev` install (already in existing workflow).
- `macos-latest` and `windows-latest` do not require any extra PCSC installation.

---

## Validation Architecture

> `nyquist_validation` is explicitly `false` in `.planning/config.json` — this section is skipped.

---

## Sources

### Primary (HIGH confidence)
- Direct `cargo test` execution — confirmed 0 existing tests, clean build (2026-03-24)
- Direct `cargo clippy -- -D warnings` execution — confirmed 0 warnings (2026-03-24)
- Source code audit: `src/yubikey/*.rs`, `src/ui/keys.rs`, `src/app.rs`, `Cargo.toml`, `.github/workflows/rust.yml` — confirmed all parser visibility, detection architecture, CI state
- Direct `ykman 5.0.1` execution on development machine — confirmed touch policy output format, `list --serials` output format, `attest` subcommand signature

### Secondary (MEDIUM confidence)
- GitHub Actions documentation — matrix strategy, conditional steps, artifact upload — standard patterns confirmed by project already using `actions/checkout@v4`
- `pcsc` crate cross-platform behavior (Linux pkg-config, macOS PCSC.framework, Windows winscard) — consistent with crate documentation

### Tertiary (LOW confidence)
- None required — all critical claims verified by direct code/tool inspection.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; verified existing crate versions in Cargo.toml
- Architecture: HIGH — all patterns verified against actual source code + ykman 5.0.1 CLI output
- Parser visibility audit: HIGH — confirmed by reading all four parser files
- CI state: HIGH — confirmed by reading existing `rust.yml`
- Pitfalls: HIGH — CI pitfalls verified by inspecting existing workflow; touch policy pitfalls verified from ykman help + code review

**Research date:** 2026-03-24
**Valid until:** 2026-06-24 (stable ykman CLI surface; GitHub Actions runners are stable)
