# Phase 05 — UI Review

**Audited:** 2026-03-25
**Baseline:** Abstract TUI standards (no UI-SPEC.md — Rust ratatui terminal application)
**Screenshots:** Not captured — TUI application, no web dev server. Code-only audit.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Copy is specific and actionable; "Working..." fallback and stale "Launching GPG" strings are minor gaps |
| 2. Visuals | 3/4 | Clear hierarchy via color and bold; touch policy confirm screen loses context (full-area takeover) |
| 3. Color | 3/4 | Semantic color usage is consistent; 7 named colors in use is appropriate for TUI status semantics |
| 4. Typography | 4/4 | Two text weights (normal + BOLD) used disciplinedly; no font size concept in TUI applies |
| 5. Spacing | 3/4 | Layout constraints are consistent; `centered_area` in popup.rs uses a buggy percentage calculation that clips popups on short terminals |
| 6. Experience Design | 4/4 | Loading spinners, error states, empty states, destructive confirmation — all present and correct |

**Overall: 20/24**

---

## Top 3 Priority Fixes

1. **Stale "Launching GPG" screens in PIN management** — Users who know Phase 5 replaced all GPG card reads will see misleading copy; anyone who hits these screens while the TUI is mid-flow gets incorrect expectations about what is about to happen. Four screens are affected: Change User PIN (`pin.rs:235`), Change Admin PIN (`pin.rs:250`), Set Reset Code (`pin.rs:265`), Unblock User PIN (`pin.rs:278`). Fix: change copy to "Preparing PIN operation..." or the specific gpg command name that *is* still used (gpg --edit-key for PIN changes).

2. **`centered_area` popup uses wrong vertical centering math in `popup.rs`** — Lines 13-17 compute vertical split with `Constraint::Percentage((100 - height.min(100)) / 2)`. The `height` param is a `u16` line count (e.g. `8` for a confirm dialog), not a percentage. This means at a typical 24-line terminal the vertical margins are computed as `(100 - 8) / 2 = 46%` each, squeezing the `Length(8)` popup against an oversized percentage floor. The `pin_input.rs` widget has a correct `centered_area` implementation using `Length(v_margin)` — use that same approach in `popup.rs`. Impact: confirm dialogs and result popups can be clipped or badly positioned on short terminals.

3. **"Working..." generic fallback text in `OperationRunning` progress popup** — `pin.rs:110` calls `render_progress_popup` with `.unwrap_or("Working...")` when `operation_status` is `None`. If the caller forgets to set `operation_status` before transitioning to `OperationRunning`, users see a meaningless status. Fix: change the fallback to `"PIN operation in progress…"` and audit all `PinScreen::OperationRunning` transitions in `app.rs` to confirm `operation_status` is always set before the screen transition (lines ~831-832 in app.rs set both together, which is correct, but the fallback should still be meaningful).

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)

**Strengths:**
- Empty states are specific: "No GPG keys found in keyring. Generate a key first, or import one with: gpg --import <file>" (`keys.rs:364`) — gives users an actionable command.
- No-YubiKey empty state includes troubleshooting steps: check USB, run diagnostics (`dashboard.rs:94-101`).
- Factory reset warning is thorough and lists what will be destroyed (`pin.rs:474-506`).
- PIN unblock wizard surfaces real retry counts inline ("Your Reset Code has {retries} retries remaining" — `pin.rs:436`).
- Touch policy `Fixed (IRREVERSIBLE)` and `Cached-Fixed (IRREVERSIBLE)` labels at point of selection (`keys.rs:629`) — correct placement of risk language.
- Key attributes empty slots show `[empty]` (not "null", "N/A", or "—") which is idiomatic for terminal display (`keys.rs:476, 492, 508`).

**Gaps:**
- `pin.rs:235,250,265,278` — four confirmation screens say "Launching GPG to change/set/unblock...". These still perform gpg subprocess calls (PIN changes use `gpg --edit-key` which is in scope per CONTEXT.md), but the phrasing implies the TUI is handing off entirely. Users cannot tell whether the action will happen inside the TUI or open a PTY. Consider "GPG will prompt you for your PIN" instead of "Launching GPG".
- `pin.rs:110` — `unwrap_or("Working...")` is a generic fallback. Low-risk since the caller in app.rs sets `operation_status` before transitioning, but the string itself is uninformative if the fallback ever fires.
- `keys.rs:1036` — `unwrap_or("Operation complete.")` is slightly more meaningful than "Working..." but still generic. For an operation that could have succeeded or failed, the fallback message should not imply success. Consider `unwrap_or("Operation finished.")` or audit that this path is only reached after a success branch.
- `keys.rs:317` — "Launching GPG to show full card status" (ViewStatus screen). After Phase 5 this is a legacy path since card state is read natively. If this screen is still reachable, the copy should reflect what actually happens.

