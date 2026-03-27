# yubitui Roadmap

## Milestones

- ✅ **v1.0 Production-Ready** — Phases 1–5 (shipped 2026-03-26)
- 🚧 **v1.1 Accessible to New Users** — Phases 6–10 (in progress)

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

### 🚧 v1.1 Accessible to New Users (In Progress)

**Milestone Goal:** Make yubitui approachable for new users — working mouse support, feature parity with Yubico Authenticator (TOTP/HOTP, FIDO2, OTP slots), in-TUI protocol education, new user onboarding, and a clean Model/View architecture ready for Tauri GUI.

- [ ] **Phase 6: Tech Debt + Infrastructure** - Pay v1.0 debt, Model/View split, mock mode, CI lint enforcement
- [ ] **Phase 7: Mouse Support + E2E Test Harness** - Working click navigation, ClickRegionMap, tmux E2E tests, snapshot tests
- [ ] **Phase 8: OATH/TOTP Screen** - Full OATH credential management with live TOTP codes and countdown timer
- [ ] **Phase 9: FIDO2 Screen** - FIDO2 info, PIN management, resident credential list/delete, reset
- [ ] **Phase 10: OTP Slots + Education + Onboarding** - OTP slot view, per-screen help panels, protocol glossary, new user onboarding flow

## Phase Details

### Phase 6: Tech Debt + Infrastructure
**Goal**: The codebase is a clean foundation for new screen development — v1.0 debt paid, architecture split complete, mock mode enabling hardware-free CI
**Depends on**: Phase 5 (v1.0 complete)
**Requirements**: INFRA-01, INFRA-02, INFRA-03, INFRA-04, INFRA-05, INFRA-06
**Success Criteria** (what must be TRUE):
  1. `cargo run -- --mock` launches the app with fixture YubiKey state and no hardware present
  2. `src/model/` contains all application state with zero ratatui imports; `src/tui/` contains all rendering; CI lint step enforces the boundary
  3. Each screen's key handling lives in its own typed action enum and function — app.rs match arm is no longer a monolith
  4. All model types compile with `#[derive(serde::Serialize)]` and a downstream Tauri layer could consume them without code changes
  5. Card connection is reliable — 50ms sleep after scdaemon kill is in place and no Card Busy regressions appear in CI
**Plans**: 3 plans
Plans:
- [x] 06-01-PLAN.md — Architecture rename (yubikey/ -> model/, ui/ -> tui/) + serde::Serialize + AppState + CI lint
- [x] 06-02-PLAN.md — Per-screen key handling decomposition (action enums + handle_key functions)
- [x] 06-03-PLAN.md — Mock mode (--mock flag + fixture) + 50ms sleep audit

### Phase 7: Mouse Support + E2E Test Harness
**Goal**: Users can navigate the entire app by mouse, and every feature going forward is verified by automated tests before the user sees it
**Depends on**: Phase 6
**Requirements**: MOUSE-01, MOUSE-02, MOUSE-03, MOUSE-04, TEST-01, TEST-02, TEST-03, TEST-04
**Success Criteria** (what must be TRUE):
  1. User can click any navigation item, menu entry, or button on any existing screen and it activates correctly
  2. User can scroll any list with the mouse wheel
  3. After resizing the terminal, mouse click targets remain accurate — no phantom clicks or missed targets
  4. On Windows (ConPTY), the app continues to work keyboard-only with no crash or error message when mouse is unavailable
  5. `tests/e2e/` tmux harness runs against `--mock` in CI; all existing screens have at least one passing smoke test
**Plans**: 4 plans
Plans:
- [ ] 07-01-PLAN.md — ClickRegion types + action enum Clone + AppState field + ConPTY graceful degradation
- [ ] 07-02-PLAN.md — Wire all 7 screens for mouse click regions + region-based dispatch + scroll
- [x] 07-03-PLAN.md — tmux E2E test harness (6 screen smoke tests + run_all.sh driver)
- [ ] 07-04-PLAN.md — insta snapshot tests for all screens + decouple dashboard/ssh from &App
**UI hint**: yes

