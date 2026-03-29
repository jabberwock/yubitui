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
**Phases:** 8 (phases 6–13) | **Plans:** 34 | **Commits:** ~200

### What Was Built

- **textual-rs migration**: All 7 screens rebuilt as textual-rs components; Pilot snapshot tests replace tmux harness; 160 tests, no hardware
- **Model/View separation**: `src/model/` zero ratatui, CI boundary lint, all types `serde::Serialize`
- **Mouse support**: Region-based click dispatch (reverse iteration, popup-first), scroll, Windows ConPTY graceful degradation
- **OATH/TOTP screen**: Live codes with ProgressBar countdown, Add Account wizard, OATH password prompt
- **FIDO2 screen**: PIN set/change, resident credential list/delete, factory reset with timing guidance
- **OTP slots screen**: Slot 1/2 status view, hardware write-only constraint surfaced
- **Education system**: Per-screen `?` help panels (all 8 screens), Glossary with Markdown rendering
- **Onboarding flow**: Factory-default heuristic detection, guided first-time setup screen
- **Slot delete workflows**: OpenPGP (RSA attribute-change trick), PIV cert (empty PUT DATA), PIV key (MOVE KEY firmware 5.7+)
- **UI polish**: DataTable, Button widgets, status badges, consistent Header→data→spacer→Buttons→Footer layout

### What Worked

- **textual-rs component model** — Switching from raw ratatui to textual-rs widgets gave structural consistency across all screens. The Header/Footer/Button primitives meant every new screen followed the same pattern without extra effort.
- **Pilot snapshot tests** — Replacing tmux E2E with insta+Pilot eliminated process-spawn flakiness entirely. 160 tests run in `cargo test` with no hardware and no timing sensitivity.
- **Multi-worker collab** — @win, @kali, @macos-live-tester running in parallel enabled live hardware verification on 3 platforms simultaneously. Issues surfaced the same day they were introduced.
- **DataTable API discovery** — When the plan documented `DataTable::new(columns, rows)` but the actual API was `DataTable::new(columns)` + `add_row()`, the gap was caught and fixed within the same plan execution rather than during UAT.
- **Gap closure naming discipline** — Phases 12's 12-04/12-05 gap plans targeted exactly the PinInputWidget and card refresh issues without touching unrelated scope.

### What Was Inefficient

- **textual-rs path dep for worktrees** — Multiple worktrees hit a path dependency issue when textual-rs wasn't yet on crates.io. Each worktree needed a manual path override before work could start. Publishing to crates.io (done during v1.1) resolved this.
- **DataTable API mismatch** — Three separate plans discovered the same `add_row()` API discrepancy independently. A single "API orientation" note in the phase CONTEXT.md would have prevented the repeat lookups.
- **STATE.md drift** — STATE.md `stopped_at` was stale by 2 plans at milestone close. Automated state update after each plan completion would prevent this.
- **Phase 11 directory naming** — `11-yubikey-slot-delete-workflow` mislabeled what is actually the OTP/Education phase. A naming collision at worktree time was never corrected. Phase directory names should always match ROADMAP phase names.

### Patterns Established

- `DataTable::new(columns)` + `add_row(&mut self)` — textual-rs DataTable API
- `ctx.quit()` for global quit (added in textual-rs 0.3.5; `q` global was removed in 0.3.3)
- `push_screen_deferred()` for wizard sub-screens; `pop_screen()` for return
- Factory-default heuristic: `fido2.pin_is_set == false && oath.credentials.is_empty() && piv.slots.is_empty()`
- OpenPGP slot delete: PUT DATA RSA4096 → PUT DATA RSA2048 (attribute-change trick, no DELETE KEY APDU exists)
- PIV key delete: MOVE KEY INS=0xF6 P1=0xFF, firmware 5.7+ only — gate in UI with firmware check
- `des 0.9.0-rc.3` with `cipher = "0.5"` (not 0.4) — cipher version incompatibility trap

### Key Lessons

1. **API orientation in CONTEXT.md pays off** — Documenting the actual textual-rs DataTable API once (rather than re-discovering it per plan) would have saved 3 plan-level lookups across the milestone.
2. **Multi-platform collab is powerful** — Having @win/@kali/@macos-live-tester verifying live simultaneously meant cross-platform bugs were caught same-day. This cadence should be maintained for v2.
3. **Phase directory names must match roadmap** — The phase 11 naming collision created confusion in roadmap analysis tooling. Always verify the directory slug matches the ROADMAP phase name at creation time.
4. **Publish deps before worktree-heavy sprints** — textual-rs path dep issue blocked multiple worktrees at sprint start. Publishing dependencies to registries before parallelizing work avoids the per-worktree manual fix.

### Cost Observations

- Sessions: 3-day sprint (2026-03-26 → 2026-03-29)
- ~200 commits, 304 files changed, 46,543 insertions / 6,222 deletions
- 8 phases, 34 plans — velocity maintained from v1.0
- Multi-worker collab (5 workers): @win, @kali, @macos-live-tester, @textual-rs, @claude-ipc

## Cross-Milestone Trends

| Metric | v1.0 | v1.1 |
|--------|------|------|
| Phases | 5 | 8 |
| Plans | 21 | 34 |
| Tests | 87 | 160 |
| LOC (Rust) | ~10,053 | ~15,732 |
| Sprint (days) | 3 | 3 |
| Commits | 168 | ~200 |
| Gap plans needed | 3 | 2 |
| UAT issues | 7 | 0 (snapshot-verified) |
