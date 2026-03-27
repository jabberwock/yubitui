---
status: awaiting_human_verify
trigger: "GPG keytocard operation fails on ENC slot with 'Card removed -- reinsert and retry' even when running one gpg --edit-key session per slot."
created: 2026-03-25T00:00:00Z
updated: 2026-03-27T03:00:00Z
---

## Current Focus

hypothesis: CONFIRMED (follow-up) — gpg-agent passphrase cache is warm from a prior operation, so gpg issues only ONE passphrase.enter in the keytocard session (the Admin PIN prompt). The key_passphrase_sent logic unconditionally sends key_passphrase on the first passphrase.enter when key_passphrase is non-empty, feeding the key passphrase to the card as the Admin PIN → SC_OP_FAILURE 2 (Bad PIN).
test: Read debug logs from /tmp/yubitui-keytocard-3A663583*. Each slot shows: cardedit.genkeys.storekeytype → SEND slot → GOT_IT → GET_HIDDEN passphrase.enter → SEND <key_passphrase> → GOT_IT → SC_OP_FAILURE 2. Only ONE passphrase.enter fires; the code sends key_passphrase for it instead of admin_pin.
expecting: Clearing the gpg-agent passphrase cache (CLEAR_PASSPHRASE <keygrip>) before each keytocard session forces gpg to always ask for the key passphrase first (prompt 1) then the Admin PIN (prompt 2), making the prompt count deterministic and the existing 2-prompt logic correct.
next_action: Add CLEAR_PASSPHRASE call via gpg-connect-agent before run_keytocard_session; update tests

## Symptoms

### Original (session 1)
expected: All three slots (SIG, ENC, AUT) get imported to the YubiKey successfully
actual: ENC slot fails with "Card removed -- reinsert and retry" + "Smartcard operation failed" + "ENC slot import failed". SIG and AUT show — (not filled).
errors:
  - Card removed -- reinsert and retry
  - Smartcard operation failed
  - ENC slot import failed
reproduction: Run key import flow in yubitui, select a GPG key, enter admin PIN, trigger import
started: Still broken after commits e084bc9 and c730e67

### Follow-up (session 2) — appeared after SC_OP_SUCCESS/save-deferred fix
expected: Key imports cleanly, all slots filled, no PIN prompts after — admin PIN consumed exactly once
actual: Import appears to succeed (no crash), then all three slots fail silently; admin PIN retries consumed
errors: SC_OP_FAILURE 2 (Bad PIN) on every slot
reproduction: Import any passphrase-protected GPG key after warm gpg-agent session (prior gpg operation touched the key)
started: After save-deferred fix was applied; predicted by Gemini reviewer as HIGH concern

## Eliminated

- hypothesis: UI refresh (detect_all) killing scdaemon between slots
  evidence: execute_key_import is synchronous; detect_all is called only after import_key_programmatic returns
  timestamp: 2026-03-25T00:01:00Z

- hypothesis: Separate gpg sessions still sharing scdaemon state
  evidence: Each session is a fresh gpg process; the real failure is stdin ordering, not scdaemon state
  timestamp: 2026-03-25T00:01:00Z

- hypothesis: gpg always asks for key passphrase before Admin PIN (two passphrase.enter per session)
  evidence: /tmp/yubitui-keytocard-3A663583*-slot1.log shows exactly ONE passphrase.enter when gpg-agent cache is warm; code sends key_passphrase for it, card rejects with SC_OP_FAILURE 2 (Bad PIN); the two-prompt sequence only appears when cache is cold (no prior gpg operation on this key)
  timestamp: 2026-03-27T03:00:00Z

## Evidence

- timestamp: 2026-03-25T00:01:00Z
  checked: run_keytocard_session stdin write order (key_operations.rs:329-332)
  found: Four commands written upfront before stderr thread starts: `key N`, `keytocard`, `{slot}`, `save`; PIN written later in event loop
  implication: When gpg issues GET_HIDDEN for admin PIN, it reads from command-fd; the next buffered byte is `save\n` (already in pipe), not the PIN. gpg uses "save" as the admin PIN, authentication fails, card operation fails.

