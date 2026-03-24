# Phase 3: Advanced YubiKey Features - Research

**Researched:** 2026-03-24
**Domain:** YubiKey CLI integration (ykman), Rust unit testing, GitHub Actions CI matrix, binary release distribution
**Confidence:** HIGH

---

## Summary

Phase 3 adds power-user features (touch policy, multi-key support, attestation) and ships the project as a properly tested, CI-validated release. The codebase is already in a strong state: `cargo build` and `cargo clippy -- -D warnings` both pass cleanly, but zero unit tests exist. All YubiKey operations are already channeled through `ykman` and `gpg` CLI calls, so the testing strategy focuses on testing the pure-Rust parsers (which don't touch hardware) rather than the command execution itself.

The three feature areas all rely on `ykman openpgp` subcommands: `ykman openpgp info` already parsed in Phase 2 for key attributes exposes touch policies in the same output; `ykman list --serials` gives one serial per line for multi-key enumeration; and `ykman openpgp keys attest <slot> -` streams a PEM certificate to stdout for attestation. The detection layer already calls `ykman` via `find_ykman()` which handles both PATH and the Windows well-known install path — multi-key support extends this by iterating over `ykman list --serials` output and adding `--device <serial>` to subsequent commands.

The existing CI workflow is Linux-only (`ubuntu-latest`) with no macOS or Windows runners, no release job, and no `cargo clippy` step. Expanding it to a 3-OS matrix is straightforward with GitHub Actions `strategy.matrix`, but Windows needs `pcsclite` handled differently (not `apt-get`), and macOS needs no extra system dependency. Release binaries via `cross` or native runners are both viable; native runners (one job per OS) are simpler and avoid Docker complexity for a Windows+Rust project.

**Primary recommendation:** Implement in 4 plans — (1) unit tests for all parsers, (2) touch policy UI + backend, (3) multi-key detection + switcher UI, (4) attestation display + CI matrix + release workflow.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.29 | TUI rendering | Already in project |
| crossterm | 0.28 | Terminal events | Already in project |
| anyhow | 1.0 | Error handling | Already in project |
| ykman CLI | 5.0.1 (on dev machine) | YubiKey management commands | Yubico-official tool; used throughout |

### Supporting (already present in Cargo.toml)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| mockall | 0.13 | Mock traits in unit tests | Mock CLI output sources |
| tempfile | 3.13 | Temp files in tests | Attestation cert file tests |
| assert_cmd | 2.0 | Test CLI invocations | Integration-level command tests |

### No New Dependencies Required
All phase 3 features are implementable with the existing dependency set. No new crates need to be added.

**Version verification:** `cargo 1.94.0`, `rustc 1.94.0 (2026-03-02)`, `ykman 5.0.1` on development machine.

---

## Architecture Patterns

### Recommended Project Structure (additions only)

```
src/
├── yubikey/
│   ├── touch_policy.rs    # NEW: touch policy parsing + set command
│   ├── attestation.rs     # NEW: attestation cert fetch + parse
│   └── detection.rs       # MODIFY: multi-key enumeration
├── ui/
│   ├── keys.rs            # MODIFY: touch policy display, attestation view
│   └── dashboard.rs       # MODIFY: multi-key switcher indicator
└── app.rs                 # MODIFY: selected_yubikey_serial field

tests/
├── parser_tests.rs        # NEW: unit tests for all pure-Rust parsers
```

### Pattern 1: Separate pure-Rust parsers from command dispatch

**What:** Every `parse_*` function that transforms a `&str` into a Rust struct must be `pub` and live in its own module. Command execution (spawning ykman/gpg) lives in a separate function that calls the parser.

**When to use:** Everywhere a CLI output is parsed. This is already partially true (e.g., `parse_card_status`, `parse_piv_info`, `parse_ykman_openpgp_info`), but the functions are currently private `fn` — they need to become `pub fn` so tests can call them directly.

**Example:**
```rust
// src/yubikey/piv.rs
pub fn parse_piv_info(output: &str) -> PivState { ... }  // make public

// tests/parser_tests.rs
#[test]
fn test_parse_piv_info_empty() {
    let result = yubitui::yubikey::piv::parse_piv_info("");
    assert!(result.slots.is_empty());
}
```

### Pattern 2: Multi-key detection via ykman list --serials

**What:** Enumerate connected YubiKeys by serial number, then pass `--device <serial>` to all subsequent ykman subcommands.

**When to use:** Startup detection and key switching.

**Example:**
```rust
// Verified by direct ykman 5.0.1 testing
pub fn list_connected_serials() -> Result<Vec<u32>> {
    let output = Command::new(find_ykman()?)
        .args(["list", "--serials"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let serials = stdout
        .lines()
        .filter_map(|l| l.trim().parse::<u32>().ok())
        .collect();
    Ok(serials)
}
```

### Pattern 3: Touch policy from ykman openpgp info

**What:** `ykman openpgp info` already outputs touch policies in its structured output. Extend the existing `parse_ykman_openpgp_info` function to capture the `Touch policies:` section. Setting touch policy uses `ykman openpgp keys set-touch <slot> <policy>`.

**ykman openpgp info touch policy output format (verified):**
```
Touch policies:
  Signature key:      Off
  Encryption key:     Off
  Authentication key: Off
  Attestation key:    Off
```

**Valid slots for set-touch:** `sig`, `enc`, `aut`, `att`
**Valid policies:** `on`, `off`, `fixed`, `cached`, `cached-fixed`

**Setting touch policy requires Admin PIN.** Command signature:
```
ykman openpgp keys set-touch <slot> <policy> --admin-pin <pin> [--force]
```
Without `--force`, ykman prompts interactively. With `--force`, it sets without prompting but still requires `--admin-pin`.

**Important:** Setting `fixed` or `cached-fixed` cannot be reversed without deleting the private key. The UI must warn users about this.

### Pattern 4: Attestation via ykman openpgp keys attest

**What:** Generates a PEM certificate proving the key was generated on the device. Writes to stdout when `CERTIFICATE` argument is `-`.

**Command:**
```
ykman openpgp keys attest <slot> - [--pin <pin>] [--format PEM|DER]
```

**Slots:** `sig`, `enc`, `aut` (not `att`)

**Failure modes (verified):**
- Slot is empty (no key loaded): command exits non-zero with error message
- PIN blocked: command exits non-zero
- Key was imported, not generated on-device: command exits non-zero

**Display:** The PEM output is a certificate block. The TUI should show it in a popup overlay (same pattern as SSH pubkey popup from Phase 2). The most useful detail to display is the issuer/subject parsed from the PEM, but displaying the raw PEM with a "copy" instruction is also acceptable.

### Pattern 5: App-level YubiKey selection state

**What:** The `App` struct currently has a single `yubikey_state: Option<YubiKeyState>`. For multi-key support, this becomes a list plus an index.

```rust
pub struct App {
    // existing fields...
    yubikey_states: Vec<YubiKeyState>,
    selected_yubikey_idx: usize,
}
```

The `detect_yubikeys()` function in `detection.rs` already returns `Vec<YubiKeyInfo>` but the caller discards all but the first. Expanding this requires passing the serial to `get_openpgp_state()`, `get_piv_state()`, and `get_pin_status()` so they use `ykman --device <serial>`.

### Anti-Patterns to Avoid

- **Making attestation interactive:** `ykman openpgp keys attest` should always use `-` for stdout and pass `--pin` as a flag (not prompt). Prompting in the TUI event loop will deadlock.
- **Storing Admin PIN in memory:** Do not persist the Admin PIN in `App` state between operations. Prompt once via the "drop to terminal" pattern, use, discard.
- **Passing fixed/cached-fixed without confirmation:** The UI must show a destructive-action warning (matching the factory reset confirmation pattern from Phase 2) before setting `fixed` or `cached-fixed` touch policy.
- **Blocking the TUI event loop on ykman:** All ykman operations that need to prompt for PIN should use the "drop to alternate screen" pattern already established in `execute_key_operation` and `execute_pin_operation`.
- **Testing with live hardware in CI:** Tests that require a physical YubiKey must be gated behind the `device-tests` feature flag already defined in `Cargo.toml`. CI must NOT enable this feature.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Multi-key enumeration | Custom USB scanning | `ykman list --serials` | Handles USB, NFC, reader abstraction |
| Touch policy management | Direct APDU commands | `ykman openpgp keys set-touch` | Handles all policy variants and error cases |
| Attestation certificate generation | OpenPGP APDU attestation | `ykman openpgp keys attest` | Yubico-specific extension; APDU flow is undocumented |
| Cross-platform binary builds | Custom build scripts | GitHub Actions native runners (ubuntu, macos, windows) | Handles toolchain installation automatically |

**Key insight:** This codebase's architecture deliberately externalizes all YubiKey protocol complexity to `ykman` and `gpg`. Phase 3 should stay consistent — there is no reason to introduce `yubikey` crate APDU calls when `ykman` CLi equivalents exist for all three features.

---

## Common Pitfalls

### Pitfall 1: pcsclite on GitHub Actions macOS and Windows

**What goes wrong:** CI fails with "could not find library pcsclite" on non-Linux runners.

**Why it happens:** The `pcsc` and `card-backend-pcsc` crates link against system pcsclite. The existing CI only runs on Ubuntu and installs `libpcsclite-dev`. macOS ships `PCSC.framework` natively (no install needed). Windows needs `winscard.dll` which ships with Windows but the Rust link target must be `winscard`.

**How to avoid:**
- Linux: keep `sudo apt-get install -y libpcsclite-dev`
- macOS: no extra step; `pcsc` crate links to `PCSC.framework` automatically via `pkg_config` feature or framework link
- Windows: no extra step; `pcsc` crate on Windows links to `winscard.lib` which is always present

**Warning signs:** CI errors mentioning `pkg-config`, `libpcsclite`, or link errors on the `pcsc` crate.

### Pitfall 2: `cargo test` on CI without a YubiKey hangs or fails

**What goes wrong:** Any test that calls `Command::new("ykman")` or `Command::new("gpg").arg("--card-status")` in a test context will either fail (tool not installed), return empty output (no device), or hang waiting for a smart card reader.

**Why it happens:** CI runners have no YubiKey plugged in.

**How to avoid:** All parser functions must be tested with pre-captured string fixtures, not live commands. Use `#[cfg(feature = "device-tests")]` on any test that actually runs ykman or gpg. The `device-tests` feature is already defined in `Cargo.toml`. The CI `cargo test` command must NOT include `--features device-tests`.

**Warning signs:** CI test suite hangs or `gpg --card-status` returns error in test output.

### Pitfall 3: Touch policy confirmation for Fixed/Cached-Fixed is irreversible

**What goes wrong:** User accidentally sets `fixed` or `cached-fixed` touch policy and cannot change it without deleting their private key.

**Why it happens:** ykman does not warn about irreversibility when `--force` is passed.

**How to avoid:** The UI layer must check the requested policy before spawning ykman. If policy is `fixed` or `cached-fixed`, show a destructive-action confirmation overlay (matching the factory reset double-confirmation from Phase 2) before proceeding.

### Pitfall 4: ykman --device requires integer serial, not quoted string

**What goes wrong:** `Command::new("ykman").args(["--device", "\"13390292\"", ...])` fails with "invalid integer".

**Why it happens:** Shell quoting is not needed when using `Command::args()` — but if the serial is stringified with surrounding quotes it will fail.

**How to avoid:** Pass the serial as `serial.to_string()` (a plain decimal integer string) directly.

### Pitfall 5: Release workflow artifact naming collisions

**What goes wrong:** All three OS jobs produce a binary named `yubitui` and the upload-artifact step overwrites or collides.

**Why it happens:** Default artifact names are not OS-scoped.

**How to avoid:** Name artifacts per-OS: `yubitui-linux-amd64`, `yubitui-macos-amd64`, `yubitui-windows-amd64.exe`. Use matrix variable in artifact name: `yubitui-${{ matrix.os }}-amd64`.

### Pitfall 6: Windows binary extension in release paths

**What goes wrong:** The release workflow tries to upload `target/release/yubitui` on Windows but the actual file is `target/release/yubitui.exe`.

**Why it happens:** Windows requires `.exe` extension; Linux/macOS do not.

**How to avoid:** In the GitHub Actions matrix, define a `binary_extension` matrix variable: `''` for Linux/macOS, `.exe` for Windows. Reference as `yubitui${{ matrix.binary_extension }}` in artifact upload path.

---

## Code Examples

Verified patterns from direct ykman 5.0.1 testing:

### Parsing touch policy from ykman openpgp info output

```rust
// Output section verified on ykman 5.0.1:
//   Touch policies:
//     Signature key:      Off
//     Encryption key:     Off
//     Authentication key: Off
//     Attestation key:    Off

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
            // Lines like "  Signature key:      Off"
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
                // Non-empty line without colon = left touch section
                in_touch_section = false;
            }
        }
    }
    policies
}
```

### Multi-key detection

```rust
// Verified: ykman list --serials outputs one decimal integer per line
// Example output: "13390292\n"
pub fn list_connected_serials() -> Result<Vec<u32>> {
    let ykman = find_ykman()?;
    let output = Command::new(ykman)
        .args(["list", "--serials"])
        .output()?;
    if !output.status.success() {
        return Ok(vec![]);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let serials = stdout
        .lines()
        .filter_map(|l| l.trim().parse::<u32>().ok())
        .collect();
    Ok(serials)
}
```

### Attestation fetch (PEM to stdout)

```rust
// Verified: ykman openpgp keys attest sig - writes PEM to stdout
// Returns Err if slot is empty or key was imported (not generated on-device)
pub fn get_attestation_cert(slot: &str, serial: Option<u32>) -> Result<String> {
    let ykman = find_ykman()?;
    let mut cmd = Command::new(&ykman);
    if let Some(s) = serial {
        cmd.args(["--device", &s.to_string()]);
    }
    cmd.args(["openpgp", "keys", "attest", slot, "-"]);
    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Attestation failed for slot {}: {}", slot, stderr);
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
```

### CI matrix with 3 OS runners (GitHub Actions)

```yaml
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
```

### Unit test structure for parsers

```rust
// tests/parser_tests.rs  OR  inline in each module as #[cfg(test)]
// No hardware required — tests use fixture strings

#[test]
fn test_parse_pin_status_blocked_user() {
    let output = "PIN retry counter : 0 3 0\n";
    let result = yubitui::yubikey::pin::parse_pin_status(output).unwrap();
    assert_eq!(result.user_pin_retries, 0);
    assert!(result.user_pin_blocked);
    assert!(!result.admin_pin_blocked);
}

#[test]
fn test_parse_touch_policies_all_off() {
    let output = "Touch policies:\n  Signature key:      Off\n  Encryption key:     Off\n  Authentication key: Off\n  Attestation key:    Off\n";
    let result = parse_touch_policies(output);
    assert_eq!(result.signature, TouchPolicy::Off);
}

#[test]
fn test_parse_ykman_openpgp_info_no_keys() {
    // ykman openpgp info when no keys are loaded
    let output = "SIG key:\nENC key:\nAUT key:\n";
    let result = parse_ykman_openpgp_info(output).unwrap();
    assert!(result.signature.is_none());
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Linux-only CI | 3-OS matrix CI | Phase 3 | Catches Windows/macOS regressions |
| Zero unit tests | Parser unit tests | Phase 3 | Makes future refactoring safe |
| Single-key detection | Multi-key enumeration | Phase 3 | Supports users with multiple YubiKeys |

**Deprecated/outdated:**
- `gpg --card-status` for multi-key detection: GPG only accesses the first card seen. `ykman list --serials` is the correct cross-platform approach for enumerating multiple connected YubiKeys.

---

## Open Questions

1. **Touch policy — should it require Admin PIN in the TUI?**
   - What we know: `ykman openpgp keys set-touch` requires `--admin-pin TEXT` as a flag. Without it and without `--force`, it prompts interactively.
   - What's unclear: Should the TUI drop to alternate screen and let ykman prompt, or should we build a PIN input field?
   - Recommendation: Drop to alternate screen (matching the existing PIN operation pattern). Add `--force` flag once user has confirmed the action in the TUI, and drop to terminal for Admin PIN entry.

2. **Attestation PEM display — parse or show raw?**
   - What we know: PEM certificates are long (~800 bytes base64). The popup pattern supports scrollable text.
   - What's unclear: Whether parsing the cert for issuer/serial adds meaningful value for the target audience.
   - Recommendation: Show raw PEM in a scrollable popup with a "copy to clipboard" or "save to file" affordance. Parsing the cert requires adding an X.509 crate (e.g., `x509-parser`) which adds a dependency. For v1.0, raw PEM display is sufficient.

3. **Multi-key UI — per-screen selector or global switcher?**
   - What we know: The `App` struct currently has a single `yubikey_state`. The dashboard is the natural place to show which key is active.
   - What's unclear: Whether users need to switch keys mid-session from any screen, or only from the dashboard.
   - Recommendation: Add a key switcher to the Dashboard only (e.g., `Tab` cycles through connected keys, dashboard header shows current serial). Other screens operate on the selected key.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| ykman CLI | All new features + existing | yes | 5.0.1 | No fallback; show "ykman not found" error |
| gpg CLI | Card status, PIN ops | yes | (present) | No fallback |
| libpcsclite-dev | CI Linux build | CI-only | — | `apt-get install` in workflow |
| PCSC.framework | CI macOS build | macOS-native | — | No install needed |
| winscard.lib | CI Windows build | Windows-native | — | No install needed |
| cargo/rustc | Build + test | yes | 1.94.0 | — |

**Missing dependencies with no fallback:**
- None — all runtime dependencies are available or auto-provided by the OS.

**CI-specific notes:**
- GitHub Actions `ubuntu-latest` requires manual `libpcsclite-dev` install (already in existing workflow).
- GitHub Actions `macos-latest` and `windows-latest` do not require any extra PCSC installation.

---

## Validation Architecture

> nyquist_validation is explicitly `false` in `.planning/config.json` — this section is skipped.

---

## Sources

### Primary (HIGH confidence)
- Direct `ykman 5.0.1` execution on development machine — touch policy commands, list --serials, attest --help, openpgp info output format
- Direct `cargo test` execution — confirms 0 existing tests, clean build
- Direct `cargo clippy -- -D warnings` execution — confirms no current lint failures
- Source code audit: `src/yubikey/*.rs`, `src/app.rs`, `Cargo.toml`, `.github/workflows/rust.yml`

### Secondary (MEDIUM confidence)
- GitHub Actions documentation (matrix strategy, conditional steps, artifact upload) — standard patterns confirmed by project already using `actions/checkout@v4`
- `pcsc` crate cross-platform behavior (Linux pkg-config, macOS framework, Windows winscard) — consistent with crate documentation and common knowledge for this crate family

### Tertiary (LOW confidence)
- X.509 parsing options (skipped — raw PEM display recommended for v1.0, no parsing needed)

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; verified existing crate versions in Cargo.toml
- Architecture: HIGH — all patterns verified against actual ykman 5.0.1 CLI output on target machine
- Pitfalls: HIGH — CI pitfalls verified by inspecting existing workflow; touch policy pitfalls verified from ykman help text
- Unit test strategy: HIGH — confirmed 0 existing tests; parser functions identified for exposure as `pub`

**Research date:** 2026-03-24
**Valid until:** 2026-06-24 (stable ykman CLI surface; GitHub Actions runners are stable)
