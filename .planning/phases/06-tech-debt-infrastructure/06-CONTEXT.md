# Phase 6: Tech Debt + Infrastructure - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Pay v1.0 tech debt, split the architecture into Model/View, add mock mode, and enforce the boundary in CI — so Phases 7–10 build new screens on clean foundations.

Concrete deliverables:
- `--mock` flag: launches app with fixture YubiKeyState, no hardware required
- `src/yubikey/` renamed to `src/model/`, `src/ui/` renamed to `src/tui/`
- All model types annotated with `#[derive(serde::Serialize)]`
- Per-screen typed action enums and `handle_key()` functions; app.rs shrinks to ~100-200 line orchestrator
- CI grep step: `grep -r 'ratatui' src/model/ && exit 1`
- Fix YubiKey NEO misidentification (management AID TLV parsing + bad firmware fallback)
- Verify 50ms sleep coverage after scdaemon kill (already in connect_card() — confirm no bypass paths)

Out of scope: new screens (OATH, FIDO2, OTP), mouse support, E2E test harness.

</domain>

<decisions>
## Implementation Decisions

### Mock mode (INFRA-01)
- **D-01:** Add `--mock` flag to clap `Args` in `main.rs`; pass a boolean into `App::new()`. No hardware interaction when mock is active.
- **D-02:** Mock fixture represents a **fully configured YubiKey** — GPG key loaded and in all slots, SSH configured, PINs changed from default. Gives E2E tests (Phase 7) real content to exercise on all screens.
- **D-03:** Fixture is a **hardcoded Rust struct** — `const` or `lazy_static` in `src/model/`. No file I/O, always consistent, compiles in. Not a JSON/TOML fixture file.

### Architecture migration (INFRA-03, INFRA-04)
- **D-04:** **Direct rename** — `src/yubikey/` → `src/model/`, `src/ui/` → `src/tui/`. `yubikey/` already contains zero ratatui imports; the rename formalizes what's already true.
- **D-05:** **Big-bang**: rename + fix all imports in one plan. No intermediate partial state.
- **D-06:** `app.rs` stays at `src/app.rs` as a thin orchestrator (~100-200 lines). Not moved into `src/tui/`. Owns the event loop; calls per-screen `handle_key()` and `render()`.

### Navigation state (INFRA-03, INFRA-05)
- **D-07:** Navigation state (`current_screen`, `previous_screen`, popup flags) moves into `src/model/` — it's pure enum/bool state with no ratatui types. Lives in `src/model/app_state.rs` or similar.
- **D-08:** `App` struct in `app.rs` holds an `AppState` (from `src/model/`) + ratatui `Terminal`. `AppState` is what Tauri would serialize; `App` is the TUI runtime.

### Per-screen action enums (INFRA-05)
- **D-09:** Each screen gets its own typed action enum and `handle_key()` function. These live in `src/tui/<screen>.rs` alongside the render function. The massive match arm in `app.rs` is replaced by dispatch calls.

### serde::Serialize (INFRA-06)
- **D-10:** All types that land in `src/model/` get `#[derive(serde::Serialize)]`. This includes `YubiKeyState`, `PinStatus`, `KeyAttributes`, `Screen` enum, navigation state, and any other model-layer types. Tauri can consume the full `AppState` without code changes.

### CI lint enforcement (INFRA-04)
- **D-11:** Add a CI step: `grep -r 'ratatui' src/model/ src/yubikey/ 2>/dev/null && echo "ERROR: ratatui found in model layer" && exit 1`. Runs on the existing 3-OS matrix. Fast, zero dependencies, fails loudly.

### 50ms sleep (INFRA-02)
- **D-12:** The 50ms sleep is **already implemented** in `card.rs` inside `connect_card()` (line 64). Plan must verify no code paths issue APDUs while bypassing `connect_card()`. If gaps found, add the sleep there too.

