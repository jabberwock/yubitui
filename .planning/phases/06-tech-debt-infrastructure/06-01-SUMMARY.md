---
phase: 06-tech-debt-infrastructure
plan: 01
subsystem: infra
tags: [rust, serde, pcsc, architecture, refactor, model-layer, tui-layer, tauri-prep]

# Dependency graph
requires:
  - phase: 05-native-card-protocol
    provides: card.rs, detection.rs, APDU primitives, all src/yubikey/ modules

provides:
  - src/model/ directory (renamed from src/yubikey/) with all model types
  - src/tui/ directory (renamed from src/ui/) with all TUI rendering modules
  - serde::Serialize on all model types (YubiKeyInfo, Version, Model, FormFactor, YubiKeyState, PinStatus, OpenPgpState, KeyInfo, PivState, SlotInfo, TouchPolicies, TouchPolicy, SshConfig)
  - AppState struct in src/model/app_state.rs with Tauri-serializable state
  - Screen enum moved from app.rs to model/app_state.rs (includes Piv variant)
  - CI lint step rejecting ratatui imports in src/model/
  - YubiKey NEO misidentification bug fixed: firmware=None → Model::Unknown
  - parse_device_info_tlv() helper for 0x71-wrapped and bare GET_DEVICE_INFO responses
  - model_info_from_dev_info_firmware() helper for testable model resolution

affects:
  - 06-02 (will consume model types, AppState, Screen)
  - 06-03 (will consume model layer for Tauri bindings)
  - Any future Tauri GUI work (model layer is now serializable)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "model/tui boundary: src/model/ has zero ratatui imports, enforced by CI grep lint"
    - "AppState struct encapsulates all Tauri-serializable navigation + YubiKey state"
    - "Screen enum lives in model layer (not app.rs) for GUI-agnostic nav"
    - "parse_device_info_tlv() extracted for hardware-free unit testing of APDU parsing"
    - "model_info_from_dev_info_firmware() prevents openpgp_version fallback bug"

key-files:
  created:
    - src/model/app_state.rs
  modified:
    - src/model/mod.rs
    - src/model/card.rs
    - src/model/detection.rs
    - src/model/pin.rs
    - src/model/openpgp.rs
    - src/model/piv.rs
    - src/model/touch_policy.rs
    - src/model/ssh.rs
    - src/app.rs
    - src/tui/mod.rs
    - .github/workflows/rust.yml

key-decisions:
  - "git mv used for rename (not cp) to preserve git history; sed bulk-replaced all import paths"
  - "pub use crate::model::Screen re-exported from app.rs for backward compat with callers"
  - "App struct uses state: AppState field for serializable state; pin_state/key_state/ssh_state/dashboard_state remain direct fields (TUI-specific, non-serializable)"
  - "Screen::Piv added to model/app_state.rs variant but handled as no-op in render() -- future plan wires it"
  - "Version now derives PartialEq (needed for test assertions on Option<Version>)"
  - "parse_device_info_tlv() checks raw[0]==0x71 first (outer container), else skips leading length byte"
  - "firmware=None fallback removed: use Version{0,0,0} for display, Model::Unknown for classification"
  - "todo moved from pending/ to done/ after both bug layers verified"

patterns-established:
  - "Model layer (src/model/) must have zero ratatui imports -- enforced by CI"
  - "TLV response helpers take &[u8] slices for hardware-free testing"
  - "When management AID firmware=None, return Model::Unknown (never fall back to OpenPGP spec version)"

requirements-completed: [INFRA-03, INFRA-04, INFRA-06]

# Metrics
duration: 9min
completed: 2026-03-26
---

# Phase 06 Plan 01: Tech Debt Infrastructure Summary

**src/yubikey/ renamed to src/model/, src/ui/ to src/tui/, all model types derive serde::Serialize, AppState struct extracted, CI boundary lint added, and YubiKey NEO misidentification bug fixed with TDD tests**

## Performance

- **Duration:** ~9 min
- **Started:** 2026-03-26T19:34:42Z
- **Completed:** 2026-03-26T19:44:12Z
- **Tasks:** 3
- **Files modified:** 26 (renamed) + 11 (modified) + 1 (created)

## Accomplishments

- Atomic rename of `src/yubikey/` → `src/model/` and `src/ui/` → `src/tui/` using `git mv` + bulk sed; all 85 existing tests pass immediately after rename
- All public model types now derive `serde::Serialize` making them Tauri-ready; `AppState` struct with `Screen` enum extracted to `src/model/app_state.rs`
- CI lint step added to `rust.yml` that fails builds if any file in `src/model/` imports ratatui
- YubiKey NEO misidentification bug fixed: two-layer fix — (a) `get_device_info()` now correctly unwraps 0x71 outer TLV container via `parse_device_info_tlv()`, (b) `firmware=None` no longer falls back to `openpgp_version` (OpenPGP spec version, not hardware firmware); returns `Model::Unknown` instead
- 9 new unit tests bring total to 94; all pass; `cargo clippy -- -D warnings` clean

## Task Commits

