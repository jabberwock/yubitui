# yubitui Milestones

## v1.0 — Production-Ready

**Shipped:** 2026-03-26
**Phases:** 5 | **Plans:** 21 | **Commits:** 168
**LOC:** ~10,053 Rust | **Files:** 112 changed (+23,893 / -497)
**Timeline:** 2026-03-24 → 2026-03-26 (3 days)

### Delivered

Complete, self-contained YubiKey TUI with zero external CLI dependencies. All card reads go through native PC/SC APDUs (pcsc crate); gpg is used for keyring operations only; ykman binary is not required.

### Key Accomplishments

1. **Interactive key picker + help screen** — arrow-key navigation replaces hardcoded key selection; `?` opens keybinding reference from any screen
2. **PIN unblock wizard** — 4-branch decision tree (reset code → admin PIN → factory reset → abort) with double-confirmation destructive action
3. **SSH setup wizard** — guides non-experts through gpg-agent.conf, agent restart, SSH_AUTH_SOCK, and connection test without leaving the TUI
4. **Full programmatic GPG control** — `--command-fd`/`--status-fd`/`--pinentry-mode loopback` for all PIN ops, key import, and 7-step on-device key generation wizard; zero terminal escape
5. **Native PC/SC APDU protocol** — card.rs module replaces all ykman/gpg card reads with direct APDUs; T=0 GET RESPONSE chaining; BER-TLV parser
6. **PIV certificates screen** — Screen::Piv with native slot occupancy reads for 9a/9c/9d/9e; multi-YubiKey Tab-switching; touch policy per slot; attestation popup

### Known Gaps (Tech Debt)

- Phase 04 VERIFICATION.md missing (functional evidence confirmed by Phase 05 verifier cross-check)
- cargo fmt diffs in src/app.rs, src/ui/pin.rs, src/ui/widgets/popup.rs, src/utils/config.rs
- Char('t') nav arm doesn't clear stale message (cosmetic)
- No 50ms sleep after kill_scdaemon() on Linux (may cause Card Busy on slow teardown)

**Archive:** `.planning/milestones/v1.0-ROADMAP.md`
**Audit:** `.planning/milestones/v1.0-MILESTONE-AUDIT.md`
