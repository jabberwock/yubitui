---
status: resolved
trigger: "Two bugs in the key generation wizard: (1) Enter on the backup filename step activates text editing instead of advancing the wizard — user is stuck; (2) even if wizard completes, no key appears on the card (silent failure)."
created: 2026-03-26T00:00:00Z
updated: 2026-03-26T00:00:00Z
---

## Current Focus

hypothesis: Post-operation "Card removed -- reinsert and retry" noise comes from
  CardCtrl(3) events fired by scdaemon after each successful keytocard slot.
  scdaemon releases the card as normal housekeeping; this fires unconditionally.
  The fix: only push the CardCtrl(3) message when success=false at that point.

test: cargo test 87/87 passes after guard added.
expecting: Three successful slots produce zero "Card removed" noise messages.
next_action: resolved — archive session.

## Symptoms

expected: On the backup step, user types a filename then presses Enter to advance to the next step. At the end of the wizard, a GPG key should be generated on-device.
actual:
  Bug 1: Pressing Enter on the backup filename step enters editing mode (cursor appears in box) instead of advancing the wizard. User cannot progress past this step.
  Bug 2: Wizard steps complete with no error, but checking the card shows no key was generated.
errors: No explicit error messages reported
reproduction:
  Bug 1: Navigate to key generation wizard, reach the backup filename step, press Enter
  Bug 2: Complete the key generation wizard (skipping backup or somehow advancing), wizard finishes but card has no key
started: Never worked as far as the user knows

## Eliminated

- hypothesis: Admin PIN locked (counter "3 0 3" middle field = Admin PIN = 0 retries)
  evidence: |
    parse_pin_status() documents gpg field order as user RC admin.
    "3 0 3" = user_pin=3, reset_code=0, admin_pin=3 (healthy).
    Card Admin PIN is NOT locked. Checkpoint response misread the field order.
    The middle field (0) is the Reset Code, not the Admin PIN.
  timestamp: 2026-03-26

## Evidence

- timestamp: 2026-03-26T03
  checked: generate_key_batch batch file construction and import_key_programmatic AUT slot
  found: |
    Bug A root cause: tui/keys.rs line 67 sets `backup_path = format!("~/yubikey-backup-...gpg")`.
    The literal "~/" is passed to gpg as a command argument, not a shell — no expansion occurs.
    Fix: in generate_key_batch, expand leading "~/" via dirs::home_dir() before passing to gpg.

    Bug B root cause: generate_key_batch only generated a primary SIG key + one ENC subkey.
    The gpg batch file had no AUT subkey. import_key_programmatic scans subkeys for 'a'
    capability; finding none, it sets aut_subkey=None and skips card slot 3. The card's AUT
    slot retained its factory rsa2048 key. The "card removed" errors during AUT were a
    secondary consequence of attempting to transfer a subkey that didn't exist in the keyring.

    Secondary issue: gpg 2.4.9 does NOT support repeated "Subkey-Type" (duplicate keyword error)
    or "Subkey2-Type". The only way to add a second subkey non-interactively is:
    gpg --batch --no-tty --pinentry-mode loopback --passphrase "" --quick-add-key <FP> ed25519 auth

    Verified: --quick-add-key with loopback+passphrase "" works cleanly for %no-protection keys.
    Verified: slot loop in import_key_programmatic needs 50ms sleep between sessions to prevent
    scdaemon "card removed" on successive gpg --edit-key processes.
  implication: |
    Fix A: expand "~/" in generate_key_batch backup path via dirs::home_dir().
    Fix B: call --quick-add-key after generate_key_batch succeeds; add 50ms sleep in slot loop.

- timestamp: 2026-03-26T02
  checked: /tmp/yubitui-keytocard-D60E57EB5A9813ABEEC6938A35F65BB980AF81B0-slot{1,2,3}.log
  found: |
    All three slot sessions fail with SC_OP_FAILURE 2 (Bad PIN).
    For each slot: gpg issues GET_HIDDEN passphrase.enter exactly once.
    State machine responds with key_passphrase="" (empty string, because %no-protection).
    gpg interprets empty string as the Admin PIN → Bad PIN → SC_OP_FAILURE 2.
    The Admin PIN entered by user in the wizard is never sent to gpg.
    Also noted: CARDCTRL 3 fires after cardedit.genkeys.storekeytype (card removed event),
    but the session continues and the passphrase prompt still fires — so card removal is not
    the failure cause. The SC_OP_FAILURE is directly caused by the wrong PIN.
  implication: |
    run_keytocard_session's passphrase.enter handler has a two-step assumption:
    "first passphrase.enter = key passphrase, second = admin PIN."
    For %no-protection keys (empty key_passphrase), gpg never asks for a key passphrase,
    so the FIRST passphrase.enter is the Admin PIN. But the code sends "" instead.
    Fix: when key_passphrase is empty, treat every passphrase.enter as Admin PIN.

