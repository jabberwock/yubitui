# Phase 7: Mouse Support + E2E Test Harness - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-26
**Areas discussed:** ClickRegionMap design, Click target scope, E2E test format, Snapshot test coverage

---

## ClickRegionMap design

**Q: Where should ClickRegionMap live?**
Options: AppState field / Per-screen state struct / Return from render
→ **Selected: AppState field**

**Q: How should actions be typed?**
Options: Enum per screen / String tag + dispatch / Single flat enum
→ **Selected: Enum per screen** (ClickAction wrapping DashboardAction, KeyAction, etc.)

**Q: Who populates the ClickRegionMap?**
Options: render() writes to AppState / Separate register() call / Hardcoded in handle_mouse
→ **Selected: render() writes to AppState** via `&mut Vec<ClickRegion>` parameter

---

## Click target scope

**Q: Which screens need full click support?**
Options: Dashboard / Keys + PIV + SSH / Pin + Diagnostics + Help
→ **Selected: All three (all screens)**
User note: "Every element that was highlightable/interactable via arrow keys and Enter should also have mouse support. And please remember to keep the view logic separate because the TUI library will change, and we need to add GUI (Tauri for example)."

**Q: How to reconcile AppState storage with ratatui::Rect?**
Options: Own Region type in model / Live on App not AppState / AppState with #[serde(skip)]
→ **Selected: Own Region type in model** — `pub struct Region { x, y, w, h }` in src/model/, render layer maps ratatui::Rect → Region

---

## E2E test format

**Q: How should tmux E2E tests be implemented?**
Options: Shell scripts / Rust integration test binary / cargo test --test
→ **Selected: Shell scripts** (tests/e2e/*.sh)

**Q: How should E2E test scripts be organized?**
Options: One per screen / One per feature / Single test runner
→ **Selected: One script per screen**

**Q: What should a smoke test verify?**
Options: Navigate + visible text / Navigate + interact + return / Full happy path
→ **Selected: Navigate + interact + return** — navigate to screen, perform one key interaction, assert result, return to dashboard

---

## Snapshot test coverage

**Q: What should insta snapshot tests cover?**
Options: Key states per screen (~3) / Full screen render only / Every meaningful widget
→ **Selected: Key states per screen** — ~3 snapshots: default populated, empty/no-data, one interactive state

**Q: How should snapshot tests use the mock fixture?**
Options: Shared mock fixture / Minimal per-test structs
→ **Selected: Shared mock fixture** from Phase 6
