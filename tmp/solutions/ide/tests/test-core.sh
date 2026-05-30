#!/bin/bash
# ============================================================================
# Core Protocol Tests — session lifecycle, streaming, error handling
# ============================================================================
# Tests the fundamental ACP protocol mechanics that every IDE consumer needs.
# These should ALL pass for basic IDE integration to work.
# ============================================================================

source "$(dirname "$0")/lib.sh"

print_header "Core Protocol Tests"
print_env
check_prereqs

# --------------------------------------------------------------------------
# Test: session/new returns valid session
# --------------------------------------------------------------------------
print_section "Session Lifecycle"

if test_start "session/new returns sessionId"; then
  acp_start
  if acp_session_new "sonnet"; then
    test_pass "$ACP_SESSION_ID"
  else
    test_fail "no sessionId returned"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: session/prompt works (correct method name)
# --------------------------------------------------------------------------
if test_start "session/prompt streams response"; then
  acp_start
  acp_session_new "sonnet"
  acp_prompt "Say exactly: CORE_TEST_OK"
  if echo "$ACP_RESPONSE" | grep -q "CORE_TEST_OK"; then
    test_pass "got expected text"
  else
    test_fail "expected CORE_TEST_OK, got: $(echo "$ACP_RESPONSE" | head -c 80)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: stop reason is end_turn
# --------------------------------------------------------------------------
if test_start "stop reason is end_turn"; then
  acp_start
  acp_session_new "sonnet"
  acp_prompt "Say: hi"
  if [ "$ACP_STOP_REASON" = "end_turn" ]; then
    test_pass
  else
    test_fail "got: $ACP_STOP_REASON"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: multiple prompts in same session
# --------------------------------------------------------------------------
if test_start "multi-turn conversation"; then
  acp_start
  acp_session_new "sonnet"
  acp_prompt "Remember the word: BANANA" 30 2
  acp_prompt "What word did I ask you to remember?" 30 3
  if echo "$ACP_RESPONSE" | grep -qi "banana"; then
    test_pass "context preserved"
  else
    test_fail "no BANANA in response"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Error Handling
# --------------------------------------------------------------------------
print_section "Error Handling"

if test_start "wrong session ID → error -32000"; then
  acp_start
  acp_session_new "sonnet"
  acp_send '{"jsonrpc":"2.0","method":"session/prompt","id":99,"params":{"sessionId":"sess_FAKE","prompt":[{"type":"text","text":"hi"}]}}'
  if acp_read_until '"id":99' 10; then
    if echo "$ACP_LAST_LINE" | grep -q '"code":-32000'; then
      test_pass
    else
      test_fail "wrong error code: $(echo "$ACP_LAST_LINE" | head -c 100)"
    fi
  else
    test_fail "no response within 10s"
  fi
  acp_stop
fi

if test_start "unknown method → error -32601"; then
  acp_start
  acp_session_new "sonnet"
  acp_send '{"jsonrpc":"2.0","method":"message/send","id":100,"params":{}}'
  if acp_read_until '"id":100' 10; then
    if echo "$ACP_LAST_LINE" | grep -q '"code":-32601'; then
      test_pass
    else
      test_fail "wrong error code: $(echo "$ACP_LAST_LINE" | head -c 100)"
    fi
  else
    test_fail "no response within 10s"
  fi
  acp_stop
fi

if test_start "malformed JSON → graceful handling"; then
  acp_start
  acp_session_new "sonnet"
  acp_send 'this is not json'
  # Should not crash the process — verify we can still send valid messages
  sleep 0.5
  acp_prompt "Say: ALIVE" 15 101
  if echo "$ACP_RESPONSE" | grep -q "ALIVE"; then
    test_pass "process survived"
  else
    test_fail "process died or no response"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Disconnect Behavior
# --------------------------------------------------------------------------
print_section "Disconnect Behavior"

if test_start "clean exit on stdin close"; then
  acp_start
  acp_session_new "sonnet"
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":99,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"Say hi\"}]}}"
  sleep 0.3
  # Close write FD (stdin of process)
  exec 3>&- 2>/dev/null || true
  # Wait for process to notice and exit (3s max)
  for i in 1 2 3; do
    if ! kill -0 "$ACP_PID" 2>/dev/null; then break; fi
    sleep 1
  done
  exec 4<&- 2>/dev/null || true
  if kill -0 "$ACP_PID" 2>/dev/null; then
    test_fail "process still alive after 3s"
    kill "$ACP_PID" 2>/dev/null
  else
    test_pass "process exited"
  fi
  wait "$ACP_PID" 2>/dev/null || true
  rm -f "$ACP_FIFO_IN" "$ACP_FIFO_OUT" 2>/dev/null
fi

# --------------------------------------------------------------------------
print_summary

# Save results for run-all
echo "$TESTS_PASSED $TESTS_FAILED $TESTS_SKIPPED" > "$LOG_DIR/core-results.txt"
exit $TESTS_FAILED
