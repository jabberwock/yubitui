# yubitui Retrospective

## Milestone: v1.0 — Production-Ready

**Shipped:** 2026-03-26
**Phases:** 5 | **Plans:** 21 | **Commits:** 168

### What Was Built

- Interactive key picker + help screen overlay from any screen
- PIN unblock wizard (4-branch: reset code → admin PIN → factory reset → abort)
- SSH setup wizard with agent config, shell rc, and in-TUI connection test
- Fully programmatic GPG subprocess control (zero terminal escape, --command-fd/--status-fd/--pinentry-mode loopback)
- 7-step on-device key generation wizard
- Native PC/SC APDU protocol: card.rs, BER-TLV parser, T=0 GET RESPONSE chaining
- Touch policy view/set, attestation, multi-YubiKey Tab-switching
- PIV certificates screen with native slot occupancy reads
- 87 unit tests, 3-OS CI matrix, tag-triggered release builds

### What Worked

- **Phase-scoped gap closure plans** — When UAT revealed 7 issues in Phase 5, creating targeted 05-04/05-05/05-06 plans focused work without disrupting the main execution track. Naming as "gap" plans kept intent clear.
- **card.rs as single primitive module** — Centralizing all PC/SC APDU operations in one file made debugging contention issues (SW 0x6B00, scdaemon kills) tractable. Cross-references were always obvious.
- **Read-all-OpenPGP-DOs-before-SELECT-MGMT ordering** — Discovering that SELECT_MGMT corrupts subsequent OpenPGP GET DATA responses was subtle. Documenting it inline prevented reintroduction.
- **T=0 GET RESPONSE chaining** — Implementing this transparently in get_data() meant all callers just work; no caller-side awareness needed.
- **State machine approach for gpg --command-fd** — Writing a proper state machine for gpg interactive sessions (key import, PIN ops) eliminated the pre-buffering failures and infinite-loop risks from earlier attempts.

### What Was Inefficient

- **Phase 5 needed 3 extra gap plans** — UAT surfaced 7 issues that weren't caught before hardware testing. Earlier hardware testing during planning would reduce gap plan churn.
- **Phase 04 VERIFICATION.md was never created** — Functional evidence existed but required Phase 05 verifier to reconstruct it cross-phase. Verification should happen before moving to next phase.
- **cargo fmt drift** — Formatting diffs accumulated across the milestone and were flagged only at audit time. Running `cargo fmt` on modified files as part of plan execution would keep it clean.

### Patterns Established

- `previous_screen: Screen` field for modal overlay return navigation (help, attestation, popup)
- `card::kill_scdaemon()` before any exclusive card connection; gpgconf --launch scdaemon after
- `Vec<YubiKeyState>` with selected index replacing `Option<YubiKeyState>` for multi-key support
- `.cloned()` at render boundary to avoid lifetime propagation into render signatures
- Fixture-based unit tests for all parsers (no hardware required; fast, portable)
- `#[allow(dead_code)]` with explicit comment noting which future plan will wire the item

### Key Lessons

1. **Hardware testing early** — Many Phase 5 gaps (touch policy display, SSH status on load, PIV screen) were visible in UAT but not in planning. A "pre-verify on hardware" step before declaring a phase complete would catch these.
2. **Verify as you go** — Phase 04 skipping VERIFICATION.md created ambiguity that required cross-phase reconstruction. Each phase should complete verification before advancing.
3. **Model identification bugs are subtle** — The YubiKey NEO misidentification (OpenPGP spec 3.4 ≠ firmware 3.x; outer 0x71 TLV unwrap missing) required understanding both the ykman source and hardware response format. Cross-referencing vendor SDK source earlier would have caught it.

### Cost Observations

- Sessions: 3-day sprint
- All phases executed inline (no parallel workstreams needed)
- Yolo mode / coarse granularity — effective for focused sprint work
