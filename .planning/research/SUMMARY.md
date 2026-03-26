# Project Research Summary

**Project:** yubitui v1.1
**Domain:** Native PC/SC YubiKey TUI management — multi-protocol extension
**Researched:** 2026-03-26
**Confidence:** HIGH (protocol specs and stack); MEDIUM (FIDO2 HID specifics, OTP write)

## Executive Summary

yubitui v1.1 extends a mature, already-working v1.0 TUI (PIV, OpenPGP, SSH, diagnostics, multi-key) into a comprehensive YubiKey management tool that rivals Yubico Authenticator and ykman — but with native PC/SC APDUs only, no ykman subprocess dependency. The key architectural insight from research is that the YubiKey exposes three separate USB interfaces: CCID/PC/SC (PIV, OpenPGP, OATH — already working in v1.0), HID FIDO (`0xF1D0` — required for FIDO2), and HID Keyboard (required for OTP slot programming). The existing pcsc crate covers OATH completely; FIDO2 requires the `ctap-hid-fido2` crate as a net-new transport; OTP slot write is deferred as high-risk HID frame work. Two minimal new crates cover all v1.1 feature work: `totp-rs 5.7` for local TOTP display and `ctap-hid-fido2 3.5.9` for FIDO2 info.

The current `app.rs` is 1,617 lines and mixes ratatui types with business logic. This is the primary risk for v1.1 development. Research across all four files converges on the same prescription: do the Model/View architectural split first, before any new screens are added. The target structure puts all state in `src/model/` (zero ratatui imports, Tauri-serializable), renders in `src/tui/` (ratatui OK), and hardware I/O in `src/yubikey/` (unchanged boundary). Every new screen then follows a consistent 8-step addition pattern rather than expanding a monolithic match arm.

Critical risks are the three-way intersection of: OATH password-protected keys returning `0x6982` silently; TOTP codes requiring the host to supply the current Unix timestep as an 8-byte big-endian challenge (the YubiKey has no clock); and mouse click coordinates requiring render-time rect storage for hit testing (ratatui has no retained widget tree). All three risks are well-documented with precise mitigations. A mandatory Phase 0 pays down the 50ms scdaemon kill-wait debt before any new protocol work begins, preventing intermittent card-busy race failures across applets.

## Key Findings

### Recommended Stack

The v1.0 stack (Rust, ratatui 0.29, crossterm 0.28, pcsc 2.8, tokio 1.40) is validated and unchanged. Two production dependencies are added: `totp-rs 5.7` (March 2026, RFC 6238 compliant, `zeroize` feature for secret clearing) handles local TOTP calculation for code display, and `ctap-hid-fido2 3.5.9` (March 2026, actively maintained) provides CTAP2 GetInfo and credential management over the FIDO HID interface. One dev dependency adds `insta 1.42` for snapshot testing against ratatui's TestBackend. The `hidapi` crate is pulled in transitively by `ctap-hid-fido2` and does not need to be explicitly listed. Stay on ratatui 0.29 — the 0.30 upgrade bumps MSRV to 1.86.0 and is a separate concern with no v1.1 feature requirement.

**Core technologies:**
- `totp-rs 5.7` (`features = ["otpauth", "zeroize"]`): Local TOTP code calculation — most maintained RFC 6238 crate, March 2026 release, secret memory zeroization
- `ctap-hid-fido2 3.5.9`: FIDO2 info display and credential management over CTAP2/HID — only maintained Rust crate covering this transport, cross-platform, March 2026 release
- `insta 1.42` (dev): Snapshot testing for ratatui TestBackend — official ratatui-recommended testing pattern
- Mouse support: No new crate — fix `EnableMouseCapture`/`DisableMouseCapture` ordering in crossterm 0.28, use existing `Rect::contains()` from ratatui 0.26+
- Architecture: No new crates — Model/View split is a code reorganization, model types gain `#[derive(Serialize)]` using the existing serde dependency

### Expected Features

Full feature details are documented in `.planning/research/FEATURES.md`. The competitor gap analysis shows Yubico Authenticator and ykman both support OATH, FIDO2, and OTP features that v1.0 lacks entirely — these are the table stakes for v1.1.

**Must have (table stakes):**
- OATH TOTP/HOTP list and code generation — core daily-use feature; pure PC/SC CCID using YKOATH APDUs on existing pcsc infrastructure
- OATH add account (manual entry) — without add, the list is permanently read-only; QR scan is impossible in a TUI
- OATH delete account — users accumulate stale accounts; irreversibility warning required
- FIDO2 PIN set and change — unprotected FIDO2 keys are insecure; required for credential enumerate
- FIDO2 resident credential list and delete — passkey audit capability; requires PIN-authenticated CTAP2 session
- FIDO2 reset — locked-out user's only recovery path; must execute within 10s of insert
- OTP slot view (read-only) — users need slot occupancy before any configuration