- timestamp: 2026-03-25T00:01:00Z
  checked: why SIG/AUT show — (not filled)
  found: parse_subkey_capabilities only scans `sub`/`ssb` lines; primary key (`pub`/`sec`) is ignored; RSA test key has primary with sign capability as `pub` line — so sig_subkey=None, aut_subkey=None, only enc_subkey=Some(1)
  implication: SIG and AUT always skip for a standard RSA key with signing primary + encryption subkey. This is a separate (pre-existing) issue from the Card removed bug.

- timestamp: 2026-03-25T00:01:00Z
  checked: gpg --list-keys --with-colons output for the test key
  found: pub:u:4096:1:A123FF7475998643 scESC (primary, sign+certify), sub:u:4096:1:DB6A9025EB06A61D e (encryption subkey only)
  implication: Confirms enc_subkey=Some(1), sig_subkey=None, aut_subkey=None for this key

- timestamp: 2026-03-27T03:00:00Z
  checked: /tmp/yubitui-keytocard-3A663583E69CDD09E6F86AC5108EDD663A0FA1A4-slot{1,2,3}.log (most recent failing run, passphrase-protected key)
  found: Every slot shows: GET_LINE cardedit.genkeys.storekeytype → SEND slot → GOT_IT → INQUIRE_MAXLEN 100 → GET_HIDDEN passphrase.enter → SEND <key_passphrase> → GOT_IT → SC_OP_FAILURE 2 (Bad PIN) → SEND quit. Only ONE passphrase.enter fires per session.
  implication: gpg-agent has the key passphrase cached from a prior operation. With warm cache, gpg does NOT re-ask for the key passphrase — it decrypts the local key material internally. The single passphrase.enter is the card Admin PIN request. Code sends key_passphrase for it → card rejects with Bad PIN.

- timestamp: 2026-03-27T03:00:00Z
  checked: /tmp/yubitui-keytocard-4C8C79C2459357A387C15C9928E45AC23AB721C5-slot3.log (earlier run, old code path)
  found: TWO passphrase.enter prompts, both received <admin_pin>, session succeeded. Old code (no key_passphrase logic) sent admin_pin for all passphrase.enter prompts.
  implication: Confirms that when cache is cold OR key has no passphrase, two prompts appear and both need admin_pin. When cache is warm, one prompt appears and it is the admin_pin. The count is not a reliable discriminator without also clearing the agent cache.

- timestamp: 2026-03-27T03:00:00Z
  checked: key_passphrase_sent logic in run_keytocard_session (lines 573-584)
  found: Logic sends key_passphrase on first passphrase.enter if key_passphrase is non-empty. This is correct when cache is cold but wrong when cache is warm (0 key passphrase prompts, 1 admin PIN prompt).
  implication: The count-based approach is inherently ambiguous. Fix: make the count deterministic by always clearing the agent cache before starting the keytocard session, forcing cold-cache behavior (2 prompts: key passphrase first, admin PIN second).

## Resolution

root_cause: gpg-agent caches the key passphrase after any gpg operation that touches the key. When keytocard runs with a warm cache, gpg issues only ONE passphrase.enter (the card Admin PIN). The key_passphrase_sent logic sends key_passphrase for the first passphrase.enter when key_passphrase is non-empty, feeding the wrong value to the card → SC_OP_FAILURE 2 (Bad PIN) on every slot.
fix: Before each keytocard session, invoke `gpg-connect-agent "CLEAR_PASSPHRASE <keygrip>"` to flush the agent's passphrase cache for that key. This forces gpg to always issue two passphrase.enter prompts for a passphrase-protected key (key passphrase first, then Admin PIN), making the existing count-based logic in run_keytocard_session correct and deterministic. Only called when key_passphrase is non-empty (passphrase-protected keys). Inserted in import_key_programmatic after the shadow stub cleanup block, before run_keytocard_session.
verification: 109 tests pass; awaiting live YubiKey test with passphrase-protected key
files_changed: [src/model/key_operations.rs]
