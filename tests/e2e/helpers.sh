#!/usr/bin/env bash
# Shared E2E test helpers for yubitui tmux-based tests

# Binary path — can be overridden by run_all.sh
BINARY="${BINARY:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/target/debug/yubitui}"

# Build if needed (binary missing or source newer)
build_if_needed() {
    local project_root
    project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
    if [ ! -f "$BINARY" ] || [ "$(find "$project_root/src/" -newer "$BINARY" -name '*.rs' 2>/dev/null | head -1)" ]; then
        echo "Building yubitui..."
        (cd "$project_root" && cargo build --quiet 2>&1) || { echo "FAIL: cargo build failed"; exit 1; }
    fi
}

# Start app in a tmux session
# Usage: start_session SESSION_NAME
start_session() {
    local session="$1"
    tmux new-session -d -s "$session" -x 120 -y 40 "$BINARY --mock"
    # Use wait_for_text instead of fixed sleep for startup
    wait_for_text "$session" "Dashboard" 10 || {
        echo "FAIL: App did not start (Dashboard not visible within 10s)"
        cleanup "$session"
        return 1
    }
}

# Capture current pane content
# Usage: capture SESSION_NAME
capture() {
    tmux capture-pane -t "$1" -p
}

# Wait for text to appear in tmux pane with retry loop.
# Polls every 0.3s until text is found or timeout expires.
# Usage: wait_for_text SESSION_NAME "expected text" TIMEOUT_SECONDS
# Returns: 0 if found, 1 if timeout
wait_for_text() {
    local session="$1"
    local expected="$2"
    local timeout="${3:-5}"
    local elapsed=0
    local interval="0.3"

    while [ "$(echo "$elapsed < $timeout" | bc -l)" = "1" ]; do
        local output
        output=$(capture "$session" 2>/dev/null)
        if echo "$output" | grep -q "$expected"; then
            return 0
        fi
        sleep "$interval"
        elapsed=$(echo "$elapsed + $interval" | bc -l)
    done

    # Final attempt with diagnostic output on failure
    local output
    output=$(capture "$session" 2>/dev/null)
    if echo "$output" | grep -q "$expected"; then
        return 0
    fi

    echo "TIMEOUT: '$expected' not found after ${timeout}s"
    echo "--- Captured output ---"
    echo "$output"
    echo "--- End output ---"
    return 1
}

# Assert text exists in captured output (immediate, no retry).
# Prefer wait_for_text for post-navigation checks.
# Usage: assert_contains SESSION_NAME "expected text" "test description"
assert_contains() {
    local session="$1"
    local expected="$2"
    local desc="$3"
    local output
    output=$(capture "$session")
    if ! echo "$output" | grep -q "$expected"; then
        echo "FAIL: $desc - '$expected' not found in output"
        echo "--- Captured output ---"
        echo "$output"
        echo "--- End output ---"
        cleanup "$session"
        return 1
    fi
}

# Cleanup tmux session
cleanup() {
    local session="$1"
    tmux send-keys -t "$session" q 2>/dev/null || true
    sleep 0.3
    tmux kill-session -t "$session" 2>/dev/null || true
}