**Should have (competitive advantage):**
- In-TUI protocol education (`?` keybinding per screen) — no competitor has this; zero hardware dependency; static content modals using existing popup widget
- New user onboarding flow — detect factory-default state (no FIDO2 PIN, empty OATH, default PIV mgmt key), present setup checklist
- TOTP countdown timer — show time remaining in 30s window; derives from `SystemTime::now() % period`
- Mouse support fix — v1.0 partial implementation is broken; stated v1.1 requirement

**Defer (v1.2+):**
- PIV certificate view (X.509 decode) — useful, requires `x509-parser` crate, not blocking
- PIV Management Key change — low complexity, can wait
- OATH application password protection — complex HMAC challenge/response state, niche use case
- OTP slot write configuration — high-risk HID frame protocol, underdocumented, power-user only
- Application enable/disable toggle — enterprise niche, complex Management Key auth
- FIDO2 fingerprint management (Bio series only)

**Explicitly avoided (anti-features):**
- QR code scanning (no camera in TUI), cloud backup of TOTP secrets (defeats security model), ykman subprocess fallback (project core constraint), OpenPGP reset from TUI (catastrophic irreversible without gpg confirmation flow)

### Architecture Approach

The architecture target is a strict three-layer system: `src/model/` (zero ratatui imports, all application state, Tauri-serializable), `src/tui/` (all ratatui code, owns Terminal), `src/yubikey/` (existing PC/SC hardware I/O, unchanged boundary). The Model/View split converts the current 1,617-line `app.rs` monolith into per-screen state structs in `src/model/` with per-screen `handle_key()` functions returning typed action enums, and render functions in `src/tui/screens/` that accept `&AppModel` rather than `&App`. Mouse hit testing uses a `ClickRegionMap` in the TUI layer that is rebuilt each render frame and stores `Rect` values registered by widgets during render — never recomputed in event handlers. Full details in `.planning/research/ARCHITECTURE.md`.

**Major components:**
1. `src/model/mod.rs` — `AppModel` struct with per-screen sub-models (PinModel, OathModel, Fido2Model, etc.); zero ratatui imports; all fields `Clone + Debug + Serialize`
2. `src/tui/app.rs` — event loop, Terminal ownership, dispatches to per-screen `handle_key()` in model, applies returned actions, holds `ClickRegionMap`
3. `src/tui/screens/` — one file per screen, accepts `&AppModel`, registers click regions, returns `Option<Action>`
4. `src/yubikey/oath.rs` (new) — YKOATH CCID APDU layer: SELECT AID `A0:00:00:05:27:21:01`, LIST, CALCULATE, PUT, DELETE
5. `src/yubikey/fido2.rs` (new) — FIDO2 via `ctap-hid-fido2`: GetInfo, credential management, PIN operations
6. `src/yubikey/otp.rs` (new) — OTP slot status reads via HID; write operations deferred
7. `src/tui/mouse.rs` (new) — `ClickRegionMap`, `ClickTarget` enum, `hit_test()` implementation

### Critical Pitfalls

Full pitfall catalog with recovery strategies in `.planning/research/PITFALLS.md`. The top risks are:

1. **scdaemon card-busy race (Pitfall 9)** — OATH and FIDO2 add new applet connections that collide with scdaemon restart. Pay the 50ms kill-wait debt from v1.0 before any v1.1 APDU work. Implement 3-retry exponential backoff (50ms/100ms/200ms) for `SCARD_E_SHARING_VIOLATION`. Reuse one `pcsc::Card` handle across applet switches within a single user action.

2. **OATH host-supplied timestamp (Pitfall 4)** — The YubiKey OATH applet has no clock. The CALCULATE APDU requires the caller to send `floor(unix_seconds / 30)` as an 8-byte big-endian challenge in TLV `0x74`. Use `SystemTime::now()` at the moment of the call — never cached at startup. Wrong or missing timestamp produces codes that fail every validation.

3. **OATH password-protected keys (Pitfall 5)** — If the user set an OATH access key in Yubico Authenticator, all OATH commands return SW `0x6982` until a VALIDATE handshake completes. Always inspect the SELECT response for TLV tag `0x74`; if present, prompt for OATH password and run VALIDATE before any LIST/CALCULATE.

