#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
build_if_needed

SESSION="yubitui_e2e_diagnostics_$$"
start_session "$SESSION" || exit 1

# Navigate to Diagnostics screen (menu index 0: Enter to open menu, Enter to select first item)
tmux send-keys -t "$SESSION" Enter
wait_for_text "$SESSION" "Diagnostics" 3 || { echo "FAIL: diagnostics - context menu not visible"; cleanup "$SESSION"; exit 1; }
tmux send-keys -t "$SESSION" Enter

# Assert: Diagnostics screen content visible
wait_for_text "$SESSION" "System Diagnostics" 5 || { echo "FAIL: diagnostics screen not visible"; cleanup "$SESSION"; exit 1; }

# Navigate back (Esc returns to dashboard)
tmux send-keys -t "$SESSION" Escape
wait_for_text "$SESSION" "Navigation" 3 || { echo "FAIL: not back at dashboard"; cleanup "$SESSION"; exit 1; }

cleanup "$SESSION"
echo "PASS: diagnostics_smoke"
