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

---

## Milestone: v1.1 — Accessible to New Users

**Shipped:** 2026-03-29
**Phases:** 8 (6–13) | **Plans:** 35 | **Timeline:** 2026-03-26 → 2026-03-29 (3 days)

### What Was Built

- Model/View split: `src/model/` zero ratatui, CI lint boundary, serde::Serialize on all types
- Full textual-rs migration: all 7 screens rebuilt as components with Footer, Buttons, themes
- OATH/TOTP screen: live codes, countdown, add/delete wizard, password-protected vault
- FIDO2 screen: PIN management, resident credential list/delete, CTAPHID factory reset
- OTP slot read-only view
- Per-screen `?` help panels and protocol glossary
- Factory-default detection heuristic and onboarding checklist for new users
- OpenPGP slot deletion (Admin PIN + RSA attribute trick)
- PIV cert/key deletion (3DES management key auth, firmware 5.7+ gate)
- DataTable, Button, ProgressBar, Markdown on every screen; consistent bracket badges
- 161 unit/snapshot tests (74 added this milestone)

### What Worked

- **Multi-instance collab coordination** — @mac, @win, @kali, @macos-live-tester as parallel workers with collab message passing caught cross-platform issues (PinInputWidget height collapse on Windows) before they reached users.
- **Phase 12 gap closure plans (12-04, 12-05)** — Verifier caught missing refresh wiring after the main delete workflow landed. Dedicated gap plans kept the core plan clean while closing the holes systematically.
- **textual-rs component pattern** — Once established in Phase 8, each new screen (OATH, FIDO2, OTP) followed the same widget pattern with minimal deviation. Decision memos in STATE.md prevented drift.
- **Snapshot tests as visual regression guard** — 161 tests run in 0.1s and catch render regressions immediately. Phase 13 used snapshots to verify DataTable/Button layout changes across all 10 screens in one pass.
- **Yolo mode with granularity: coarse** — Removed friction from routine decisions; workers could execute without confirming every step while still respecting blocking deviations.

### What Was Inefficient

- **textual-rs not on crates.io** — Required git dependency throughout v1.1, adding friction at every worktree/clone. Resolved by textual-rs team publishing to crates.io during the milestone, but the gap caused deviation handling in nearly every phase.
- **Snapshot file conflicts on parallel branches** — Multiple workers updating `.snap` files on separate worktrees created merge conflicts. Pattern: snapshot updates should be committed with the plan that caused them, not deferred.
- **STATE.md overwritten by gsd-tools** — `milestone complete` CLI command reset STATE.md to stale counts. Needed manual correction. The tool should preserve higher `completed_phases` counts, not regress them.

### Patterns Established

- `push_screen_deferred` for all screen transitions in textual-rs (sync context constraint)
- `on_mount()` Cell<Option<WidgetId>> for matching worker source_id in async events (ResetGuidanceScreen)
- `detect_all()` for post-delete refresh (full state, not partial); pop+push-fresh-screen pattern for refresh
- F5 for FIDO2 refresh (R reserved for factory reset); R for all other screen refreshes
- `Vertical{height:1fr}` children must be direct, not wrapped — collapses to 0 in screen-stack
- 3-state model for credential lists: `None`=locked, `Some([])`=empty, `Some(vec)`=populated

### Key Lessons

1. **collab coordination scales** — Running 4 parallel workers with clear role separation (mac=primary executor, win=Windows verifier, kali=Linux verifier, macos-live-tester=hardware) caught more issues than single-instance execution. The overhead is low when collab messages are signal-only (API changes, blockers).
2. **Firmware-gate early** — PIV key delete (5.7+ only) required firmware version check that touched multiple layers. Build the gate first, then the feature — not the reverse.
3. **Research phase pays off for novel protocols** — FIDO2 CTAPHID framing was the highest-risk part of v1.1. The spike in planning caught that ctap-hid-fido2 didn't expose `authenticatorReset` before committing to the approach.

### Cost Observations

- Sessions: 3-day sprint (2026-03-26 → 2026-03-29), 201 commits
- Multi-instance collab: 4 active workers across mac/win/kali/macos-live-tester
- Yolo mode / coarse granularity throughout