4. **OATH CALCULATE_ALL incomplete data (Pitfall 6)** — HOTP credentials (tag `0x77`) and touch-required credentials (tag `0x7c`) are returned without computed codes. Also, responses over 256 bytes require SEND REMAINING (`0xa5`) pagination. Parse response tags correctly; implement `send_remaining_loop()` parallel to the existing `get_data()` chaining.

5. **Model/View borrow checker conflict (Pitfall 10)** — `terminal.draw(|f| self.render(f))` captures `&mut self`. If render also needs to write `ClickRegionMap`, both reads and mutations compete. Resolution: `render()` takes `&self` and returns `RenderedAreas`; the outer loop stores it after the draw closure exits. No `RefCell` workaround needed.

## Implications for Roadmap

Based on combined research, the dependency graph and pitfall-to-phase mapping converge on a clear 6-phase structure. The Model/View split is not optional — it is the prerequisite for all subsequent phases. The architecture file's build order and the pitfalls file's phase mapping agree on this ordering independently.

### Phase 0: Tech Debt and Infrastructure
**Rationale:** The 50ms scdaemon kill-wait listed as unpaid debt in PROJECT.md directly causes intermittent failures for every new APDU operation added in v1.1. Fix it before writing any new protocol code. Also add the `--mock` CLI flag here so E2E tests have a hardware-free path from day one.
**Delivers:** Reliable card connection, mock mode for testing, zero known regressions from v1.0
**Addresses:** scdaemon race (Pitfall 9)
**Research flag:** Skip — well-understood debt with prescribed fix

### Phase 1: Model/View Architectural Split
**Rationale:** Every pitfall related to mouse handling, borrow checking, and ratatui type leakage (Pitfalls 1, 3, 10) requires the Model/View separation as its structural fix. All four research files independently recommend this as the first feature-enabling step. Adding OATH or FIDO2 screens into the current monolith creates compounding debt.
**Delivers:** `src/model/` with `AppModel`, per-screen sub-models, per-screen `handle_key()` functions; all existing screens migrated; `cargo test` passes with identical behavior; no ratatui imports in `src/yubikey/` or `src/model/`
**Avoids:** Pitfalls 3 (ratatui type leakage), 10 (borrow checker in render), 5 (monolithic match arm growth)
**Research flag:** Skip — well-documented Elm architecture pattern for ratatui; multiple official sources confirm the approach

### Phase 2: Mouse Support and E2E Test Suite Foundation
**Rationale:** Mouse support is a stated v1.1 requirement (broken in v1.0). The `ClickRegionMap` and `RenderedAreas` patterns belong in Phase 2 because they require the clean Model/View structure from Phase 1. The E2E tmux test suite is established here so all subsequent feature phases can write tests as they go. Research confirms the tmux `send-keys` + `capture-pane` approach requires no new crates beyond `insta`.
**Delivers:** Working mouse click-to-navigate on all existing screens; `ClickRegionMap` with Z-order enforcement; `tests/e2e/` harness with 5-10 navigation smoke tests; ratatui TestBackend + insta snapshot tests
**Avoids:** Pitfalls 1 (mouse coordinates), 2 (Z-order click passthrough)
**Research flag:** Skip for mouse (crossterm 0.28 already present, fix is code not crates); skip for tmux tests (standard shell scripting)

### Phase 3: OATH/TOTP Screen
**Rationale:** OATH is the highest-value new feature (daily use), runs on the existing PC/SC CCID transport (no new OS interface), and builds directly on `card.rs` infrastructure already proven in v1.0. Adding `totp-rs 5.7` is the only new dependency. The three OATH-specific pitfalls (timestamp challenge, password protection, CALCULATE_ALL pagination) are well-documented with precise code solutions.
**Delivers:** OATH screen listing all stored credentials with current TOTP/HOTP codes; countdown timer per credential; add/delete credential flows; OATH password prompt when `0x74` tag present in SELECT response
**Uses:** `totp-rs 5.7` (local calculation display), existing `pcsc` crate (YKOATH APDUs), existing BER-TLV infrastructure
**Implements:** `src/yubikey/oath.rs` APDU layer + `src/model/oath.rs` + `src/tui/screens/oath.rs`
**Avoids:** Pitfalls 4 (timestamp), 5 (OATH password), 6 (CALCULATE_ALL pagination)
**Research flag:** Skip — Yubico publishes the full YKOATH specification; all APDU bytes are confirmed

