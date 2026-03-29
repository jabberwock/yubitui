# Phase 13: UI Polish — Context

**Gathered:** 2026-03-29
**Status:** Ready for planning
**Source:** User feedback + codebase analysis

<domain>
## Phase Boundary

Bring all screens up to PIN Management's visual standard. The migration to textual-rs (Phase 8) preserved functionality but left screens as flat Label lists. This phase applies consistent visual hierarchy, replaces plain label-based action lists with Button widgets, and uses the richer widget palette textual-rs offers.

**Screens to polish:**
- Dashboard — status indicators, layout
- KeysScreen — action buttons, data layout
- PivScreen — slot list with cursor, action buttons
- DiagnosticsScreen — status indicators, cleaner layout
- OathScreen — account list, countdown bar
- Fido2Screen — credential list, status
- OtpScreen — slot display
- HelpScreen / GlossaryScreen — Markdown rendering

**Out of scope:**
- New functionality or card operations
- Navigation changes (key bindings stay the same)
- Backend / model layer changes

</domain>

<decisions>
## Implementation Decisions

### Reference Standard
- PIN Management screen is the approved visual baseline — uses `Button` for actions, `Label` for data, `Header`/`Footer` chrome, status words in brackets `[OK]`/`[DANGER]`/`[BLOCKED]`
- Match this pattern across all screens

### Widget Upgrade Priorities
- **Action items:** Replace all `Label::new("  g  Generate")` style action labels with `Button::new()` — Buttons render with visual affordance (border + highlight)
- **Tabular data:** Use `DataTable` for key slot lists, PIV slot tables, OTP slot rows — cleaner than stacked Labels
- **Status badges:** Use bracket notation `[OK]`, `[EMPTY]`, `[SET]`, `[BLOCKED]` consistently — already used in PIN Management
- **Countdown:** `ProgressBar` or `Sparkline` for OATH TOTP countdown instead of ASCII bar
- **Long text:** Use `Markdown` for HelpScreen and GlossaryScreen content instead of raw Labels

### Layout
- `Vertical` layout is default (compose() returns vec of children)
- Use `Horizontal` layout for side-by-side elements (label + status badge on same line) where currently done with format strings
- Empty `Label::new("")` spacers are fine for section gaps — keep using them

### Consistency Rules
- Every screen: `Header` → data section → blank spacer → action buttons → `Footer`
- No screen should have action items only accessible via keybinding text — all primary actions must have a Button
- Section headings: use bold text via Label or a divider line (`Label::new("─".repeat(40))`)
- YubiKey-absent state: every screen shows "No YubiKey detected — insert device and press R" with a refresh button

### Rollout Plan
- One plan per screen group (high-traffic screens first)
- Snapshot tests update as screens change
- No new keybindings; Buttons fire existing action strings

### Claude's Discretion
- Exact wording of status labels
- Whether to use Collapsible for optional detail sections (e.g. touch policies in KeysScreen)
- Sparkline vs ProgressBar for OATH countdown
- DataTable column widths

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Reference screen (approved visual standard)
- `src/tui/pin.rs` — PIN Management screen; use as visual template for all screens

### Screens to modify
- `src/tui/dashboard.rs` — Dashboard
- `src/tui/keys.rs` — OpenPGP Keys + sub-screens
- `src/tui/piv.rs` — PIV screen
- `src/tui/diagnostics.rs` — Diagnostics
- `src/tui/oath.rs` — OATH/TOTP
- `src/tui/fido2.rs` — FIDO2
- `src/tui/otp.rs` — OTP Slots
- `src/tui/help.rs` — Help
- `src/tui/glossary.rs` — Glossary

### Widget palette (textual-rs 0.3.11)
Available: `Button`, `ButtonVariant`, `DataTable`, `ColumnDef`, `ProgressBar`, `ListView`, `Horizontal`, `Vertical`, `Markdown`, `Collapsible`, `ScrollView`, `LoadingIndicator`, `Sparkline`
Import from: `textual_rs::{...}`

### Project state
- `.planning/STATE.md` — decisions log, especially Phase 08 migration notes
- `.planning/ROADMAP.md` — phase 13 goal

</canonical_refs>

<specifics>
## Specific Ideas

User feedback (verbatim): "I think yubitui seems extremely basic. PIN Management is the only screen I think looks semi-modern."

@kali feedback: "Most screens are very flat: plain Label text with ASCII [OK]/[!!] markers, no visual hierarchy beyond Header/Footer. Navigation is just numbered text lines. Functional but minimal."

@win feedback: "ModalScreen wrapper removal may have stripped visual layering. Worth checking if PopupWidget styling regressed."

The goal is not a complete redesign — it is bringing the rest of the app up to PIN Management's quality, which is achievable with targeted widget substitutions and layout improvements.

</specifics>

<deferred>
## Deferred Ideas

- Full two-column sidebar layout (requires textual-rs layout primitives beyond what's currently stable)
- Animations or transition effects
- Custom CSS themes per screen type
- Mouse click regions for buttons (can follow in a separate phase)

</deferred>

---

*Phase: 13-ui-polish*
*Context gathered: 2026-03-29*
