# Phase 6: Tech Debt + Infrastructure - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-26
**Discussed areas:** Mock fixture design, Architecture migration, Navigation state ownership, CI lint enforcement

---

## Todo Review

**Q:** Fold in "Fix wrong model detection when management AID unavailable"?
**Options:** Fold it in / Leave it for later
**Selected:** Fold it in
**Rationale:** card.rs and detection.rs are being touched for the model split anyway — natural fit.

---

## Area 1: Mock fixture design

**Q1:** What YubiKey state should the mock represent?
**Options:** Fully configured / Factory fresh / Multiple presets
**Selected:** Fully configured
**Rationale:** Best for testing existing screens — Phase 7 E2E tests need real content to exercise.

**Q2:** How should the mock fixture data be defined?
**Options:** Hardcoded Rust struct / Fixture file (JSON/TOML)
**Selected:** Hardcoded Rust struct
**Rationale:** Simple, always consistent, no file I/O, compiles in.

---

## Area 2: Architecture migration

**Q1:** What should happen to src/yubikey/ and src/ui/?
**Options:** Rename yubikey/→model/, ui/→tui/ / Create new src/model/ for state types only / You decide
**Selected:** Rename: src/yubikey/ → src/model/, src/ui/ → src/tui/

**Q2:** Should the migration happen in one commit or incrementally?
**Options:** Big-bang in one plan / Incremental over multiple plans
**Selected:** Big-bang in one plan
**Rationale:** Easier to reason about; CI stays green end-to-end.

**Q3:** After the rename, what should app.rs become?
**Options:** Thin orchestrator — stays at src/app.rs / Move into src/tui/app.rs / You decide
**Selected:** Thin orchestrator — stays at src/app.rs

**Q4:** Any ratatui imports in src/yubikey/ right now?
**Options:** No / Some to remove / Not sure — check it
**Selected:** Not sure — check it
**Finding (grep):** Zero ratatui imports in src/yubikey/ — no cleanup needed, just enforce boundary.

---

## Area 3: Navigation state ownership

**Q1:** Where does current_screen/previous_screen/popup state live after the split?
**Options:** src/model/ — pure state, no ratatui / Stays in app.rs / You decide
**Selected:** src/model/ — pure state, no ratatui
**Rationale:** Navigation state is just enums and booleans — no ratatui types. AppState in src/model/ holds it; Tauri can serialize the full graph.

---

## Area 4: CI lint enforcement

**Q1:** How to enforce "no ratatui in src/model/" in CI?
**Options:** grep script in CI / Cargo workspace feature gates / You decide
**Selected:** grep script in CI
**Rationale:** Fast, zero dependencies, fails loudly. Fits the existing GitHub Actions matrix.
