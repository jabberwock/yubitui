#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
build_if_needed

SESSION="yubitui_e2e_keys_$$"
start_session "$SESSION" || exit 1

# Navigate to Keys screen (menu index 1: Enter to open menu, Down once, Enter to select)
tmux send-keys -t "$SESSION" Enter
wait_for_text "$SESSION" "Diagnostics" 3 || { echo "FAIL: keys - context menu not visible"; cleanup "$SESSION"; exit 1; }
tmux send-keys -t "$SESSION" Down
sleep 0.1
tmux send-keys -t "$SESSION" Enter

# Assert: Key Management screen content visible
wait_for_text "$SESSION" "Key Management" 5 || { echo "FAIL: keys screen not visible"; cleanup "$SESSION"; exit 1; }

# Navigate back (Esc returns to dashboard)
tmux send-keys -t "$SESSION" Escape
wait_for_text "$SESSION" "Navigation" 3 || { echo "FAIL: not back at dashboard"; cleanup "$SESSION"; exit 1; }

cleanup "$SESSION"
echo "PASS: keys_smoke"
