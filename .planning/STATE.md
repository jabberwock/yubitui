---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: Phases 1-3 are already implemented. Starting from Phase 1 to close remaining gaps.
status: active
last_updated: "2026-03-24T19:18:44Z"
progress:
  total_phases: 3
  completed_phases: 1
  total_plans: 7
  completed_plans: 5
---

# Project State

## Current Phase

**Phase 2** — UX Menus, Wizards & Fixes

## Status

active

## Current Plan

Phase 2 — Plan 03 (02-03)

## Progress

[█████░░░░░] 57%

- Phase 1: complete (all 3 plans complete)
- Phase 2: in progress (2 of 4 plans complete)
- Phase 3: not started

## Completed Plans

- 01-01: Interactive key picker — arrow-key navigation replaces hardcoded available_keys[0] (2026-03-24)
- 01-02: Help screen — ? key opens keybinding reference overlay from any screen (2026-03-24)
- 01-03: README roadmap sync — checkboxes corrected, log path platform-aware (2026-03-24)
- 02-01: Foundation infrastructure — popup widgets, mouse capture, gpgconf-authoritative gnupg path (2026-03-24)
- 02-02: PIN unblock wizard — 4-branch decision tree, ykman factory reset with double confirmation (2026-03-24)

## Decisions

- README roadmap checkboxes corrected to reflect actual implementation state (Phase 1-3 items checked accurately)
- Log path note updated with platform-aware language covering Linux/macOS and Windows examples
- Consolidated redundant Phase 2 import lines into single 'Import keys to card (via GPG)' entry
- Global ? handler at top of handle_key_event before screen-specific blocks ensures uniform access from all screens
- previous_screen: Screen field stores return destination for modal overlay pattern
- Interactive key picker: use selected_key_index in KeyState, ratatui List widget with per-item styles for ImportKey screen
- [Phase 02-ux-menus-wizards-fixes]: Used Layout-based centering for popups since Rect::centered() does not exist in ratatui 0.29
- [Phase 02-ux-menus-wizards-fixes]: gpgconf --list-dirs homedir is now authoritative gnupg path source with Windows GPG4Win fallback
- [02-02]: UnblockUserPin kept with #[allow(dead_code)] for compat; UI routes through wizard variants
- [02-02]: factory_reset_openpgp uses ykman (not gpg) -- only ykman supports --force full OpenPGP app reset

## Notes

- Cross-platform requirement is non-negotiable (Linux/macOS/Windows)
- Security rules: no sensitive values in logs, no shell injection, no hardcoded paths
- Always run `cargo clippy -- -D warnings` before committing

## Last Session

- Stopped at: Completed 02-02-PLAN.md (PIN unblock wizard, factory reset, ykman detection)
- Date: 2026-03-24