### Folded Todos
- **Fix YubiKey NEO misidentification** (`detection.rs:82-90`, `card.rs:280-318`): Two-layer bug — (1) GET_DEVICE_INFO response has an outer 0x71 TLV container that our `tlv_find` doesn't unwrap before searching for inner tags; (2) when `firmware` is None, fallback incorrectly uses `openpgp_version` (spec version 3.4) which routes to YubiKey NEO. Fix: unwrap 0x71 outer container first; when firmware is None do NOT fall back to openpgp_version — return `Model::Unknown` instead. Card.rs/detection.rs are being touched for the model split anyway — natural fit.

### Claude's Discretion
- Exact file name for navigation state in src/model/ (`app_state.rs`, `state.rs`, etc.)
- Whether Screen enum lives in `src/model/app_state.rs` or its own `src/model/screen.rs`
- APDU constant naming style (already established in Phase 5)
- Retry logic on transient card errors

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` §Infrastructure & Architecture (INFRA-01 through INFRA-06) — acceptance criteria for all 6 infra requirements

### Key source files (read before planning)
- `src/app.rs` — the 1617-line monolith; this is what gets decomposed (INFRA-05)
- `src/main.rs` — Args struct where `--mock` flag gets added (INFRA-01)
- `src/yubikey/card.rs` — `kill_scdaemon()` + `connect_card()` with 50ms sleep (INFRA-02 verification)
- `src/yubikey/detection.rs:82-90` — YubiKey NEO misidentification bug (firmware fallback)
- `src/yubikey/card.rs:280-318` — GET_DEVICE_INFO TLV parsing (0x71 unwrap bug)

### Todo detail
- `.planning/todos/pending/2026-03-26-fix-wrong-model-when-management-aid-unavailable.md` — full problem statement, investigation steps, and solution for the folded NEO misidentification bug

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `kill_scdaemon()` + `connect_card()` in `src/yubikey/card.rs` — established pattern for card access; 50ms sleep already at line 64
- `apdu_sw()` helper in `card.rs` — reuse in all APDU sites
- `src/ui/widgets/` — existing popup/dialog widgets move to `src/tui/widgets/` unchanged

### Established Patterns
- `anyhow::bail!` + plain English error messages (Phase 5)
- SW codes to `tracing::debug!` only — never shown in UI (Phase 5)
- Parser functions take `&str`/`&[u8]` and are unit-testable without hardware (Phase 3+)

### Current Structure → Target Structure
```
src/
  app.rs          → src/app.rs (thin orchestrator, ~100-200 lines)
  main.rs         → src/main.rs (add --mock flag)
  yubikey/        → src/model/ (zero ratatui imports — already true)
  ui/             → src/tui/ (all ratatui rendering)
  diagnostics/    → stays (no change)
  utils/          → stays (no change)
```

### Integration Points
- `App::new()` in `app.rs` — receives mock boolean, skips hardware detection if true
- `detect_all_yubikey_states()` in `detection.rs` — returns fixture state when mock=true
- All screen render functions in `src/ui/` — move to `src/tui/` with path-only changes
- `AppState` (new) — holds `Vec<YubiKeyState>`, `Screen`, nav state; lives in `src/model/`

</code_context>

<specifics>
## Specific Ideas

- The rename is "formalizing what's already true" — `src/yubikey/` has zero ratatui imports already. The split is primarily a semantic and enforcement move, not a large code change.
- Mock fixture should be realistic enough for Phase 7 E2E tests to exercise all screens meaningfully. A YubiKey with: FW 5.4.3, serial 12345678, all 3 GPG slots occupied, SSH key configured, PINs at 3/3 retries.
- The NEO misidentification fix was triggered by hardware testing — the 0x71 outer TLV container behavior was previously fixed for GET_DEVICE_INFO (commit 789abb2) but the firmware fallback path was not.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

### Reviewed Todos (not folded)
None — the one matched todo was folded in.

</deferred>

---

*Phase: 06-tech-debt-infrastructure*
*Context gathered: 2026-03-26*
