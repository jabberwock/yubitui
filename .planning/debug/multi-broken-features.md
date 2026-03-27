---
status: awaiting_human_verify
trigger: "Investigate multiple broken features in yubitui. Find and fix ALL of them."
created: 2026-03-25T00:00:00Z
updated: 2026-03-25T02:00:00Z
---

## Current Focus

hypothesis: |
  Three additional root causes identified from new evidence:

  Bug A (on_card missed case): parse_subkey_capabilities() only checked for sec>/ssb> record
    types, but gpg can show a card-referenced key as plain `sec`/`ssb` with the card AID in
    field 15 (index 14) — this happens when a keytocard op updated the keybox but failed to
    write actual key material to card. list_gpg_keys() also included these as importable.

  Bug B (Key Attributes [empty] = correct): The card genuinely has no keys. The prior
    keytocard failures never wrote fingerprints to DO 0xC5. gpg's keybox incorrectly
    references the card via token S/N in field 15. The [empty] display is accurate.
    The ❌ Sign ❌ Encrypt ❌ Auth on dashboard is also correct — key presence, not touch policy.

  Bug C (card status one-line): view_card_status() returns multi-line string via join("\n"),
    stored in key_state.message, then rendered as Span::raw(msg) — ratatui does not split
    Span text on \n, so all lines appear concatenated. Fixed by splitting on msg.lines().

test: Applied all three fixes, cargo build + cargo test pass
expecting: Import skips card-referenced keys; card status displays on separate lines
next_action: Human verify — test import with sec+field15 key, check card status formatting

## Symptoms

expected: Key import works, key info displays, SSH export works, slot view shows card slots
actual:
  Bug1: Key import fails with "Unusable secret key" when key already moved to card stub
  Bug2: Key info / SSH export / slot view all broken since phase 05 changes
errors:
  Bug1: "gpg: KEYTOCARD failed: Unusable secret key" — key already on card, local stub not exportable
  Bug2: Unknown — need to trace code
reproduction:
  Bug1: Try to import a key that was already imported to the card in a previous session
  Bug2: Try to view key info, export SSH key, or view card slots
started: After phase 05 changes (native PC/SC APDU calls replaced ykman/gpg-card-status)

## Eliminated

- hypothesis: Only sec>/ssb>/sec#/ssb# record types indicate card keys
  evidence: gpg shows sec+ssb with card serial in field 15 after partial keytocard failure
  timestamp: 2026-03-25

- hypothesis: Key Attributes [empty] is a PC/SC read bug
  evidence: Card genuinely has no keys — prior imports all failed with SC_OP_FAILURE.
    DO 0xC5 correctly returns zeros. gpg keybox is stale (references card but no keys there).
  timestamp: 2026-03-25

## Evidence

- timestamp: 2026-03-25
  checked: key_operations.rs run_keytocard_session() GetBool handler
  found: GpgStatus::GetBool { .. } always sends 'y' including for cardedit.genkeys.replace_key
  implication: When key already on card, gpg asks replace_key, we say y, then tries to export stub → fails

- timestamp: 2026-03-25
  checked: key_operations.rs get_key_attributes()
  found: Calls connect_to_openpgp_card() which kills scdaemon; does NOT restart scdaemon after
  implication: After 'k' (key attributes), scdaemon stays dead, breaking any subsequent gpg operations

- timestamp: 2026-03-25
  checked: detection.rs detect_all_yubikey_states()
  found: Restarts scdaemon after exclusive use (line 143-145)
  implication: get_ssh_public_key_text() and view_card_status() which use detect_all() are OK

- timestamp: 2026-03-25
  checked: gpg --list-secret-keys output for card stub key
  found: Card stub key shows D2760001240100000006269280890000 in field 11 (not sec:>: stub marker)
  implication: list_gpg_keys() includes card-stub keys because it only checks parts[0] == "sec"

- timestamp: 2026-03-25
  checked: parse_subkey_capabilities() for card-stub detection
  found: gpg --list-keys (public) works fine; gpg --list-secret-keys shows ssb# or sec# for stubs
  implication: Can detect stubs via --list-secret-keys with colons by checking for "#" in sec/ssb type

