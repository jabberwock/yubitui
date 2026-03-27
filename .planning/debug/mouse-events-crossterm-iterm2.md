---
status: awaiting_human_verify

trigger: "Mouse events are inconsistent across screens in a crossterm 0.28 TUI app running in iTerm2 3.6.9. Dashboard clicks work, but on other screens the hit targets are wrong/unresponsive."
created: 2026-03-26T00:00:00Z
updated: 2026-03-26T03:00:00Z
---

## Current Focus
<!-- OVERWRITE on each update - reflects NOW -->

hypothesis: CONFIRMED (sixth round) — Two compounding issues:
  1. STRUCTURAL: click_regions.clear() is inside each per-screen render function, not in app.rs. The app loop relies on each screen remembering to call clear(). This is fragile — any new screen or sub-screen that forgets will accumulate stale regions.
  2. pin.rs sub-screens: all 8 sub-screens except Main, OperationRunning, OperationResult have NO swallow region — only the full-area NavigateTo(Dashboard) back region is active. Clicking anywhere on ChangeUserPin, ChangeAdminPin, SetResetCode, UnblockUserPin, all 4 UnblockWizard screens, and PinInputActive navigates back to Dashboard.
test: (1) Move click_regions.clear() into app.rs render() before the screen match — guaranteed structural clear every frame; (2) Add swallow closure to pin.rs for all missing sub-screen arms
expecting: Pin sub-screens no longer bounce to Dashboard on click; structural invariant enforced at app level not per-screen level
next_action: human verification — test all pin sub-screens (no click should bounce to Dashboard), OperationRunning/OperationResult (clicks swallowed), and all previously fixed screens

## Symptoms
<!-- Written during gathering, then IMMUTABLE -->

expected: Mouse click events work consistently and accurately across all screens/views
actual: Dashboard click targets work correctly. On other screens, click targets are misaligned or unresponsive — "no idea where to click, some things are responsive and others aren't"
errors: No explicit error messages; EnableMouseCapture succeeds
reproduction: Navigate from dashboard to any other screen (e.g., key management, SSH, diagnostics) and try clicking UI elements
started: Ongoing issue; EnableMouseCapture confirmed working
environment: iTerm2 3.6.9, crossterm 0.28, Rust TUI app (ratatui)

## Eliminated
<!-- APPEND only - prevents re-investigating -->

- hypothesis: crossterm mouse capture not working on non-dashboard screens
  evidence: click_regions.clear() is called in every render function, confirming capture is active; the problem is which regions are registered after clearing
  timestamp: 2026-03-26

- hypothesis: coordinate mapping between crossterm and ratatui is wrong on non-dashboard screens
  evidence: all screens use the same Rect passed from app.rs chunks[0]; the coordinate system is identical
  timestamp: 2026-03-26

- hypothesis: status bar or layout offset is causing coordinate shift on non-dashboard screens
  evidence: layout is identical for all screens (chunks[0] = main, chunks[1] = status bar 3 rows); no per-screen layout difference
  timestamp: 2026-03-26

## Evidence
<!-- APPEND only - facts discovered -->

- timestamp: 2026-03-26
  checked: dashboard.rs render() click_regions registration
  found: dashboard.clear()s then pushes per-row ClickRegion for each nav menu item (6 navigation rows + refresh/menu rows). Each region is exactly 1 row tall mapped to actual rendered pixel rows.
  implication: dashboard works correctly because click targets match rendered rows exactly

- timestamp: 2026-03-26
  checked: pin.rs render() click_regions registration (lines 264-316)
  found: After clear(), dispatches to sub-render functions that register NO click regions, then pushes ONE region covering `area` (entire screen) mapped to NavigateTo(Dashboard). Result: the entire PIN screen is a single "back" button — any click goes to dashboard, no action items are clickable.
  implication: PIN screen has no clickable action buttons

- timestamp: 2026-03-26
  checked: keys.rs render() click_regions registration (lines 706-737)
  found: After clear(), dispatches to sub-renders. Only render_main() registers action-specific click regions (5 action rows, lines 923-930). Then after the match block, pushes a full-area NavigateTo(Dashboard) region. Since handle_mouse_event() uses .iter().rev() (last-pushed = highest priority), the full-area "back" region is checked FIRST — it matches every click before any action-specific region is checked.
  implication: On keys main screen, every click navigates to dashboard because the full-area region is last-pushed and thus checked first in reverse iteration

- timestamp: 2026-03-26
  checked: ssh.rs render() click_regions registration (lines 133-150, 229-250)
  found: render_main() registers 5 action rows. Then outer render() appends full-area NavigateTo(Dashboard). Same last-in-first-win bug as keys.rs: the full-area back region is pushed last, checked first via .iter().rev(), swallows all clicks.
  implication: SSH screen action buttons are unreachable by mouse

- timestamp: 2026-03-26
  checked: diagnostics.rs click_regions (line 148)
  found: Registers only chunks[0] (the title bar rect, ~3 rows tall) as back target. Content area has zero click regions.
  implication: Only clicking the title bar navigates back; rest of screen is dead to mouse

