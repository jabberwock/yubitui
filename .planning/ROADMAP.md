# yubitui Roadmap

## Milestones

- ✅ **v1.0 Production-Ready** — Phases 1–5 (shipped 2026-03-26)
- ✅ **v1.1 Accessible to New Users** — Phases 6–13 (shipped 2026-03-29)
- 📋 **v1.2 Guided Workflows & Advanced Operations** — Phases 14–17 (in progress)

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

- [x] Phase 6: Tech Debt + Infrastructure (3/3 plans) — completed 2026-03-27
- [x] Phase 7: Mouse Support + E2E Test Harness (4/4 plans) — completed 2026-03-27
- [x] Phase 8: textual-rs Migration (6/6 plans) — completed 2026-03-27
- [x] Phase 9: OATH/TOTP Screen (4/4 plans) — completed 2026-03-27
- [x] Phase 10: FIDO2 Screen (4/4 plans) — completed 2026-03-28
- [x] Phase 11: OTP Slots + Education + Onboarding (3/3 plans) — completed 2026-03-28
- [x] Phase 12: YubiKey Slot Delete Workflow (5/5 plans) — completed 2026-03-29
- [x] Phase 13: UI Polish (5/5 plans) — completed 2026-03-29

See full details: `.planning/milestones/v1.1-ROADMAP.md`

</details>

### 📋 v1.2 Guided Workflows & Advanced Operations (Phases 14–17)

- [x] **Phase 14: OATH Import & Password Management** - OATH URI import plus full password set/change/remove lifecycle
- [x] **Phase 15: PIV Management Key** - Management key change workflow with factory-default detection and warning
- [x] **Phase 16: Provisioning Wizards** - Initial YubiKey setup and SSH-with-touch-policy wizards with device state surfacing
- [x] **Phase 17: Dashboard Navigation Affordance** - 1–9 key discovery hints so users find all screens without docs

## Phase Details

### Phase 14: OATH Import & Password Management
**Goal**: Users can import OATH accounts via URI paste and fully manage the OATH application password
**Depends on**: Phase 13 (OATH screen already exists; extends it)
**Requirements**: OATH-07, OATH-08, OATH-09, OATH-10
**Success Criteria** (what must be TRUE):
  1. User pastes an otpauth:// URI and sees issuer, account, secret, and algorithm pre-filled before confirming
  2. User can set an OATH application password when none exists; subsequent OATH operations prompt for it only when SW 0x6982 is returned
  3. User can change the OATH password by first authenticating with the current password
  4. User can remove the OATH password after authenticating, returning the applet to unprotected mode
**Plans**: TBD
**UI hint**: yes

### Phase 15: PIV Management Key
**Goal**: Users can change the PIV management key and are warned when it is at factory default
**Depends on**: Phase 13 (PIV screen already exists; extends it)
**Requirements**: PIV-03, PIV-04, PIV-05
**Success Criteria** (what must be TRUE):
  1. PIV screen shows a banner or badge when the management key is at the factory default, with a link to the change workflow
  2. User can change the management key from default by confirming "I know it's default" without entering the key value
  3. User can change the management key from a non-default value by first authenticating with the current key
  4. User can select 3DES or AES-128/192/256 key type when setting the new management key on YubiKey 5.7+
**Plans**: TBD

### Phase 16: Provisioning Wizards
**Goal**: Users can complete goal-oriented provisioning flows ("Initial Setup", "SSH with Touch Policy") that guide through all required steps with device state visible at each step and touch policy chosen upfront
**Depends on**: Phase 14, Phase 15 (wizards invoke OATH and PIV operations already built)
**Requirements**: WIZARD-01, WIZARD-02, WIZARD-03, WIZARD-05
**Success Criteria** (what must be TRUE):
  1. User can launch "Initial YubiKey Setup" from the dashboard and step through FIDO2 PIN, first OATH account, and PIV/SSH key config — each step shows current device state and can be skipped
  2. User can launch "Set Up SSH Key with Touch Policy" and complete key generation/import, touch policy selection, SSH public key export, and shell config instructions in one flow
  3. Touch policy options (no touch, touch, cached touch) with plain-language descriptions appear before any irreversible operation in both wizards
  4. Each wizard step displays the current device state (e.g. "FIDO2 PIN: not set") before the user commits to a change
**Plans**: TBD
**UI hint**: yes

### Phase 17: Dashboard Navigation Affordance
**Goal**: New users can discover all screens from the dashboard without reading documentation
**Depends on**: Phase 16 (wizards exist and can be surfaced in the dashboard nav)
**Requirements**: WIZARD-04
**Success Criteria** (what must be TRUE):
  1. Dashboard displays a visible 1–9 key hint mapping so users see which number key opens which screen
  2. The hint does not clutter the dashboard for experienced users (dismissible or low-visual-weight presentation)
**Plans**: TBD
**UI hint**: yes

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Polish & Cross-Platform Fixes | v1.0 | 3/3 | Complete | 2026-03-24 |
| 2. UX — Menus, Wizards & Bug Fixes | v1.0 | 4/4 | Complete | 2026-03-24 |
| 3. Advanced YubiKey Features | v1.0 | 4/4 | Complete | 2026-03-24 |
| 4. Programmatic Subprocess Control | v1.0 | 4/4 | Complete | 2026-03-25 |
| 5. Native Card Protocol | v1.0 | 6/6 | Complete | 2026-03-26 |
| 6. Tech Debt + Infrastructure | v1.1 | 3/3 | Complete | 2026-03-27 |
| 7. Mouse Support + E2E Test Harness | v1.1 | 4/4 | Complete | 2026-03-27 |
| 8. textual-rs Migration | v1.1 | 6/6 | Complete | 2026-03-27 |
| 9. OATH/TOTP Screen | v1.1 | 4/4 | Complete | 2026-03-27 |
| 10. FIDO2 Screen | v1.1 | 4/4 | Complete | 2026-03-28 |
| 11. OTP Slots + Education + Onboarding | v1.1 | 3/3 | Complete | 2026-03-28 |
| 12. YubiKey Slot Delete Workflow | v1.1 | 5/5 | Complete | 2026-03-29 |
| 13. UI Polish | v1.1 | 5/5 | Complete | 2026-03-29 |
| 14. OATH Import & Password Management | v1.2 | 4/4 | Complete | 2026-03-29 |
| 15. PIV Management Key | v1.2 | 3/3 | Complete | 2026-03-29 |
| 16. Provisioning Wizards | v1.2 | 4/4 | Complete | 2026-03-29 |
| 17. Dashboard Navigation Affordance | v1.2 | 1/1 | Complete | 2026-03-29 |
