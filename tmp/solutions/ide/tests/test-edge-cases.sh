#!/bin/bash
# ============================================================================
# Edge Case Tests — concurrency, config variations, boundary conditions
# ============================================================================
# Tests unusual but valid usage patterns and boundary conditions.
# ============================================================================

source "$(dirname "$0")/lib.sh"

print_header "Edge Case Tests"
print_env
check_prereqs

# --------------------------------------------------------------------------
print_section "Concurrent Sessions"

# --------------------------------------------------------------------------
# Test: multiple sessions on same ACP process
# --------------------------------------------------------------------------
if test_start "create 2 sessions on same process"; then
  acp_start
  acp_session_new "sonnet"
  SID1="$ACP_SESSION_ID"
  # Create second session (need different request ID approach)
  acp_send '{"jsonrpc":"2.0","method":"session/new","id":10,"params":{"model":"haiku"}}'
  SID2=""
  while IFS= read -r -t 10 line <&4; do
    S=$(echo "$line" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('result',{}).get('sessionId',''))" 2>/dev/null) || true
    if [ -n "$S" ] && echo "$S" | grep -q "sess_"; then
      SID2="$S"
      break
    fi
  done
  acp_drain

  if [ -n "$SID1" ] && [ -n "$SID2" ] && [ "$SID1" != "$SID2" ]; then
    test_pass "sid1=$SID1, sid2=$SID2"
  else
    test_fail "failed to create distinct sessions (sid1=${SID1:-empty}, sid2=${SID2:-empty})"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: prompt session 1 after session 2 created
# --------------------------------------------------------------------------
if test_start "session 1 still works after session 2 created"; then
  acp_start
  acp_session_new "sonnet"
  SID1="$ACP_SESSION_ID"
  acp_send '{"jsonrpc":"2.0","method":"session/new","id":10,"params":{"model":"sonnet"}}'
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":10'; then break; fi
  done
  acp_drain

  # Prompt on session 1
  ACP_SESSION_ID="$SID1"
  acp_prompt "Say exactly: SESSION_1_WORKS" 30 20
  if echo "$ACP_RESPONSE" | grep -q "SESSION_1_WORKS"; then
    test_pass
  else
    test_fail "session 1 prompt after session 2 creation returned unexpected response: $(echo "$ACP_RESPONSE" | head -c 60)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
print_section "Config Variations"

# --------------------------------------------------------------------------
# Test: minimal valid config
# --------------------------------------------------------------------------
if test_start "minimal config (one provider, one model)"; then
  TMP_CFG=$(mktemp /tmp/roko_min_XXXXXX.toml)
  cat > "$TMP_CFG" << EOF
config_version = 2
schema_version = 2
[project]
name = "minimal"
[serve]
port = 6699
[agent]
command = "cat"
model = "default"
bare_mode = true
timeout_ms = 60000
[providers.openai]
kind = "openai_compat"
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
[models.default]
provider = "openai"
slug = "gpt-4o-mini"
supports_tools = true
context_window = 128000
max_output = 4096
EOF

  acp_start "$TMP_CFG"
  acp_session_new "default"
  acp_prompt "Say: MINIMAL_OK"
  if echo "$ACP_RESPONSE" | grep -q "MINIMAL_OK"; then
    test_pass
  else
    test_fail "minimal config prompt did not return MINIMAL_OK: $(echo "$ACP_RESPONSE" | head -c 60)"
  fi
  acp_stop
  rm -f "$TMP_CFG"
fi

# --------------------------------------------------------------------------
# Test: config with no models section
# --------------------------------------------------------------------------
if test_start "config with no [models.*] → graceful error"; then
  TMP_CFG=$(mktemp /tmp/roko_nomod_XXXXXX.toml)
  cat > "$TMP_CFG" << EOF
config_version = 2
schema_version = 2
[project]
name = "no-models"
[serve]
port = 6699
[agent]
command = "cat"
model = "sonnet"
timeout_ms = 60000
[providers.openai]
kind = "openai_compat"
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
EOF

  acp_start "$TMP_CFG"
  acp_send '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{}}'
  GOT_SESSION=false
  GOT_ERROR=false
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"sessionId"'; then GOT_SESSION=true; break; fi
    if echo "$line" | grep -q '"error"'; then GOT_ERROR=true; break; fi
  done
  acp_stop
  rm -f "$TMP_CFG"

  if [ "$GOT_ERROR" = true ]; then
    test_pass "returned error"
  elif [ "$GOT_SESSION" = true ]; then
    test_warn "session created without models (should warn)"
  else
    test_fail "no response from session/new with no-models config (no error, no session)"
  fi
fi

# --------------------------------------------------------------------------
# Test: config with provider that has no API key
# --------------------------------------------------------------------------
if test_start "provider without API key shows not-ready"; then
  TMP_CFG=$(mktemp /tmp/roko_nokey_XXXXXX.toml)
  cat > "$TMP_CFG" << EOF