- timestamp: 2026-03-26
  checked: piv.rs click_regions (line 106)
  found: Same pattern as diagnostics — only chunks[0] (title) registered as back target
  implication: Only title area is mouse-sensitive

- timestamp: 2026-03-26
  checked: keys.rs render() match arm for SshPubkeyPopup; render_ssh_pubkey_popup()
  found: render_ssh_pubkey_popup() registers zero click regions. The outer render() pushes a full-area NavigateTo(Dashboard) region first (line 711-716), then calls render_ssh_pubkey_popup() which adds nothing. Under iter().rev() (last-in-first-win), the full-area back region is the sole active region — every click including inside the popup fires NavigateTo(Dashboard), dismissing the popup and navigating to dashboard.
  implication: Root cause of the Export SSH popup regression. Fix: push a full-area KeyAction::None region in the SshPubkeyPopup match arm AFTER the render call, so it takes priority over NavigateTo(Dashboard) and swallows all clicks as no-ops (allowing terminal native text selection to work).

- timestamp: 2026-03-26
  checked: help.rs click_regions (line 189)
  found: Full-area region registered with HelpAction::Close. Acceptable since the whole help screen is "click anywhere to close".
  implication: Help screen works correctly by intent

- timestamp: 2026-03-26
  checked: KeyAction::None approach confirmed broken by user
  found: EnableMouseCapture itself blocks terminal native text selection — iTerm2 shows "mouse reporting has prevented making a selection" banner; universal across all terminals, not iTerm2-specific
  implication: Cannot use no-op click regions as a workaround; must toggle mouse capture at the crossterm level

- timestamp: 2026-03-26
  checked: Option 3 implementation — toggle mouse capture off for SshPubkeyPopup
  found: Added mouse_capture_enabled: bool to App struct; added apply_mouse_capture() called in event_loop after each handle_events(); derives want_capture from current_screen==Keys && key_state.screen==SshPubkeyPopup; handles Event::Resize by re-applying current flag; removed KeyAction::None region from SshPubkeyPopup arm in keys.rs; 109/109 tests pass
  implication: Mouse capture is cleanly toggled per-screen-state with no action enum pollution or AppState changes

- timestamp: 2026-03-26
  checked: app.rs handle_mouse_event() rev() iteration (lines 163-166)
  found: Uses iter().rev() — rightfully documented as "last-in-first-win" for popups. BUT the outer render() functions for keys/ssh push the full-area back region AFTER the action-specific regions from render_main(). This means the full-area region is always checked first, consuming every click.
  implication: The fix for keys/ssh/pin is to push the full-area back region BEFORE the specific action regions (or remove it and handle "unmatched click = no-op" instead)

- timestamp: 2026-03-26
  checked: where click_regions.clear() is called and the event loop ordering in app.rs
  found: (1) clear() is called at the top of each top-level render function (dashboard, keys, ssh, pin, piv, diagnostics, help) — NOT in app.rs before delegating. app.rs render() uses std::mem::take to extract the Vec, passes it to the screen render fn, then puts it back. (2) Event loop: render → poll_import_task → handle_events → apply_mouse_capture. So regions are always rebuilt before events are processed. The per-screen-clear is structurally sound but fragile: any future screen that forgets clear() will accumulate stale regions.
  implication: Structural fix: call click_regions.clear() in app.rs render() at line 114 after the mem::take. This makes clearing an architectural invariant instead of a per-screen responsibility.

- timestamp: 2026-03-26
  checked: pin.rs render() match arms for all sub-screens
  found: ChangeUserPin, ChangeAdminPin, SetResetCode, UnblockUserPin, UnblockWizardCheck, UnblockWizardWithReset, UnblockWizardWithAdmin, UnblockWizardFactoryReset, PinInputActive — none push a swallow region. Only the full-area NavigateTo(Dashboard) region (pushed before the match) is active for all 9 sub-screens. OperationRunning and OperationResult correctly call render_main() which pushes action-row regions.
  implication: Clicking anywhere on any of these 9 sub-screens navigates back to Dashboard. This is the direct cause of "sub-screens lingering" — the user arrives at a sub-screen, clicks to interact, and is immediately bounced to Dashboard.

- timestamp: 2026-03-26
  checked: keys.rs execute_key_action() vs handle_key() for V/E/K actions (second round)
  found: ExecuteViewStatus → execute_key_operation() checks key_state.screen to decide what to do; on Main it falls to _ => and resets screen to Main — no visible effect. Keyboard 'v' sets screen=ViewStatus first, then returns None (no immediate operation). Mouse click action was calling execute_key_operation() without first setting the screen. Same pattern for ExecuteExportSSH and LoadKeyAttributes (which also needs to set screen=KeyAttributes before loading).
  implication: Fix: ExecuteViewStatus and ExecuteExportSSH should only set key_state.screen to sub-screen (not call execute_key_operation); LoadKeyAttributes should set key_state.screen=KeyAttributes before loading data.