- timestamp: 2026-03-26
  checked: src/tui/keys.rs KeyGenStep::Backup Enter handler (lines 581-591)
  found: |
    When backup=true and editing_path=false: Enter sets editing_path=true (enters edit mode).
    When editing_path=true: Enter sets editing_path=false (exits edit mode, stays on Backup step).
    When backup=false and editing_path=false: Enter advances to Confirm (correct path).
    The "[Enter] Continue" help text on the Backup step render (line 1715) is therefore wrong
    for the backup=true case — pressing Enter enters edit mode, not Continue.
  implication: User with backup=true presses Enter: enters edit mode. Presses Enter again: exits edit mode. Stuck in loop. Cannot reach Confirm.

- timestamp: 2026-03-26
  checked: src/app.rs execute_keygen_batch() and src/model/key_operations.rs generate_key_batch()
  found: |
    execute_keygen_batch calls generate_key_batch(&params, &admin_pin).
    generate_key_batch parameter is declared as (_admin_pin: &str) — unused.
    generate_key_batch uses gpg --batch --gen-key with %no-protection; creates key in local keyring.
    After generate_key_batch, execute_keygen_batch does NOT call import_key_programmatic().
    The key fingerprint is available in result.fingerprint after generation.
    import_key_programmatic(fingerprint, "", admin_pin) would transfer it to the card
    (empty passphrase because %no-protection was used).
  implication: Keygen wizard completes successfully (local key created) but no key ever reaches the card. Silent failure.

- timestamp: 2026-03-26
  checked: Phase 04 context D-09 and D-11
  found: |
    D-09 says wizard ends with "confirm + Admin PIN entry" — Admin PIN is the CARD admin PIN.
    D-11 says %no-protection "since the card PIN is the protection."
    This implies the intent was: generate local key (unprotected) → transfer to card using admin PIN.
    The admin_pin collected in the wizard is the card admin PIN for keytocard, not a key passphrase.
  implication: Confirms intended flow: generate_key_batch then import_key_programmatic(fp, "", admin_pin).

## Resolution

root_cause: |
  Bug A — Backup path not expanded:
    tui/keys.rs initialises backup_path with a literal "~/..." string.
    When generate_key_batch passes this to gpg as a Command argument, the shell
    never processes it — gpg receives the literal "~/..." path and fails to create the file.

  Bug B — AUT slot card disconnect / rsa2048 unchanged:
    generate_key_batch only generated SIG (primary) + ENC (one subkey).
    gpg 2.4.9 does not support multiple Subkey-Type declarations in a batch file
    (returns "duplicate keyword" error). No AUT subkey was created.
    import_key_programmatic found no 'a'-capability subkey → aut_subkey=None → slot 3 skipped.
    The card's AUT slot retained its factory rsa2048 key the whole time.
    The "card removed" errors were a secondary symptom — scdaemon needs 50ms between
    successive gpg --edit-key sessions or it reports card removal on the next session.

fix: |
  Bug A (src/model/key_operations.rs): In generate_key_batch, before passing backup_path to
    gpg --export-secret-keys, expand any leading "~/" using dirs::home_dir(). The expansion
    is applied to the actual Command argument, so gpg receives an absolute path.

  Bug B (src/model/key_operations.rs):
    (1) After generate_key_batch generates the key and obtains the fingerprint, call
        gpg --batch --no-tty --pinentry-mode loopback --passphrase "" --quick-add-key <FP> <algo> auth
        to add an authentication subkey. Algorithm: "ed25519" for Ed25519 keys, "rsa2048"/"rsa4096"
        for RSA keys. import_key_programmatic then finds an 'a'-capability subkey and transfers it.
    (2) In the slot loop of import_key_programmatic, added a 50ms sleep after each
        run_keytocard_session call so scdaemon can settle before the next gpg session starts.

verification: |
  87/87 tests pass. Real-card verification confirmed by user:
  SIG ✓  ENC ✓  AUT ✓ on card.
  Backup exported successfully.
  Post-operation "Card removed -- reinsert and retry" noise fixed by guarding
  CardCtrl(3) message emission on !success in run_keytocard_session.
files_changed:
  - src/tui/keys.rs
  - src/app.rs
  - src/model/key_operations.rs
