---
phase: 03-advanced-yubikey-features
plan: 04
subsystem: infra
tags: [github-actions, ci, release, cross-platform, cargo]

requires: []
provides:
  - 3-OS CI matrix (Linux, macOS, Windows) with build, test, and clippy
  - Release workflow building platform-specific binaries on version tag push
  - GitHub Release creation with auto-generated notes via softprops/action-gh-release
affects: [release-readiness, ci]

tech-stack:
  added: [github-actions, softprops/action-gh-release@v2, actions/upload-artifact@v4, actions/download-artifact@v4]
  patterns: [matrix-ci, conditional-os-deps, tag-triggered-release]

key-files:
  created: [.github/workflows/release.yml]
  modified: [.github/workflows/rust.yml]

key-decisions:
  - "CI uses fail-fast: false so all OS results visible even when one fails"
  - "libpcsclite-dev install is Linux-only conditional (macOS uses PCSC.framework, Windows uses winscard.dll natively)"
  - "Release artifact names include OS suffix to avoid collisions (yubitui-linux-amd64, yubitui-macos-amd64, yubitui-windows-amd64.exe)"
  - "Windows release binary has .exe extension via separate binary_name matrix field"
  - "Tests run during release build to ensure released binaries come from tested code"
  - "device-tests feature NOT enabled in any workflow — no YubiKey on CI runners"

patterns-established:
  - "Matrix CI pattern: strategy.matrix.include with per-OS entries for heterogeneous runner config"
  - "Two-stage release: build-release matrix jobs feed into single create-release job via needs:"

requirements-completed: []

duration: 10min
completed: 2026-03-24
---

# Phase 3 Plan 4: CI 3-OS Matrix and Release Workflow Summary

**GitHub Actions CI expanded to Linux/macOS/Windows matrix with clippy, plus tag-triggered release workflow that builds and publishes platform binaries via softprops/action-gh-release**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-03-24T20:44:00Z
- **Completed:** 2026-03-24T20:54:17Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- CI workflow now tests on all 3 OS (Linux, macOS, Windows) with build, test, and clippy
- Release workflow creates platform-specific binaries and a GitHub Release on v* tag push
- Linux-only conditional for libpcsclite-dev; macOS and Windows use native PCSC support

## Task Commits

Each task was committed atomically:

1. **Task 1: Expand CI to 3-OS matrix with clippy** - `81098545` (feat)
2. **Task 2: Create release workflow for binary distribution** - `393c5882` (feat)

## Files Created/Modified

- `.github/workflows/rust.yml` - Expanded from ubuntu-only to 3-OS matrix; renamed job to build-and-test; added fail-fast: false, conditional Linux deps, and cargo clippy step
- `.github/workflows/release.yml` - New release workflow triggering on v* tags; builds release binaries per OS with OS-specific artifact names; creates GitHub Release with all 3 binaries

## Decisions Made

- `fail-fast: false` ensures all three OS results are always visible even when one platform fails
- Linux-only conditional `if: runner.os == 'Linux'` for libpcsclite-dev since macOS and Windows provide PCSC natively
- Artifact names encode OS to prevent download collisions: `yubitui-linux-amd64`, `yubitui-macos-amd64`, `yubitui-windows-amd64.exe`
- Windows binary explicitly named `yubitui.exe` via `binary_name` matrix field
- `permissions: contents: write` required on release workflow to create GitHub Releases
- `device-tests` feature deliberately excluded from all workflows — no YubiKey hardware on CI runners

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required. GitHub Actions runs automatically on push.

## Next Phase Readiness

- CI and release infrastructure are complete for all 3 platforms
- Any subsequent phase can rely on cross-platform CI validation
- To publish a release: push a tag matching `v*` (e.g., `git tag v0.1.0 && git push origin v0.1.0`)

---
*Phase: 03-advanced-yubikey-features*
*Completed: 2026-03-24*
