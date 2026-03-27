#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
build_if_needed

SESSION="yubitui_e2e_dashboard_$$"
start_session "$SESSION" || exit 1

# Assert: mock YubiKey info visible (model is YubiKey 5 NFC in mock data)
wait_for_text "$SESSION" "YubiKey" 5 || { echo "FAIL: dashboard - yubikey info not visible"; cleanup "$SESSION"; exit 1; }

# Open context menu (Enter key)
tmux send-keys -t "$SESSION" Enter
wait_for_text "$SESSION" "Diagnostics" 3 || { echo "FAIL: dashboard - context menu not visible"; cleanup "$SESSION"; exit 1; }

# Close menu (Esc)
tmux send-keys -t "$SESSION" Escape
sleep 0.3

cleanup "$SESSION"
echo "PASS: dashboard_smoke"