### Phase 4: FIDO2 Screen
**Rationale:** FIDO2 is table stakes but requires the only significant new transport dependency (`ctap-hid-fido2`). Doing it after OATH means the pattern of adding a new screen is already proven. FIDO2's HID transport is separate from PC/SC — implement it cleanly in `src/yubikey/fido2.rs` without touching `card.rs`. The CTAP2 double-wrapped APDU framing (Pitfall 7) requires careful attention to spec section 8.2.
**Delivers:** FIDO2 info screen showing firmware version, supported algorithms, PIN status, retry count; FIDO2 PIN set/change; resident credential list and delete; FIDO2 reset with 10s timing window warning
**Uses:** `ctap-hid-fido2 3.5.9`, `ciborium` or CBOR parsing (pulled transitively)
**Implements:** `src/yubikey/fido2.rs` + `src/model/fido2.rs` + `src/tui/screens/fido2.rs`
**Avoids:** Pitfall 7 (CTAP2 double-wrapped APDU), Anti-Pattern 4 (do not route FIDO2 through PC/SC)
**Research flag:** Needs research-phase during planning — CTAP2 CBOR response parsing and credential management over HID has MEDIUM confidence; exact HID APDU structure benefits from implementation validation before committing to full scope

### Phase 5: OTP Slot View and In-TUI Education
**Rationale:** OTP slot status read is achievable via existing `hidapi` (transitively available from Phase 4's ctap-hid-fido2). Write operations are explicitly deferred — the HID frame protocol is underdocumented and high-risk. In-TUI education is bundled here as it is zero-dependency (static text modals using the existing popup widget) and benefits from all screens being stable before writing their explanatory content. New user onboarding also lands here since it requires OATH and FIDO2 detection from Phases 3 and 4.
**Delivers:** OTP slots screen showing slot 1 / slot 2 occupancy and type; `?` help panel on every screen with protocol education content; new user onboarding checklist (detected from device state)
**Uses:** `hidapi` (transitive), existing popup widget, static text content
**Avoids:** Pitfall 8 (OTP access code blind writes — slot write is deferred entirely)
**Research flag:** Skip for education (static content); skip for OTP read (status structure documented); light research recommended for onboarding detection heuristics (PIV default management key detection)

### Phase 6: Polish and Cross-Platform Validation
**Rationale:** After all features land, address the remaining cross-cutting concerns: Windows ConPTY mouse fallback (confirmed limitation by Microsoft — degrade gracefully, do not crash), CI integration for tmux tests (Linux only; macOS optional), security review (ensure no sensitive data logged — OATH credential names, TOTP codes, FIDO2 PIN).
**Delivers:** Windows keyboard fallback when mouse not supported; CI pipeline with E2E tmux job; security audit of log output; "Looks Done But Isn't" checklist from PITFALLS.md fully satisfied
**Research flag:** Skip — Windows ConPTY limitation is a confirmed upstream constraint, not an implementation problem

### Phase Ordering Rationale

- Phase 0 before everything: the 50ms scdaemon debt causes intermittent failures that would corrupt test results for all subsequent phases
- Phase 1 before new screens: architecture research, pitfalls research, and feature research all independently conclude that adding screens into the current monolith is the primary technical debt risk
- Phase 2 immediately after architecture: mouse support requires ClickRegionMap which requires clean Model/View; E2E tests provide regression safety for all subsequent phases
- Phase 3 (OATH) before Phase 4 (FIDO2): OATH uses existing PC/SC transport (zero new infrastructure risk); proves the new-screen addition pattern before introducing the HID transport dependency
- Phase 5 bundles OTP read + education + onboarding: all three are low-risk deliverables that benefit from Phases 3 and 4 being stable; education content can only be written once the screens it describes are complete

### Research Flags

Needs deeper research during planning:
- **Phase 4 (FIDO2):** CTAP2 CBOR response parsing and credential enumeration over HID has MEDIUM confidence. The `ctap-hid-fido2` crate API for credential management (not just GetInfo) benefits from prototyping before full feature spec is locked. Recommend a `gsd:research-phase` spike on the credential management CTAP2 commands.
- **Phase 5 (OTP read via hidapi):** OTP status APDU structure is documented but exact PC/SC vs. HID routing for the status read needs validation. MEDIUM confidence — verify against a real device before writing the APDU implementation.

Phases with standard patterns (skip research-phase):
- **Phase 0:** Prescribed fix from existing PROJECT.md debt list
- **Phase 1:** Elm architecture pattern fully documented in official ratatui docs with code examples
- **Phase 2:** crossterm mouse capture fix is a code correction, not an architecture decision; tmux E2E pattern is standard shell scripting
- **Phase 3 (OATH):** Full YKOATH specification published by Yubico; all INS bytes, TLV tags, and response formats confirmed at HIGH confidence
- **Phase 6:** Windows ConPTY limitation is a documented upstream constraint, mitigation strategy is graceful degradation

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All crate versions verified; transport boundaries confirmed from Yubico official docs; ratatui 0.29 compatibility confirmed |
| Features | HIGH | Feature map derived from official Yubico Authenticator and ykman documentation; APDU commands confirmed against Yubico SDK manual |
| Architecture | HIGH (patterns) / MEDIUM (FIDO2 HID specifics) | Elm architecture pattern from official ratatui docs is HIGH; FIDO2 HID APDU specifics require implementation validation |
| Pitfalls | HIGH | Protocol pitfalls from official specs; mouse pitfalls from crossterm/ratatui issue trackers; Windows ConPTY from Microsoft confirmation |

**Overall confidence:** HIGH for phases 0-3 and 5-6; MEDIUM for Phase 4 (FIDO2 credential management)

### Gaps to Address

- **FIDO2 credential management APDU details:** `ctap-hid-fido2 3.5.9` is confirmed for GetInfo; the credential management commands (enumerate, delete) need prototyping against a real device before Phase 4 scope is locked. Spike recommended during Phase 4 planning.
- **OTP status APDU transport (PC/SC vs. HID):** Architecture.md notes that OTP "straddles both transports" — exact read path for slot occupancy needs verification against `card.rs` CCID patterns. If PC/SC works, `hidapi` is not needed for OTP read.
- **OATH credential 64-byte name limit enforcement:** Pitfalls research flags this as a "looks done but isn't" item. UI input field must enforce the limit before PUT APDU; ensure OathModel includes the validation.
- **Windows ConPTY mouse absence:** Mouse events are confirmed not to transit in Windows Terminal's ConPTY mode (Microsoft upstream issue). Plan keyboard-only fallback path from the start of Phase 2 rather than adding it in Phase 6.

## Sources

### Primary (HIGH confidence)
- https://developers.yubico.com/OATH/YKOATH_Protocol.html — Full YKOATH APDU specification (AID, INS bytes, TLV tags, response format)
- https://docs.yubico.com/hardware/yubikey/yk-tech-manual/yk5-apps.html — YubiKey application transport mapping (CCID vs. HID FIDO vs. HID keyboard)
- https://docs.yubico.com/yesdk/users-manual/application-oath/oath-commands.html — OATH command APDUs with parameters
- https://docs.yubico.com/yesdk/users-manual/application-fido2/fido2-commands.html — FIDO2 commands, HID transport requirement
- https://docs.yubico.com/yesdk/users-manual/application-fido2/apdu/get-info.html — FIDO2 getInfo APDU structure
- https://docs.yubico.com/yesdk/users-manual/application-otp/slots.html — OTP slot structure, access codes
- https://docs.rs/crate/totp-rs/latest — totp-rs 5.7.1 features (March 2026)
- https://github.com/gebogebogebo/ctap-hid-fido2 — ctap-hid-fido2 3.5.9 capabilities (March 2026)
- https://ratatui.rs/concepts/application-patterns/the-elm-architecture/ — Model/Update/View pattern with Rust code examples
- https://ratatui.rs/recipes/testing/snapshots/ — insta snapshot testing with ratatui TestBackend
- https://ratatui.rs/highlights/v030/ — ratatui 0.30 MSRV bump to 1.86

### Secondary (MEDIUM confidence)
- https://github.com/ratatui/ratatui/discussions/1051 — Mouse hit testing on Rect; no native ratatui hit test; community RenderedAreas pattern
- https://github.com/ratatui/ratatui/discussions/220 — ratatui best practices discussion
- https://github.com/AntonGepting/tmux-interface-rs — tmux_interface v0.4.0 marked experimental/unstable by authors
- https://github.com/StarGate01/CTAP-bridge — FIDO2 PC/SC CTAPHID bridge reference implementation
- https://docs.yubico.com/yesdk/users-manual/application-otp/hid.html — OTP HID keyboard transport

### Tertiary (confirmed upstream constraints)
- https://github.com/crossterm-rs/crossterm/issues/446 — Windows Terminal mouse events (ConPTY limitation confirmed by Microsoft)
- https://github.com/microsoft/terminal/issues/376 — ConPTY does not transit mouse reports (Microsoft developer confirmed)
- /.planning/PROJECT.md — yubitui own documented tech debt (50ms scdaemon kill-wait)

---
*Research completed: 2026-03-26*
*Ready for roadmap: yes*
