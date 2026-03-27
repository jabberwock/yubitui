# GSD Debug Knowledge Base

Resolved debug sessions. Used by `gsd-debugger` to surface known-pattern hypotheses at the start of new investigations.

---

## touch-policy-silent-fail — touch policy wizard silently fails: DO 0xF9 TLV tag byte mistaken for KDF-active flag
- **Date:** 2026-03-26
- **Error patterns:** touch policy unchanged, UIF off, set_touch_policy bail, KDF PIN hashing, DO 0xF9, VERIFY ykman, S2K, silent fail, no error message
- **Root cause:** DO 0xF9 returns [81 01 00] (tag=0x81, len=1, value=0x00 = no KDF). Code read raw first byte 0x81 (the tag byte) and concluded KDF was active. Then tried to parse tags 0x82/0x83/0x86 which don't exist in a "no KDF" response → error → bail with "requires ykman" → VERIFY and PUT DATA never sent.
- **Fix:** parse_kdf_do() now reads tag 0x81's VALUE (not the raw first byte) to determine algorithm; returns Ok(None) when algorithm == 0x00. kdf_hash_pin() added for when KDF is genuinely active. set_touch_policy() uses hashed or raw PIN depending on parse_kdf_do() result.
- **Files changed:** Cargo.toml, src/model/touch_policy.rs, src/app.rs
---

## keygen-backup-enter-silent-fail — keygen wizard: Enter stuck on backup step, no key on card, CardCtrl(3) noise after success
- **Date:** 2026-03-26
- **Error patterns:** backup path tilde expansion, Enter editing mode, silent failure keytocard, Card removed reinsert retry, AUT subkey missing, sc_op_failure bad PIN, CardCtrl 3, scdaemon card removed, %no-protection passphrase
- **Root cause:** (1) Backup step Enter handler toggled editing_path flag instead of advancing wizard. (2) generate_key_batch used literal "~/" path (no tilde expansion in Command args). (3) gpg batch file only generated SIG+ENC subkeys — no AUT subkey; gpg 2.4.9 rejects duplicate Subkey-Type. (4) run_keytocard_session sent empty string as Admin PIN for %no-protection keys (should send actual admin_pin on first passphrase.enter). (5) No 50ms sleep between slot sessions caused scdaemon "card removed" on next session. (6) CardCtrl(3) was unconditionally pushed to messages after each slot — scdaemon fires it as normal housekeeping after successful keytocard, producing 3x noise messages.
- **Fix:** (1) Added Enter→advance logic in tui/keys.rs Backup step handler. (2) Expanded "~/" via dirs::home_dir() before passing to gpg. (3) Used --quick-add-key to add AUT subkey after batch generation. (4) Fixed passphrase.enter handler: when key_passphrase is empty, first prompt is Admin PIN. (5) Added std::thread::sleep(50ms) after each run_keytocard_session. (6) Guarded CardCtrl(3) message push on !success in run_keytocard_session.
- **Files changed:** src/tui/keys.rs, src/app.rs, src/model/key_operations.rs
---
