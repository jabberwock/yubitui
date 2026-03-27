# Phase 9: OATH/TOTP Screen — Pre-discussion Notes

These design decisions were gathered during the Phase 8 discuss-phase session (2026-03-27) before the scope pivot. Use these as a head start when running `/gsd:discuss-phase 9`.

## Locked decisions from discussion

- **Clock**: Use `chrono::Utc::now()` as TOTP challenge. Trust OS/NTP. No drift compensation in the app. Optionally display system time in UI so users can spot clock skew.
- **Card protocol**: CALCULATE APDU to YubiKey OATH applet — YubiKey is a pure HMAC engine (no internal clock). We send `floor(unix_time / 30)` as 8-byte big-endian challenge.
- **Missing crates**: `hmac` and `sha1` not yet in Cargo.toml — needed for OATH CALCULATE.
- **Built in textual-rs**: Phase 9 is the first new screen in textual-rs. Inherits all Phase 8 design patterns.
- **Design inspiration**: yubioath-flutter was reviewed and found to have poor UX — no rule of thirds, no visible shortcuts, non-obvious click regions. Phase 9 should directly address all three.

## Gray areas still to discuss (run /gsd:discuss-phase 9)

1. **Live refresh strategy** — Poll card every second for countdown clock, or just tick the clock locally and only hit the card at 30s window boundary?
2. **Add account UX** — Sequential wizard (like PIN wizard) or multi-field form with Tab navigation?
3. **Credential row density** — Minimal (name + code) vs full (name + code + type badge + per-row countdown bar)?
4. **HOTP behavior** — Auto-generate on select, or require explicit Enter to consume next counter value?
