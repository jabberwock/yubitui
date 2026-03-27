---
phase: 7
reviewers: [gemini]
reviewed_at: 2026-03-26T00:00:00Z
plans_reviewed: [07-01-PLAN.md, 07-02-PLAN.md, 07-03-PLAN.md, 07-04-PLAN.md]
---

# Cross-AI Plan Review — Phase 7

## Gemini Review

This review covers the four implementation plans for **Phase 7: Mouse Support + E2E Test Harness** for the `yubitui` project.

### Summary
The proposed plans represent a sophisticated and architecturally sound approach to adding mouse support and a dual-layered testing strategy (E2E and Snapshot). The plans demonstrate a high degree of "architectural empathy" by strictly adhering to the "No ratatui in model" constraint while preparing the codebase for a future GUI transition by decoupling rendering from the main application state. The use of a `ClickRegionMap` that is rebuilt every frame is an industry-standard pattern for TUIs that effectively handles terminal resizing and dynamic layouts.

### Strengths
- **Architectural Purity:** Plan 01 correctly implements a project-owned `Region` struct, ensuring the model layer remains agnostic of the TUI library, facilitating the long-term goal of a Tauri GUI.
- **Robust State Management:** The use of `std::mem::take` in Plan 02 to manage the `click_regions` vector is a clever and idiomatic way to satisfy the Rust borrow checker during the render loop while avoiding unnecessary allocations.
- **Windows Resilience:** Plan 01 proactively addresses a common pitfall in Windows terminal environments (ConPTY limitations) with a graceful degradation strategy, ensuring the app remains stable even when mouse support fails to initialize.
- **Dual-Layer Testing:** Combining tmux-based E2E tests (for workflow logic) with `insta` snapshot tests (for visual regression) provides a "belt and suspenders" approach to quality assurance.
- **Decoupling for Testability:** Plan 04's task of decoupling `render()` functions from the `&App` struct is an excellent refactor that makes the TUI components far more unit-testable and reusable.

### Concerns
- **Event Shadowing/Occlusion (HIGH):** Plan 02 describes pushing `ClickRegion`s to a vector, but it does not explicitly define how to handle overlapping regions (e.g., when a popup/modal is open over the dashboard). Without a "z-index" or a "last-in-first-win" (iterating in reverse) dispatch logic, the user might click "through" a popup and trigger background actions.
- **Tmux Test Brittleness (MEDIUM):** Shell-based tmux testing (`capture-pane` and `send-keys`) is notoriously prone to timing issues and race conditions in CI environments. A simple `assert_contains` might fail if the terminal hasn't finished rendering.
- **Action Enum Bloat (LOW):** Adding `Clone` to all action enums is necessary for the proposed dispatch logic, but if these enums grow to contain large payloads (like whole certificate buffers or strings), cloning them every click could be slightly inefficient, though likely negligible in a TUI context.
- **Mock State Sync (LOW):** Plan 04 relies on a shared mock fixture from Phase 6. Ensure this fixture is comprehensive enough to cover the "no YubiKey" and "error state" snapshots required for the new tests.

### Suggestions
- **Implement Reverse Dispatch:** In `handle_mouse_event` (Plan 02), iterate through the `click_regions` vector in **reverse order** (`click_regions.iter().rev()`). Since popups are usually rendered last, they will be at the end of the vector; reverse iteration ensures the "top-most" element captures the click.
- **Add "Wait-For" Logic to E2E:** Instead of raw `capture` and `assert_contains` in `helpers.sh` (Plan 03), implement a `wait_for_text` function that retries the capture/assert logic for a few seconds before failing. This will significantly reduce CI flakiness.
- **Explicit Modal Handling:** Update the `ClickRegion` struct or the dispatch logic to check if a "modal" state is active in `AppState`. If it is, the dispatcher should ignore any `ClickAction` that doesn't belong to the modal/popup layer.
- **CI Snapshot Management:** Ensure the CI pipeline is configured to fail if snapshots change, but also provide a clear workflow (e.g., a CI artifact or a specific command) for developers to review and update snapshots when intentional UI changes occur.

### Risk Assessment: LOW
The plans are exceptionally well-thought-out. The risks are primarily operational (CI flakiness and UI occlusion) rather than architectural. The separation of concerns between the model's `Region` and the TUI's `Rect` is handled perfectly. By implementing the suggested "reverse dispatch" for mouse events and "wait-for" logic for tmux, the remaining risks will be mitigated. The phase is well-scoped and directly satisfies the v1.1 goal of accessibility and testability.

---

## Consensus Summary

One reviewer (Gemini) provided feedback. Claude CLI was skipped for review independence (currently running inside Claude).

### Agreed Strengths
- ClickRegion/Region architecture correctly keeps ratatui types out of the model layer
- std::mem::take borrow checker pattern is idiomatic and correct
- ConPTY graceful degradation is well-handled
- Dual-layer testing (tmux E2E + insta snapshot) is the right approach
- Decoupling render() from &App in Plan 04 improves testability

### Key Concerns (Prioritized)

| Severity | Concern | Plan | Suggested Fix |
|----------|---------|------|---------------|
| HIGH | Click-through popups — no z-index or last-in-first-win dispatch | 02 | Iterate `click_regions.iter().rev()` in handle_mouse_event |
| MEDIUM | Tmux test brittleness — timing races in CI | 03 | Add `wait_for_text` retry loop in helpers.sh |
| LOW | Large Clone payloads on action enums (e.g. String fields) | 01 | Acceptable for TUI scale; note in plan |
| LOW | Mock fixture may not cover all snapshot states | 04 | Verify mock covers no-YubiKey and error states |

### Divergent Views
N/A — single reviewer.

### Top Actionable Items for Replanning
1. **Plan 02 Task 2**: Change `click_regions.iter()` to `click_regions.iter().rev()` in `handle_mouse_event` to ensure popup/modal regions capture clicks before background regions
2. **Plan 03 Task 1**: Add a `wait_for_text SESSION PATTERN TIMEOUT` helper to `helpers.sh` with retry logic instead of fixed `sleep` + single `assert_contains`
3. **Plan 02 Task 2** (optional): Document that context-menu-open state naturally handles occlusion since menu ClickRegions are pushed after background regions — the `.rev()` fix covers this case correctly
