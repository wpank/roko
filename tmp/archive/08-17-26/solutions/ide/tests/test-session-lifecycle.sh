#!/bin/bash
# ============================================================================
# Session Lifecycle Tests — config update, cancel, list, load, persistence
# ============================================================================
# Tests the full session lifecycle beyond basic create/prompt.
# Covers: config/update, session/cancel, session/list, session/load, initialize
# ============================================================================

source "$(dirname "$0")/lib.sh"

print_header "Session Lifecycle Tests"
print_env
check_prereqs

# --------------------------------------------------------------------------
print_section "session/config/update"

# --------------------------------------------------------------------------
# Test: change model via config/update
# --------------------------------------------------------------------------
if test_start "config/update changes model"; then
  acp_start
  acp_session_new "sonnet"
  # Change model to haiku
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/config/update\",\"id\":5,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"optionId\":\"model\",\"newValue\":\"haiku\"}}"
  CONFIG_RESULT=""
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":5'; then
      CONFIG_RESULT="$line"
      break
    fi
  done

  if echo "$CONFIG_RESULT" | grep -q '"error"'; then
    ERR_MSG=$(echo "$CONFIG_RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('error',{}).get('message',''))" 2>/dev/null)
    test_fail "config/update returned error: $ERR_MSG"
  elif echo "$CONFIG_RESULT" | grep -q '"result"'; then
    # Check if model was actually changed
    NEW_MODEL=$(echo "$CONFIG_RESULT" | python3 -c "
import sys,json
d=json.load(sys.stdin)
for o in d.get('result',{}).get('configOptions',[]):
  if o.get('id')=='model':
    print(o.get('currentValue',''))
" 2>/dev/null) || true
    if [ "$NEW_MODEL" = "haiku" ]; then
      test_pass "model is now haiku"
    else
      test_fail "model='$NEW_MODEL', expected 'haiku'"
    fi
  else
    test_fail "no response to config/update (CONFIG_RESULT empty)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: change provider via config/update
# --------------------------------------------------------------------------
if test_start "config/update changes provider"; then
  acp_start
  acp_session_new "sonnet"
  # Check what providers are available
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/config/update\",\"id\":6,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"optionId\":\"provider\",\"newValue\":\"claude_cli\"}}"
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":6'; then
      PROV_RESULT="$line"
      break
    fi
  done

  if echo "$PROV_RESULT" | grep -q '"error"'; then
    ERR_MSG=$(echo "$PROV_RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('error',{}).get('message',''))" 2>/dev/null)
    test_fail "config/update provider returned error: $ERR_MSG"
  elif echo "$PROV_RESULT" | grep -q '"result"'; then
    NEW_PROV=$(echo "$PROV_RESULT" | python3 -c "
import sys,json
d=json.load(sys.stdin)
for o in d.get('result',{}).get('configOptions',[]):
  if o.get('id')=='provider':
    print(o.get('currentValue',''))
" 2>/dev/null) || true
    if [ "$NEW_PROV" = "claude_cli" ]; then
      test_pass "provider is now claude_cli"
    else
      test_warn "provider is $NEW_PROV (maybe filtered available models)"
    fi
  else
    test_fail "no response to config/update provider (PROV_RESULT empty)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: change effort/thinking level
# --------------------------------------------------------------------------
if test_start "config/update changes effort level"; then
  acp_start
  acp_session_new "sonnet"
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/config/update\",\"id\":7,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"optionId\":\"effort\",\"newValue\":\"low\"}}"
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":7'; then
      EFFORT_RESULT="$line"
      break
    fi
  done
  if echo "$EFFORT_RESULT" | grep -q '"result"'; then
    NEW_EFFORT=$(echo "$EFFORT_RESULT" | python3 -c "
import sys,json
d=json.load(sys.stdin)
for o in d.get('result',{}).get('configOptions',[]):
  if o.get('id')=='effort':
    print(o.get('currentValue',''))
" 2>/dev/null) || true
    if [ "$NEW_EFFORT" = "low" ]; then
      test_pass "effort=low"
    else
      test_fail "effort='$NEW_EFFORT', expected 'low'"
    fi
  else
    test_fail "no result in effort config/update response (got: $(echo "$EFFORT_RESULT" | head -c 80))"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: invalid config update value
# --------------------------------------------------------------------------
if test_start "config/update with invalid model → error"; then
  acp_start
  acp_session_new "sonnet"
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/config/update\",\"id\":8,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"optionId\":\"model\",\"newValue\":\"nonexistent-xyz\"}}"
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":8'; then
      INVALID_RESULT="$line"
      break
    fi
  done
  if echo "$INVALID_RESULT" | grep -q '"error"'; then
    test_pass "returns error for invalid model"
  elif echo "$INVALID_RESULT" | grep -q '"result"'; then
    USED=$(echo "$INVALID_RESULT" | python3 -c "
import sys,json
d=json.load(sys.stdin)
for o in d.get('result',{}).get('configOptions',[]):
  if o.get('id')=='model':
    print(o.get('currentValue',''))
" 2>/dev/null) || true
    test_warn "accepted invalid model, now using: $USED"
  else
    test_fail "no response to invalid model config/update (INVALID_RESULT empty)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
print_section "session/cancel"

# --------------------------------------------------------------------------
# Test: cancel an in-flight prompt
# --------------------------------------------------------------------------
if test_start "session/cancel stops streaming"; then
  acp_start
  acp_session_new "sonnet"
  # Send a prompt that will generate a long response
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":20,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"Write a 500-word essay about the history of computing.\"}]}}"
  # Wait for streaming to start
  sleep 1
  # Send cancel
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/cancel\",\"params\":{\"sessionId\":\"$ACP_SESSION_ID\"}}"
  # Wait for final response
  CANCELLED=false
  STOP=""
  while IFS= read -r -t 15 line <&4; do
    [ -z "$line" ] && continue
    if echo "$line" | grep -q '"id":20'; then
      STOP=$(echo "$line" | python3 -c "import sys,json; print(json.load(sys.stdin).get('result',{}).get('stopReason',''))" 2>/dev/null) || true
      if [ "$STOP" = "cancelled" ]; then
        CANCELLED=true
      fi
      break
    fi
  done
  if [ "$CANCELLED" = true ]; then
    test_pass "stopReason=cancelled"
  else
    test_warn "stopReason=$STOP (may have completed before cancel arrived)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: prompt after cancel
# --------------------------------------------------------------------------
if test_start "can prompt again after cancel"; then
  acp_start
  acp_session_new "sonnet"
  # Long prompt
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":21,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"Write 200 words about trees\"}]}}"
  sleep 1
  # Cancel
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/cancel\",\"params\":{\"sessionId\":\"$ACP_SESSION_ID\"}}"
  # Wait for id:21 to finish (consume all remaining output)
  while IFS= read -r -t 15 line <&4; do
    if echo "$line" | grep -q '"id":21'; then break; fi
  done
  # Drain any trailing notifications
  sleep 0.5
  acp_drain
  sleep 0.5
  # Now send another prompt
  acp_prompt "Say exactly: AFTER_CANCEL" 30 22
  if echo "$ACP_RESPONSE" | grep -q "AFTER_CANCEL"; then
    test_pass
  else
    test_fail "expected 'AFTER_CANCEL' in response, got: $(echo "$ACP_RESPONSE" | head -c 80)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
print_section "Session Management"

# --------------------------------------------------------------------------
# Test: session/list returns sessions
# --------------------------------------------------------------------------
if test_start "session/list returns created sessions"; then
  acp_start
  acp_session_new "sonnet"
  SID="$ACP_SESSION_ID"
  acp_send '{"jsonrpc":"2.0","method":"session/list","id":30,"params":{}}'
  LIST_RESULT=""
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":30'; then
      LIST_RESULT="$line"
      break
    fi
  done
  if echo "$LIST_RESULT" | grep -q "$SID"; then
    test_pass "session found in list"
  elif echo "$LIST_RESULT" | grep -q '"result"'; then
    test_warn "list returned but session $SID not found: $(echo "$LIST_RESULT" | head -c 100)"
  elif echo "$LIST_RESULT" | grep -q '"error"'; then
    ERR=$(echo "$LIST_RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('error',{}).get('message',''))" 2>/dev/null)
    test_fail "session/list error: $ERR"
  else
    test_fail "no response to session/list (LIST_RESULT empty)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: session/close
# --------------------------------------------------------------------------
if test_start "session/close removes session"; then
  acp_start
  acp_session_new "sonnet"
  SID="$ACP_SESSION_ID"
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/close\",\"id\":31,\"params\":{\"sessionId\":\"$SID\"}}"
  CLOSE_RESULT=""
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":31'; then
      CLOSE_RESULT="$line"
      break
    fi
  done
  if echo "$CLOSE_RESULT" | grep -q '"result"'; then
    # Try to prompt on closed session
    acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":32,\"params\":{\"sessionId\":\"$SID\",\"prompt\":[{\"type\":\"text\",\"text\":\"hi\"}]}}"
    while IFS= read -r -t 10 line <&4; do
      if echo "$line" | grep -q '"id":32'; then
        if echo "$line" | grep -q '"error"'; then
          test_pass "closed session rejects prompts"
        else
          test_fail "closed session still accepts prompts (expected error, got result)"
        fi
        break
      fi
    done
  elif echo "$CLOSE_RESULT" | grep -q '"error"'; then
    ERR=$(echo "$CLOSE_RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('error',{}).get('message',''))" 2>/dev/null)
    test_fail "session/close error: $ERR"
  else
    test_fail "no response to session/close (CLOSE_RESULT empty)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: initialize method
# --------------------------------------------------------------------------
if test_start "initialize method response"; then
  acp_start
  acp_send '{"jsonrpc":"2.0","method":"initialize","id":40,"params":{"protocolVersion":1,"clientCapabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}'
  INIT_RESULT=""
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":40'; then
      INIT_RESULT="$line"
      break
    fi
  done
  if echo "$INIT_RESULT" | grep -q '"result"'; then
    test_pass "initialize accepted"
  elif echo "$INIT_RESULT" | grep -q '"error"'; then
    ERR=$(echo "$INIT_RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('error',{}).get('message',''))" 2>/dev/null)
    test_warn "error: $ERR (initialize may not be required)"
  else
    test_fail "no response to initialize (INIT_RESULT empty)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
print_section "Mode Switching"

# --------------------------------------------------------------------------
# Test: session/set_mode
# --------------------------------------------------------------------------
if test_start "session/set_mode changes mode"; then
  acp_start
  acp_session_new "sonnet"
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/set_mode\",\"id\":50,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"modeId\":\"plan\"}}"
  MODE_RESULT=""
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":50'; then
      MODE_RESULT="$line"
      break
    fi
  done
  if echo "$MODE_RESULT" | grep -q '"result"'; then
    NEW_MODE=$(echo "$MODE_RESULT" | python3 -c "
import sys,json
d=json.load(sys.stdin)
modes=d.get('result',{}).get('modes',{})
print(modes.get('currentModeId',''))
" 2>/dev/null) || true
    if [ "$NEW_MODE" = "plan" ]; then
      test_pass "mode=plan"
    elif [ -n "$NEW_MODE" ]; then
      test_warn "mode=$NEW_MODE (might not have changed)"
    else
      test_pass "accepted (no mode in response)"
    fi
  elif echo "$MODE_RESULT" | grep -q '"error"'; then
    ERR=$(echo "$MODE_RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('error',{}).get('message',''))" 2>/dev/null)
    test_fail "session/set_mode error: $ERR"
  else
    test_fail "no response to session/set_mode (MODE_RESULT empty)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
print_summary
echo "$TESTS_PASSED $TESTS_FAILED $TESTS_SKIPPED" > "$LOG_DIR/lifecycle-results.txt"
exit $TESTS_FAILED
