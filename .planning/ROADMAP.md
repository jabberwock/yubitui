# yubitui Roadmap

## Milestones

- ✅ **v1.0 Production-Ready** — Phases 1–5 (shipped 2026-03-26)
- ✅ **v1.1 Accessible to New Users** — Phases 6–13 (shipped 2026-03-29)
- 📋 **v2.0** — TBD (start with `/gsd:new-milestone`)

## Phases

<details>
<summary>✅ v1.0 Production-Ready (Phases 1–5) — SHIPPED 2026-03-26</summary>

- [x] Phase 1: Polish & Cross-Platform Fixes (3/3 plans) — completed 2026-03-24
- [x] Phase 2: UX — Menus, Wizards & Bug Fixes (4/4 plans) — completed 2026-03-24
- [x] Phase 3: Advanced YubiKey Features (4/4 plans) — completed 2026-03-24
- [x] Phase 4: Programmatic Subprocess Control (4/4 plans) — completed 2026-03-25
- [x] Phase 5: Native Card Protocol (6/6 plans) — completed 2026-03-26

See full details: `.planning/milestones/v1.0-ROADMAP.md`

</details>

<details>
<summary>✅ v1.1 Accessible to New Users (Phases 6–13) — SHIPPED 2026-03-29</summary>

- [x] Phase 6: Tech Debt + Infrastructure (3/3 plans) — completed 2026-03-26
- [x] Phase 7: Mouse Support + E2E Test Harness (4/4 plans) — completed 2026-03-27
- [x] Phase 8: textual-rs Migration (6/6 plans) — completed 2026-03-27
- [x] Phase 9: OATH/TOTP Screen (4/4 plans) — completed 2026-03-27
- [x] Phase 10: FIDO2 Screen (4/4 plans) — completed 2026-03-28
- [x] Phase 11: OTP Slots + Education + Onboarding (3/3 plans) — completed 2026-03-28
- [x] Phase 12: YubiKey Slot Delete Workflow (5/5 plans) — completed 2026-03-29
- [x] Phase 13: UI Polish (5/5 plans) — completed 2026-03-29

See full details: `.planning/milestones/v1.1-ROADMAP.md`

</details>

### 📋 v2.0 (Planned)

Run `/gsd:new-milestone` to define v2.0 requirements and roadmap.

**Backlog candidates:**
- Phase 999.1: Provisioning wizards — outcome-oriented multi-step flows

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Polish & Cross-Platform Fixes | v1.0 | 3/3 | Complete | 2026-03-24 |
| 2. UX — Menus, Wizards & Bug Fixes | v1.0 | 4/4 | Complete | 2026-03-24 |
| 3. Advanced YubiKey Features | v1.0 | 4/4 | Complete | 2026-03-24 |
| 4. Programmatic Subprocess Control | v1.0 | 4/4 | Complete | 2026-03-25 |
| 5. Native Card Protocol | v1.0 | 6/6 | Complete | 2026-03-26 |
| 6. Tech Debt + Infrastructure | v1.1 | 3/3 | Complete | 2026-03-26 |
| 7. Mouse Support + E2E Test Harness | v1.1 | 4/4 | Complete | 2026-03-27 |
| 8. textual-rs Migration | v1.1 | 6/6 | Complete | 2026-03-27 |
| 9. OATH/TOTP Screen | v1.1 | 4/4 | Complete | 2026-03-27 |
| 10. FIDO2 Screen | v1.1 | 4/4 | Complete | 2026-03-28 |
| 11. OTP Slots + Education + Onboarding | v1.1 | 3/3 | Complete | 2026-03-28 |
| 12. YubiKey Slot Delete Workflow | v1.1 | 5/5 | Complete | 2026-03-29 |
| 13. UI Polish | v1.1 | 5/5 | Complete | 2026-03-29 |

## Backlog

### Phase 999.1: Provisioning wizards — outcome-oriented multi-step flows (BACKLOG)

**Goal:** Outcome-oriented provisioning flows that span applets (e.g. "Set up SSH key with touch policy", "Initial YubiKey setup"). User thinks in terms of goals, not slots. Build from existing keygen wizard pattern. Include: touch policy surfaced upfront, nav affordance hint (1-9 keys), onboarding flow for fresh YubiKey.
**Requirements:** TBD

Plans:
- [ ] TBD (promote with /gsd:review-backlog when ready)

*Consensus feedback from @macos-live-tester, @win, @kali — 2026-03-28*