- timestamp: 2026-03-26
  checked: ssh.rs click regions in render_main() and SshAction enum
  found: All 5 action rows mapped to NavigateTo(Screen::SshWizard). navigate_to(SshWizard) sets current_screen=SshWizard and refreshes status — same screen user is already on. ssh_state.screen stays at SshScreen::Main. No sub-screen is ever shown by mouse click.
  implication: Fix: add GoToEnableSSH/GoToConfigureShell/GoToRestartAgent/GoToExportKey/GoToTestConnection variants to SshAction; handle them in execute_ssh_action by setting self.ssh_state.screen.

- timestamp: 2026-03-26
  checked: ssh.rs render() match arms for all sub-screens; keys.rs render() match arms for all sub-screens
  found: All sub-screen render functions for both ssh.rs and keys.rs lack click_regions parameters and register zero click regions. The full-area NavigateTo(Dashboard) region pushed before the match arm is the sole region on every sub-screen, causing every click to navigate to dashboard. ssh.rs sub-screens: EnableSSH, ConfigureShell, RestartAgent, ExportKey have a "Press ENTER to execute" affordance — a full-area ExecuteSshOperation click region was added. TestConnection has text input fields — a full-area SshAction::None no-op was added. keys.rs sub-screens: all are keyboard-driven (list navigation, PIN input, wizard steps) with no simple click-to-execute affordance — a full-area KeyAction::None swallow was added via a shared closure for all 11 sub-screen arms.
  implication: Fix applied. 109/109 tests pass.

## Resolution
<!-- OVERWRITE as understanding evolves -->

root_cause: Seven compounding bugs across all non-dashboard screens:
  1. keys.rs and ssh.rs: full-area NavigateTo(Dashboard) region was pushed AFTER action-specific regions. handle_mouse_event uses iter().rev() (last-pushed = highest priority), so it swallowed every click. (Fixed in round 1.)
  2. pin.rs: no action-specific click regions registered at all. (Fixed in round 1.)
  3. diagnostics.rs and piv.rs: click target was only the title bar rect. (Fixed in round 1.)
  4. keys.rs (round 2): ExecuteViewStatus/ExecuteExportSSH mouse actions called execute_key_operation() without first setting sub-screen state; ssh.rs mouse actions used placeholder NavigateTo(SshWizard) no-op instead of real GoTo* sub-screen setters. (Fixed in round 2.)
  5. keys.rs SshPubkeyPopup (round 3): EnableMouseCapture itself blocks terminal native text selection — fixed by toggling mouse capture off while popup is visible. (Fixed in round 3/4.)
  6. ssh.rs and keys.rs all sub-screens (round 5): sub-screen render functions have no click_regions parameter and register zero regions. The full-area NavigateTo(Dashboard) region (only region present) captured every click on every sub-screen. Fix: add full-area ExecuteSshOperation region for SSH operation sub-screens (EnableSSH/ConfigureShell/RestartAgent/ExportKey); full-area SshAction::None no-op for TestConnection; full-area KeyAction::None swallow for all 11 keys.rs sub-screens.
  7. (round 6 — structural + pin) Two issues: (a) click_regions.clear() was inside per-screen render functions rather than guaranteed by app.rs — any new screen forgetting to call it would accumulate stale regions. Fixed by adding click_regions.clear() in app.rs render() after the std::mem::take, making it an architectural invariant. (b) pin.rs: all 9 sub-screens (ChangeUserPin, ChangeAdminPin, SetResetCode, UnblockUserPin, UnblockWizardCheck, UnblockWizardWithReset, UnblockWizardWithAdmin, UnblockWizardFactoryReset, PinInputActive) had no swallow region — any click bounced to Dashboard. OperationRunning/OperationResult called render_main without a swallow, making action rows clickable through the progress/result popup. Fixed by adding a swallow closure to pin.rs render() applied to all 11 sub-screen arms.

fix:
  - Round 1: ordering of click_regions (back-first), pin per-row regions, diagnostics/piv full-area
  - Round 2 keys.rs: ExecuteViewStatus/ExecuteExportSSH → set sub-screen state only; LoadKeyAttributes → set screen=KeyAttributes then load
  - Round 2 ssh.rs: added GoTo* variants to SshAction enum; fixed render_main() click regions; added handlers in execute_ssh_action()
  - Round 3/4 keys.rs SshPubkeyPopup: mouse capture toggle (apply_mouse_capture() in App)
  - Round 5 ssh.rs: full-area ExecuteSshOperation for operation sub-screens; full-area None for TestConnection
  - Round 5 keys.rs: full-area KeyAction::None swallow (shared closure) for all 11 sub-screen arms
  - Round 6 structural: click_regions.clear() moved to app.rs render() — guaranteed every frame
  - Round 6 pin.rs: swallow closure added; all 11 sub-screen arms (including OperationRunning/OperationResult) now swallow clicks

verification: cargo test — 109/109 passing
files_changed:
  - src/tui/keys.rs
  - src/tui/ssh.rs
  - src/tui/pin.rs
  - src/tui/diagnostics.rs
  - src/tui/piv.rs
  - src/app.rs