### Phase 8: OATH/TOTP Screen
**Goal**: Users can manage all their OATH credentials directly in the TUI — view live codes, add accounts, delete stale ones, and be prompted for OATH password when needed
**Depends on**: Phase 6
**Requirements**: OATH-01, OATH-02, OATH-03, OATH-04, OATH-05, OATH-06
**Success Criteria** (what must be TRUE):
  1. User can open the OATH screen and see all stored credentials with their current TOTP or HOTP codes
  2. User can see a countdown timer showing how many seconds remain in the current 30-second TOTP window
  3. User can add a new OATH account by entering issuer, account name, and Base32 secret — the credential appears in the list immediately
  4. User can delete an OATH account after confirming an irreversibility warning — the credential is gone from the list
  5. When the YubiKey OATH applet is password-protected, the user is prompted for the OATH password before any credential operation proceeds
**Plans**: TBD
**UI hint**: yes

### Phase 9: FIDO2 Screen
**Goal**: Users can inspect, configure, and recover their FIDO2 security key directly in the TUI — no need for external tools
**Depends on**: Phase 6
**Requirements**: FIDO-01, FIDO-02, FIDO-03, FIDO-04, FIDO-05, FIDO-06, FIDO-07
**Success Criteria** (what must be TRUE):
  1. User can open the FIDO2 screen and see firmware version, supported algorithms, PIN status (set or not set), and PIN retry count
  2. User can set a FIDO2 PIN when none is configured, and change an existing FIDO2 PIN
  3. User can view a list of all resident FIDO2 credentials (passkeys) stored on the YubiKey
  4. User can delete a specific resident credential after confirming a warning dialog
  5. User can trigger a FIDO2 applet reset with a prominent warning about credential loss; the 10-second timing window requirement is explained clearly
  6. On Windows, when FIDO2 operations require administrator privileges, the user sees a clear message explaining why and what to do
**Plans**: TBD
**Research flag**: yes — CTAP2 credential enumeration and management over HID has MEDIUM confidence; spike on ctap-hid-fido2 credential management API before locking full plan scope
**UI hint**: yes

### Phase 10: OTP Slots + Education + Onboarding
**Goal**: Users can see their OTP slot configuration, get in-TUI explanations of every protocol on every screen, and new users are guided through initial device setup
**Depends on**: Phase 8, Phase 9
**Requirements**: OTP-01, EDU-01, EDU-02, EDU-03, EDU-04
**Success Criteria** (what must be TRUE):
  1. User can open the OTP slots screen and see whether slot 1 and slot 2 are occupied and what type each contains (Yubico OTP, static password, HMAC-SHA1, or empty)
  2. User can press `?` on any screen to open a help panel explaining what the current screen does and what the underlying protocol is — each screen has its own content
  3. User can access a protocol glossary from the main menu or `?` from the dashboard that explains PIV, FIDO, FIDO2, OpenPGP/PGP, SSH, TOTP, HOTP, and Yubico OTP in plain language
  4. On first launch with a factory-default device, the user sees an onboarding checklist guiding them through FIDO2 PIN setup, OATH account creation, and PIV/SSH configuration
  5. Onboarding correctly detects factory-default state: no FIDO2 PIN set, zero OATH credentials, PIV management key at default value
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
| 6. Tech Debt + Infrastructure | v1.1 | 1/3 | In Progress|  |
| 7. Mouse Support + E2E Test Harness | v1.1 | 1/4 | In Progress|  |
| 8. OATH/TOTP Screen | v1.1 | 0/TBD | Not started | - |
| 9. FIDO2 Screen | v1.1 | 0/TBD | Not started | - |
| 10. OTP Slots + Education + Onboarding | v1.1 | 0/TBD | Not started | - |

## Backlog

- PIV certificate view (X.509 decode via x509-parser — deferred to v2)
- PIV Management Key change (deferred to v2)
- OATH application password set/change (deferred to v2)
- OATH import via otpauth:// URI (deferred to v2)
- OTP slot write (high-risk HID frame protocol — deferred to v2)
- FIDO2 fingerprint management (Bio series only — deferred to v2)
- Application enable/disable toggle (enterprise niche — deferred to v2)
- Backup/restore workflows (deferred to v2)
- Reactive ratatui rendering engine (app.rs componentization — future milestone)
