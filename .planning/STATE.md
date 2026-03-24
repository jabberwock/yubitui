# Project State

## Current Phase
**Phase 1** — Polish & Cross-Platform Fixes

## Status
active

## Progress
- Phase 1: in progress (plan 02 complete)
- Phase 2: not started
- Phase 3: not started

## Completed Plans
- 01-02: Help screen — ? key opens keybinding reference overlay from any screen (2026-03-24)

## Decisions
- Global ? handler at top of handle_key_event before screen-specific blocks ensures uniform access from all screens
- previous_screen: Screen field stores return destination for modal overlay pattern

## Notes
- Cross-platform requirement is non-negotiable (Linux/macOS/Windows)
- Security rules: no sensitive values in logs, no shell injection, no hardcoded paths
- Always run `cargo clippy -- -D warnings` before committing

## Last Session
- Stopped at: Completed 01-02-PLAN.md (help screen)
- Date: 2026-03-24T18:20:09Z
