# Phase 4: Discussion Log

**Date:** 2026-03-25
**Phase:** 04-programmatic-subprocess-control

This file is a human-readable audit trail of the discuss-phase Q&A session.
It is NOT consumed by downstream agents (researcher, planner, executor).

---

## Area: PIN input in TUI

**Q: How should PIN entry work in the TUI?**
Options: TUI input fields (Recommended) / System pinentry
Selected: **TUI input fields** — build a focused masked input widget; user never leaves the TUI.

**Q: How should characters display as the user types a PIN?**
Options: Dots (●●●●●●) / Asterisks (******) / Fully hidden
Selected: **Dots (●●●●●●)** — character count visible, standard masking.

**Q: How should multi-step PIN entry work (current → new → confirm)?**
Options: Sequential fields on one screen (Recommended) / One field at a time
Selected: **Sequential fields on one screen** — all fields visible, active field highlighted, Tab/Enter moves between them.

---

## Area: Key gen parameters

**Q: How much of the key generation configuration should be user-controlled?**
Options: Sensible defaults (Recommended) / Full wizard in TUI / Skip keygen for now
Selected: **Full wizard in TUI** — algorithm, expiry, name, email, comment.

**Q: How should the backup copy option be handled?**
Options: No backup (Recommended) / Prompt for it
Selected: **Prompt for it** — show a checkbox + file path input in the TUI wizard.

---

## Area: Progress / status feedback

**Q: What should the user see while a gpg operation is running?**
Options: Spinner + status line (Recommended) / Step checklist / Just success/fail
Selected: **Spinner + status line** — current step label ("Verifying current PIN...", "Setting new PIN...").

**Q: How should gpg error codes be surfaced to the user?**
Options: Human-readable messages (Recommended) / Raw gpg output
Selected: **Human-readable messages** — translate gpg status tokens to plain language.

---

## Area: Import key flow

**Q: Should we handle subkey selection during import?**
Options: Auto-map by capability (Recommended) / Show subkey picker
Selected: **Auto-map by capability** — S → sig slot, E → enc slot, A → aut slot.

**Q: After import, what does the user see?**
Options: Result popup (Recommended) / Updated card status
Selected: **Result popup** — shows which slots were filled, then returns to key menu.