Each task was committed atomically:

1. **Task 1: Rename directories and fix all imports** - `1feb71f` (feat)
2. **Task 2: Add serde::Serialize + AppState + CI lint** - `47ae318` (feat)
3. **Task 3: Fix YubiKey NEO misidentification bug** - `6f4f15d` (fix)

## Files Created/Modified

- `src/model/app_state.rs` - New: AppState struct + Screen enum (Tauri-serializable)
- `src/model/mod.rs` - Added pub mod app_state, serde::Serialize to all types, PartialEq to Version
- `src/model/card.rs` - Added parse_device_info_tlv() with 0x71 unwrap logic + 5 new tests
- `src/model/detection.rs` - Added model_info_from_dev_info_firmware(), fixed fallback bug + 4 new tests
- `src/app.rs` - Screen moved to model; App uses state: AppState field
- `src/tui/mod.rs` - Added Screen::Piv arm to render_status_bar match
- `.github/workflows/rust.yml` - Added model layer ratatui lint step
- All 26 renamed files: `src/{yubikey/* → model/*, ui/* → tui/*}`

## Decisions Made

- Used `git mv` for rename (preserves git history); single sed pass fixed all 26 files in one command
- Residual `ui::` references not caught by sed pattern were fixed manually (3 `Some(ui::pin::...)` instances in app.rs)
- `pub use crate::model::Screen;` re-exported from app.rs so callers using `crate::app::Screen` need no changes
- `App` struct: `state: AppState` holds serializable fields; `pin_state`, `key_state`, `ssh_state`, `dashboard_state` remain direct fields (they contain TUI-specific state not suitable for Tauri serialization)
- `Screen::Piv` added to model enum to match plan spec; render arm is no-op (TUI screen not yet built)
- `Version` gained `PartialEq` + `Eq` (required for `assert_eq!` on `Option<Version>` in new tests)
- CI lint uses simple `grep -r 'ratatui' src/model/` — comments in model files must not use the word "ratatui"

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] sed pattern missed `Some(ui::pin::...)` in app.rs**
- **Found during:** Task 1 (rename + import fix)
- **Issue:** sed pattern `' ui::'` only matched patterns with leading space; `Some(ui::pin::UnblockPath::ResetCode)` starts with `Some(` so was missed
- **Fix:** Additional `sed -i '' 's/Some(ui::pin::/Some(tui::pin::/g'` pass
- **Files modified:** src/app.rs
- **Committed in:** `1feb71f` (Task 1 commit)

**2. [Rule 1 - Bug] `use yubikey::detection::` bare reference in main.rs**
- **Found during:** Task 1
- **Issue:** `list_yubikeys()` used `use yubikey::detection::detect_all_yubikey_states` without `crate::` prefix; sed pattern only replaced `crate::yubikey::`
- **Fix:** Manual edit to `use model::detection::detect_all_yubikey_states`
- **Files modified:** src/main.rs
- **Committed in:** `1feb71f`

**3. [Rule 1 - Bug] `Screen::Piv` non-exhaustive match after enum gained Piv variant**
- **Found during:** Task 2 (compile error)
- **Issue:** Old app.rs Screen had 6 variants; new model/app_state.rs Screen has 7 (added Piv). Two match sites failed: app.rs render() and tui/mod.rs render_status_bar()
- **Fix:** Added `Screen::Piv => {}` (no-op) in app.rs and `Screen::Piv => "PIV"` in tui/mod.rs
- **Files modified:** src/app.rs, src/tui/mod.rs
- **Committed in:** `47ae318`

**4. [Rule 1 - Bug] Test byte count mismatch in test_parse_device_info_0x71_wrapped**
- **Found during:** Task 3 TDD GREEN phase
- **Issue:** TLV length byte said 0x09 (9) but only 8 inner bytes were in test vec; tlv_find correctly rejected the malformed test data
- **Fix:** Changed length byte from 0x09 to 0x08 to match actual inner byte count
- **Files modified:** src/model/card.rs
- **Committed in:** `6f4f15d`

---

**Total deviations:** 4 auto-fixed (all Rule 1 - Bug)
**Impact on plan:** All fixes were mechanical correctness issues caught during implementation. No scope changes.

## Issues Encountered

- `crate::ui` import in app.rs was structured as `use crate::{diagnostics::Diagnostics, ui, yubikey::YubiKeyState}` (compound import); sed only handled standalone `use crate::ui;` — required manual fix
- `Version` struct initially lacked `PartialEq` causing `assert_eq!` compile errors in new tests; added `PartialEq + Eq` derive

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `src/model/` and `src/tui/` directories cleanly separated; CI enforces the boundary
- All model types are Tauri-serializable via `serde::Serialize`
- `AppState` + `Screen` ready for 06-02 to consume
- YubiKey NEO detection bug resolved — hardware with management AID issues now shows "Unknown YubiKey" instead of "YubiKey NEO"
- 94 unit tests all pass; no hardware required for new tests

---
*Phase: 06-tech-debt-infrastructure*
*Completed: 2026-03-26*
