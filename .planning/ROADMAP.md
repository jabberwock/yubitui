# yubitui Roadmap

## Milestone 1: Production-Ready v1.0

Phases 1-3 are already implemented. Starting from Phase 1 to close remaining gaps.

---

## Phase 1: Polish & Cross-Platform Fixes

**Goal:** Fix known rough edges so the app works correctly on all platforms.

**Scope:**
- Key selection UI: replace hardcoded `available_keys[0]` with an interactive picker
- Wire up `?` help screen with keybinding reference
- Audit all hardcoded paths and platform-specific strings for Windows/Linux/macOS correctness
- Update README roadmap checkboxes to reflect actual implementation state

**Done when:** App runs correctly on Windows with accurate diagnostics, keys can be selected for import, help screen is accessible.

**Plans:** 3 plans

Plans:
- [ ] 01-01-PLAN.md — Interactive key selection UI for import
- [ ] 01-02-PLAN.md — Help screen with keybinding reference
- [ ] 01-03-PLAN.md — Update README roadmap checkboxes

**Requirements:** [KEY-PICKER, HELP-SCREEN, README-SYNC]

---

## Phase 2: Advanced YubiKey Features

**Goal:** Implement Phase 4 from original README — power-user features.

**Scope:**
- Touch policy configuration (view and set touch policy per slot)
- Multiple YubiKey support (detect and switch between connected keys)
- Attestation support (verify key was generated on-device)
- Backup/restore workflow guidance (not key material — operational guidance)

**Done when:** Users can configure touch policy, attestation certificates can be read, multiple keys are handled gracefully.

---

## Phase 3: Testing & Release

**Goal:** Establish test coverage and prepare for public release.

**Scope:**
- Unit tests for all parsers (card status, PIN counter, PIV info)
- Integration test harness with mock gpg output
- CI passes on Linux, macOS, Windows
- Release binary builds via GitHub Actions

**Done when:** `cargo test` passes with meaningful coverage, CI matrix is green on all three platforms.

---

## Backlog

- FIDO2/WebAuthn status display (read-only, no management)
- Configurable refresh interval
- Export key directly to clipboard
