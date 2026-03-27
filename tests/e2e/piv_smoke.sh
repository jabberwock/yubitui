#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
build_if_needed

SESSION="yubitui_e2e_piv_$$"
start_session "$SESSION" || exit 1

# Navigate to PIV Certificates (menu index 4: Enter to open menu, Down four times, Enter to select)
tmux send-keys -t "$SESSION" Enter
wait_for_text "$SESSION" "Diagnostics" 3 || { echo "FAIL: piv - context menu not visible"; cleanup "$SESSION"; exit 1; }
tmux send-keys -t "$SESSION" Down
sleep 0.1
tmux send-keys -t "$SESSION" Down
sleep 0.1
tmux send-keys -t "$SESSION" Down
sleep 0.1
tmux send-keys -t "$SESSION" Down
sleep 0.1
tmux send-keys -t "$SESSION" Enter

# Assert: PIV Certificates screen content visible
wait_for_text "$SESSION" "PIV" 5 || { echo "FAIL: piv screen not visible"; cleanup "$SESSION"; exit 1; }

# Navigate back (Esc returns to dashboard)
tmux send-keys -t "$SESSION" Escape
wait_for_text "$SESSION" "Navigation" 3 || { echo "FAIL: not back at dashboard"; cleanup "$SESSION"; exit 1; }

cleanup "$SESSION"
echo "PASS: piv_smoke"