config_version = 2
schema_version = 2
[project]
name = "no-key"
[serve]
port = 6699
[agent]
command = "cat"
model = "test-model"
timeout_ms = 60000
[providers.fake]
kind = "openai_compat"
base_url = "https://api.example.com/v1"
api_key_env = "DEFINITELY_NOT_SET_XXXXXX"
[models.test-model]
provider = "fake"
slug = "fake-model"
EOF

  acp_start "$TMP_CFG"
  acp_send '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{}}'
  NOT_SET_MSG=""
  while IFS= read -r -t 10 line <&4; do
    NS=$(echo "$line" | python3 -c "
import sys,json
d=json.load(sys.stdin)
for o in d.get('result',{}).get('configOptions',[]):
  if o.get('id')=='provider':
    for p in o.get('options',[]):
      if 'not set' in p.get('description','').lower():
        print(p.get('description',''))
        break
" 2>/dev/null) || true
    if [ -n "$NS" ]; then NOT_SET_MSG="$NS"; break; fi
    if echo "$line" | grep -q '"error"'; then break; fi
  done
  acp_stop
  rm -f "$TMP_CFG"

  if [ -n "$NOT_SET_MSG" ]; then
    test_pass "reports: $NOT_SET_MSG"
  else
    test_warn "no not-set message found"
  fi
fi

# --------------------------------------------------------------------------
print_section "Protocol Edge Cases"

# --------------------------------------------------------------------------
# Test: send prompt before session/new
# --------------------------------------------------------------------------
if test_start "prompt before session/new → error"; then
  acp_start
  acp_send '{"jsonrpc":"2.0","method":"session/prompt","id":1,"params":{"sessionId":"sess_fake","prompt":[{"type":"text","text":"hi"}]}}'
  if acp_read_until '"id":1' 10; then
    if echo "$ACP_LAST_LINE" | grep -q '"error"'; then
      test_pass
    else
      test_fail "prompt with fake session returned success instead of error: $(echo "$ACP_LAST_LINE" | head -c 80)"
    fi
  else
    test_fail "no JSON-RPC response within 10s for prompt with invalid session"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: rapid-fire prompts (second before first completes)
# --------------------------------------------------------------------------
if test_start "rapid-fire: 2nd prompt while 1st in-flight"; then
  if [ "${QUICK:-false}" = true ]; then
    test_skip "--quick mode"
  else
    acp_start
    acp_session_new "sonnet"
    # Send first prompt (long)
    acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":50,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"Count from 1 to 20, one per line\"}]}}"
    sleep 0.5
    # Send second prompt immediately
    acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":51,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"Say: SECOND\"}]}}"

    GOT_50=false
    GOT_51=false
    GOT_ERROR=false
    while IFS= read -r -t 30 line <&4; do
      [ -z "$line" ] && continue
      if echo "$line" | grep -q '"id":50'; then GOT_50=true; fi
      if echo "$line" | grep -q '"id":51'; then GOT_51=true; fi
      if echo "$line" | grep -q '"error"'; then GOT_ERROR=true; fi
      if [ "$GOT_50" = true ] && [ "$GOT_51" = true ]; then break; fi
      if [ "$GOT_50" = true ] && [ "$GOT_ERROR" = true ]; then break; fi
    done

    if [ "$GOT_50" = true ] && [ "$GOT_51" = true ]; then
      test_pass "both completed"
    elif [ "$GOT_ERROR" = true ]; then
      test_pass "rejected concurrent prompt (correct)"
    elif [ "$GOT_50" = true ]; then
      test_warn "first completed, second lost"
    else
      test_fail "neither prompt completed (got_50=$GOT_50, got_51=$GOT_51, got_error=$GOT_ERROR)"
    fi
    acp_stop
  fi
fi

# --------------------------------------------------------------------------
# Test: empty prompt text
# --------------------------------------------------------------------------
if test_start "empty prompt text"; then
  acp_start
  acp_session_new "sonnet"
  acp_prompt "" 15
  if [ -n "$ACP_RESPONSE" ] || [ -n "$ACP_ERROR" ] || [ "$ACP_STOP_REASON" = "end_turn" ]; then
    test_pass "handled gracefully"
  else
    test_fail "empty prompt produced no response, no error, and stop_reason=${ACP_STOP_REASON:-unset}"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: very long prompt
# --------------------------------------------------------------------------
if test_start "long prompt (2000 chars)"; then
  if [ "${QUICK:-false}" = true ]; then
    test_skip "--quick mode"
  else
    acp_start
    acp_session_new "sonnet"
    LONG_TEXT=$(python3 -c "print('word ' * 400)")
    acp_prompt "Summarize in one sentence: $LONG_TEXT" 30
    if [ -n "$ACP_RESPONSE" ] && [ "$ACP_STOP_REASON" = "end_turn" ]; then
      test_pass "responded (${#ACP_RESPONSE} chars)"
    else
      test_fail "long prompt got no response or bad stop_reason (stop_reason=${ACP_STOP_REASON:-unset}, response_len=${#ACP_RESPONSE})"
    fi
    acp_stop
  fi
fi

# --------------------------------------------------------------------------
print_summary
echo "$TESTS_PASSED $TESTS_FAILED $TESTS_SKIPPED" > "$LOG_DIR/edge-results.txt"
exit $TESTS_FAILED
