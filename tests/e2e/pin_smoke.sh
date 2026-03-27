#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
build_if_needed

SESSION="yubitui_e2e_pin_$$"
start_session "$SESSION" || exit 1

# Navigate to PIN Management screen (menu index 2: Enter to open menu, Down twice, Enter to select)
tmux send-keys -t "$SESSION" Enter
wait_for_text "$SESSION" "Diagnostics" 3 || { echo "FAIL: pin - context menu not visible"; cleanup "$SESSION"; exit 1; }
tmux send-keys -t "$SESSION" Down
sleep 0.1
tmux send-keys -t "$SESSION" Down
sleep 0.1
tmux send-keys -t "$SESSION" Enter

# Assert: PIN Management screen content visible
wait_for_text "$SESSION" "PIN Management" 5 || { echo "FAIL: pin screen not visible"; cleanup "$SESSION"; exit 1; }

# Navigate back (Esc returns to dashboard)
tmux send-keys -t "$SESSION" Escape
wait_for_text "$SESSION" "Navigation" 3 || { echo "FAIL: not back at dashboard"; cleanup "$SESSION"; exit 1; }

cleanup "$SESSION"
echo "PASS: pin_smoke"