- timestamp: 2026-03-25
  checked: get_key_attributes() call site in app.rs
  found: Called immediately on 'k' press (no Enter needed) and on 's' press (SSH pubkey popup)
  implication: Both calls kill scdaemon without restart → breaks subsequent import or PIN operations

- timestamp: 2026-03-25
  checked: New user evidence — gpg --list-secret-keys output
  found: |
    Records show `sec` and `ssb` (NOT sec>/ssb>) but card serial D2760001240100000006269280890000
    is in field 15 of both records. parse_subkey_capabilities() cross-reference only flagged
    sec>/ssb>/sec#/ssb# — missed plain sec/ssb with token S/N in field 15.
  implication: on_card remained false for these keys → import was attempted → fails with SC_OP_FAILURE

- timestamp: 2026-03-25
  checked: Card DO 0xC5 via native PC/SC (via view_card_status and get_key_attributes)
  found: Returns all zeros for all three fingerprint slots → build_key_info returns None → [empty]
  implication: The card genuinely has no keys. Prior keytocard attempts all failed (SC_OP_FAILURE).
    The gpg keybox entry referencing the card is stale from the failed partial import.

- timestamp: 2026-03-25
  checked: keys.rs render_keys_main — message display
  found: state.message (multi-line) passed as Span::raw(msg) — ratatui does not split on \n
  implication: Card status appears as one cramped line. Fixed by splitting on msg.lines().

## Resolution

root_cause: |
  Bug 1 — "Unusable secret key" (import fails):
    Three-layer failure:
    a) parse_subkey_capabilities() only checked sec>/ssb>/sec#/ssb# for on_card detection.
       After a failed keytocard that updated gpg's keybox, the key shows as plain `sec`/`ssb`
       with the card AID in field 15 (token S/N). The cross-reference missed this case.
    b) list_gpg_keys() included these card-referenced keys as importable (only excluded sec>/sec#).
    c) The import was attempted → gpg-agent refuses to export local material it considers card-bound
       → SC_OP_FAILURE / "Unusable secret key".

  Bug 2 — Key Attributes [empty] / Keys: ❌ (correct behavior):
    The card genuinely has no keys loaded. All prior keytocard attempts failed with SC_OP_FAILURE
    before writing fingerprints to DO 0xC5. gpg's keybox is stale — it references the card via
    token S/N in field 15 of sec/ssb records, but the card itself has no key material.
    DO 0xC5 correctly returns 60 zero bytes → [empty] and ❌ are accurate. Not a bug.

  Bug 3 — Card status all on one line:
    view_card_status() returns lines.join("\n"). Stored in key_state.message. The Keys main screen
    renders it as Span::raw(msg) — ratatui renders spans as a single paragraph segment and does
    not interpret \n as a line break. Result: all lines concatenated into one visual line.

  Previously fixed:
  Bug 2b — list_gpg_keys included subkey IDs alongside primary key IDs (fixed earlier)
  Bug 2c — scdaemon not restarted after get_key_attributes() (fixed earlier)
  Bug 3b — ViewStatus screen copy updated (fixed earlier)

fix: |
  1. parse_subkey_capabilities(): For `sec` and `ssb` records in --list-secret-keys output,
     also check field 14 (0-indexed, = field 15 in gpg docs = token S/N). If non-empty,
     mark on_card=true. This catches the partial-keytocard case where gpg's keybox was
     updated but record type is still plain sec/ssb.

  2. list_gpg_keys(): For `sec` records, check field 14 (token S/N). If non-empty, set
     is_importable=false. Prevents showing stale card-referenced keys in the import list.

  3. keys.rs render_keys_main(): Replace Span::raw(msg) with msg.lines() loop that pushes
     each line as a separate Line<>. Both the "no YubiKey" branch and the "yubikey present"
     branch are fixed.

verification: cargo build clean, all 85 tests pass. Functional verification pending.
files_changed:
  - src/yubikey/key_operations.rs
  - src/ui/keys.rs
