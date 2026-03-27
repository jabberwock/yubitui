#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
build_if_needed

SESSION="yubitui_e2e_ssh_$$"
start_session "$SESSION" || exit 1

# Navigate to SSH Setup Wizard (menu index 3: Enter to open menu, Down three times, Enter to select)
tmux send-keys -t "$SESSION" Enter
wait_for_text "$SESSION" "Diagnostics" 3 || { echo "FAIL: ssh - context menu not visible"; cleanup "$SESSION"; exit 1; }
tmux send-keys -t "$SESSION" Down
sleep 0.1
tmux send-keys -t "$SESSION" Down
sleep 0.1
tmux send-keys -t "$SESSION" Down
sleep 0.1
tmux send-keys -t "$SESSION" Enter

# Assert: SSH Setup Wizard screen content visible
wait_for_text "$SESSION" "SSH" 5 || { echo "FAIL: ssh screen not visible"; cleanup "$SESSION"; exit 1; }

# Navigate back (Esc returns to dashboard)
tmux send-keys -t "$SESSION" Escape
wait_for_text "$SESSION" "Navigation" 3 || { echo "FAIL: not back at dashboard"; cleanup "$SESSION"; exit 1; }

cleanup "$SESSION"
echo "PASS: ssh_smoke"