### Pillar 2: Visuals (3/4)

**Strengths:**
- Three-panel layout (title bar / content / actions) is used consistently across Dashboard, PIN, Keys, SSH screens — strong structural pattern.
- Status bar at bottom (`mod.rs:23-51`) gives persistent context on current screen + YubiKey serial.
- Color-coded status indicators (green/yellow/red) for PIN retry counters with threshold logic (>1 = green, ==1 = yellow/DANGER, 0 = red/BLOCKED) — semantically clear.
- Factory reset title rendered in red BOLD (`pin.rs:471`) — appropriately alarming.
- Touch policy select uses `>` arrow + yellow BOLD for selected item, bare text for others — clear selection affordance without a cursor widget.
- Attestation popup uses an 80% width overlay to show PEM content inline.
- Progress popup (`progress.rs:13`) uses ASCII spinner characters `|`, `/`, `-`, `\` — appropriate for terminal; no Unicode dependency.

**Gaps:**
- `render_set_touch_policy_confirm` (`keys.rs:662`) renders as a plain full-area paragraph without preserving the previous screen underneath. The slot and policy summary are buried in a text string. A `render_main` background + popup overlay (as used in `PinScreen::OperationResult`) would keep users oriented about which slot they are about to modify.
- Dashboard's "All systems operational - Your YubiKey is ready to use!" (`dashboard.rs:82`) is unconditionally appended even when PIN is in a DANGER or BLOCKED state (the `format!` is inside the outer `if let Some(yk)` branch but does not check the state of `pin_status.is_healthy()`). This creates a false-positive "all clear" message while a warning emoji is shown two lines above it. Either gate this line on `pin_status.is_healthy()` or remove it.

### Pillar 3: Color (3/4)

**Color inventory across all UI files:**

| Color | Usage |
|-------|-------|
| Cyan | Screen titles, section headers (BOLD) |
| Green | Healthy status (PIN OK, keys present) |
| Yellow | Warnings, selected items, navigation labels |
| Red | Errors, blocked state, destructive warnings |
| Magenta | Secondary actions (e.g. Export in SSH list) |
| Blue | Tertiary actions (e.g. Test SSH in SSH list) |
| White | Default / unselected items |
| DarkGray | Hints, disabled/empty states, status bar background |

Seven named colors in use. In a TUI context this is acceptable — each color carries semantic meaning rather than decorative weight. The 60/30/10 principle adapts here to: White/default (background text ~60%), Green/Yellow/Red status indicators (~30%), Cyan titles (~10%).

**Strengths:**
- Color is used semantically and consistently: Cyan is always titles, Green is always healthy, Red is always danger/blocked. No color is used for decoration only.
- Action lists in SSH and PIN screens use distinct colors per action type, which gives visual rhythm to what would otherwise be a flat list.

**Gaps:**
- `ssh.rs:124` — Blue for "Test SSH connection" is the only use of `Color::Blue` across all screens. This singleton creates a visual outlier in an otherwise predictable palette. Change to Cyan (for informational/neutral actions) or White.
- Dashboard navigation list (`dashboard.rs:117-128`) only applies color to the active "[1] Dashboard" item; all other items are plain `ListItem::new(...)` with no explicit style. Other screens (SSH actions, PIN actions) style their list items with named colors. Inconsistent treatment makes the Dashboard navigation feel unpolished compared to other screens.

### Pillar 4: Typography (4/4)

In ratatui, "typography" maps to: text weight (via `Modifier::BOLD` / `Modifier::DIM` etc.) and explicit spans vs. plain text.

**Weights in use:**
- `Modifier::BOLD` — section titles, selected items, warning labels (42 occurrences across 7 files)
- Plain (no modifier) — body text, list items, hints
- `Modifier::DIM` — not used (appropriate, would conflict with Red/Yellow warning colors)

Two effective weight levels. BOLD is used exclusively for:
1. Screen-level titles (every render function)
2. Selected items in lists (touch policy, key list)
3. Destructive warning headers (factory reset, irreversible confirm)

This is disciplined use — BOLD is never applied to body text or hints, preserving its emphasis value. No typography issues found.

### Pillar 5: Spacing (3/4)

Layout is defined entirely via `ratatui::layout::Constraint` — `Length(N)`, `Min(N)`, `Percentage(N)`. No arbitrary pixel/rem values apply to a TUI.

**Common layout patterns:**

| Pattern | Usage |
|---------|-------|
| `Length(3)` | Title bars (all screens) |
| `Min(10)` / `Min(0)` | Content areas |
| `Length(10)` / `Length(14)` | Action lists |
| `Constraint::Percentage(...)` | Popup centering |

**Strengths:**
- Title bar height is `Length(3)` uniformly across all screens (dashboard.rs, pin.rs, ssh.rs, keys.rs).
- Status bar at app level is `Length(3)` — matches title bar size, bookending the content area symmetrically.
- `render_pin_input` calculates `height` dynamically from field count (`pin_input.rs:172`), so it adapts to 1-field and 3-field PIN forms without hardcoding.

**Gaps:**
- `popup.rs:centered_area` (lines 13-17) uses `Constraint::Percentage((100 - height.min(100)) / 2)` for vertical margins. The `height` value is a line count (e.g. 8), not a percentage. This makes the vertical margins ~46% each, much larger than intended, which pushes the popup to the center-ish but wastes vertical space and may cause layout engine confusion. The correct approach (used in `pin_input.rs:centered_area`) uses `Length(v_margin)`. This is the same bug reported in Priority Fix #2 above.
- `ssh.rs:64` — `Constraint::Length(14)` for the actions panel, while `pin.rs:136` uses `Length(10)` for its actions panel. The SSH actions list has more items (8 lines vs 6), so the size difference is justified, but the visual result is that SSH screens feel bottom-heavy compared to PIN screens.

### Pillar 6: Experience Design (4/4)

**Loading states:**
- `render_progress_popup` widget with 4-frame ASCII spinner — used in `PinScreen::OperationRunning` (`pin.rs:103-112`) and `KeyScreen::KeyImportRunning` / `KeyGenStep::Running` (`keys.rs:168, 709`).
- `progress_tick: usize` incremented by the event loop and passed through to the spinner index.
- Status message customizable per operation (`operation_status` field).

**Error states:**
- PIN input widget has `error_message: Option<String>` displayed in red below the hint line (`pin_input.rs:205-210`).
- Empty key list shows specific actionable error in red (`keys.rs:362-370`).
- Key attributes unavailable shown in yellow with a specific message (`keys.rs:512-516`).
- "No YubiKey detected. Press 'R' to refresh." present in Dashboard, PIN, and Keys screens.

**Empty states:**
- All three key slots show explicit `[empty]` when no key is loaded (`keys.rs:474-510`).
- Dashboard's no-YubiKey empty state includes troubleshooting steps.
- PIN unblock wizard shows available recovery paths dynamically based on actual retry counters — no paths shown if all counters are 0, with a clear "No recovery paths available — only factory reset remains" message (`pin.rs:394-397`).

**Destructive action confirmation:**
- Factory reset: Red BOLD title + warning lines + `render_confirm_dialog` overlay when `confirm_factory_reset` is true (`pin.rs:515-523`).
- Touch policy irreversible change: dedicated `SetTouchPolicyConfirm` screen + `'y'` confirmation key (`keys.rs:662-676`).
- Admin PIN required before touch policy write (`SetTouchPolicyPinInput` screen, from Plan 02).

**KDF edge case:**
- `set_touch_policy` includes KDF detection before PIN verify (`touch_policy.rs` per 05-02-SUMMARY) — produces a clear error message rather than a silent wrong-PIN failure. This is excellent UX for a non-obvious failure mode.

No significant experience design gaps found. Score reflects complete coverage of loading, error, empty, and destructive-action states.

---

## Files Audited

- `/Users/michael/code/yubitui/src/ui/dashboard.rs`
- `/Users/michael/code/yubitui/src/ui/keys.rs`
- `/Users/michael/code/yubitui/src/ui/pin.rs`
- `/Users/michael/code/yubitui/src/ui/ssh.rs`
- `/Users/michael/code/yubitui/src/ui/mod.rs`
- `/Users/michael/code/yubitui/src/ui/widgets/mod.rs`
- `/Users/michael/code/yubitui/src/ui/widgets/pin_input.rs`
- `/Users/michael/code/yubitui/src/ui/widgets/progress.rs`
- `/Users/michael/code/yubitui/src/ui/widgets/popup.rs`
- `/Users/michael/code/yubitui/src/app.rs` (partial — app init, render, screen routing)
- `/Users/michael/code/yubitui/.planning/phases/05-native-card-protocol/05-01-SUMMARY.md`
- `/Users/michael/code/yubitui/.planning/phases/05-native-card-protocol/05-02-SUMMARY.md`
- `/Users/michael/code/yubitui/.planning/phases/05-native-card-protocol/05-01-PLAN.md`
- `/Users/michael/code/yubitui/.planning/phases/05-native-card-protocol/05-02-PLAN.md`
- `/Users/michael/code/yubitui/.planning/phases/05-native-card-protocol/05-03-PLAN.md`
- `/Users/michael/code/yubitui/.planning/phases/05-native-card-protocol/05-CONTEXT.md`

Registry audit: shadcn not initialized — skipped.
