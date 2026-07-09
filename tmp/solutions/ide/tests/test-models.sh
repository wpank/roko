#!/bin/bash
# ============================================================================
# Model & Provider Tests — resolution, fallback, validation
# ============================================================================
# Tests model selection, provider routing, and config edge cases.
# Issues tested: #02 (model param ignored), #03 (HashMap ordering), #11 (max_output)
# ============================================================================

source "$(dirname "$0")/lib.sh"

print_header "Model & Provider Tests"
print_env
check_prereqs

# --------------------------------------------------------------------------
print_section "Model Selection"

# --------------------------------------------------------------------------
# Test: model param in session/new (KNOWN BUG #02)
# --------------------------------------------------------------------------
if test_start "session/new respects model param"; then
  acp_start
  # Send session/new with explicit model, check if configOptions reflects it
  acp_send '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{"model":"haiku"}}'
  CURRENT_MODEL=""
  while IFS= read -r -t 10 line <&4; do
    CM=$(echo "$line" | python3 -c "
import sys,json
d=json.load(sys.stdin)
opts=d.get('result',{}).get('configOptions',[])
for o in opts:
  if o.get('id')=='model':
    print(o.get('currentValue',''))
    break
" 2>/dev/null) || true
    if [ -n "$CM" ]; then
      CURRENT_MODEL="$CM"
      break
    fi
  done
  acp_drain

  if [ "$CURRENT_MODEL" = "haiku" ]; then
    test_pass "model=haiku honored"
  else
    test_fail "expected haiku, got currentValue='$CURRENT_MODEL' (BUG #02: model param ignored)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: requesting nonexistent model
# --------------------------------------------------------------------------
if test_start "nonexistent model → error or warning"; then
  # Create a minimal config with only one model
  TMP_CFG=$(mktemp /tmp/roko_cfg_XXXXXX.toml)
  cat > "$TMP_CFG" << 'EOF'
config_version = 2
schema_version = 2
[project]
name = "test"
[serve]
port = 6699
[agent]
command = "cat"
model = "my-model"
timeout_ms = 60000
[providers.openai]
kind = "openai_compat"
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
[models.my-model]
provider = "openai"
slug = "gpt-4o"
supports_tools = true
context_window = 128000
max_output = 16000
EOF

  acp_start "$TMP_CFG"
  # Request nonexistent model
  acp_send '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{"model":"does-not-exist"}}'
  GOT_ERROR=false
  GOT_WARNING=false
  USED_MODEL=""
  WARNINGS=""
  RAW_RESPONSE=""
  while IFS= read -r -t 10 line <&4; do
    RAW_RESPONSE+="$line"$'\n'
    if echo "$line" | grep -q '"error"'; then
      GOT_ERROR=true
      break
    fi
    UM=$(echo "$line" | python3 -c "
import sys,json
d=json.load(sys.stdin)
r=d.get('result',{})
opts=r.get('configOptions',[])
for o in opts:
  if o.get('id')=='model':
    print(o.get('currentValue',''))
" 2>/dev/null) || true
    WARNINGS=$(echo "$line" | python3 -c "
import sys,json
d=json.load(sys.stdin)
w=d.get('result',{}).get('warnings',[])
print('|'.join(w)) if w else print('')
" 2>/dev/null) || true
    if [ -n "$UM" ]; then
      USED_MODEL="$UM"
      break
    fi
  done
  acp_stop
  rm -f "$TMP_CFG"

  if [ "$GOT_ERROR" = true ]; then
    test_pass "returns error for invalid model"
  elif [ -n "$WARNINGS" ]; then
    test_pass "returned warning: $(echo "$WARNINGS" | head -c 80)"
  elif [ "$USED_MODEL" = "my-model" ]; then
    test_warn "silently fell back to $USED_MODEL (no error or warning)"
  else
    test_fail "fell back to model='$USED_MODEL', raw response: $(echo "$RAW_RESPONSE" | head -c 120)"
  fi
fi

# --------------------------------------------------------------------------
print_section "Provider Routing"

# --------------------------------------------------------------------------
# Test: openai provider works
# --------------------------------------------------------------------------
if test_start "openai provider (gpt-4o)"; then
  acp_start
  acp_session_new "sonnet"
  acp_prompt "Say exactly: OPENAI_OK"
  if echo "$ACP_RESPONSE" | grep -q "OPENAI_OK"; then
    test_pass
  else
    test_fail "expected 'OPENAI_OK' in response, got: '$(echo "$ACP_RESPONSE" | head -c 100)'"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: claude_cli provider
# --------------------------------------------------------------------------
if test_start "claude_cli provider"; then
  # Check if claude-sonnet-4-6 model exists in config
  if grep -q "claude-sonnet-4-6\|claude_cli" "$ROKO_CONFIG" 2>/dev/null; then
    acp_start
    acp_session_new "claude-sonnet-4-6"
    if [ -n "$ACP_SESSION_ID" ]; then
      acp_prompt "Say exactly: CLI_OK"
      if echo "$ACP_RESPONSE" | grep -q "CLI_OK"; then
        test_pass
      elif [ -z "$ACP_RESPONSE" ]; then
        test_fail "empty response from claude_cli (may not work nested), session=$ACP_SESSION_ID"
      else
        test_fail "expected 'CLI_OK', got: '$(echo "$ACP_RESPONSE" | head -c 100)'"
      fi
    else
      test_fail "session creation failed — no ACP_SESSION_ID returned"
    fi
    acp_stop
  else
    test_skip "claude_cli not in config"
  fi
fi

# --------------------------------------------------------------------------
# Test: provider readiness info
# --------------------------------------------------------------------------
if test_start "provider readiness in configOptions"; then
  acp_start
  acp_send '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{}}'
  READY_COUNT=0
  NOT_READY_COUNT=0
  RAW_OPTS=""
  while IFS= read -r -t 10 line <&4; do
    COUNTS=$(echo "$line" | python3 -c "
import sys,json
d=json.load(sys.stdin)
opts=d.get('result',{}).get('configOptions',[])
ready=0; notready=0
for o in opts:
  if o.get('id')=='provider':
    for p in o.get('options',[]):
      if 'Ready' in p.get('description',''):
        ready+=1
      elif 'not set' in p.get('description',''):
        notready+=1
    break
print(f'{ready},{notready}')
" 2>/dev/null) || true
    if [ -n "$COUNTS" ]; then
      READY_COUNT=$(echo "$COUNTS" | cut -d, -f1)
      NOT_READY_COUNT=$(echo "$COUNTS" | cut -d, -f2)
      RAW_OPTS="$(echo "$line" | head -c 200)"
      break
    fi
  done
  acp_stop

  if [ "$READY_COUNT" -gt 0 ]; then
    test_pass "${READY_COUNT} ready, ${NOT_READY_COUNT} not ready"
  else
    test_fail "no providers reported as ready, configOptions response: ${RAW_OPTS:-<empty>}"
  fi
fi

# --------------------------------------------------------------------------
print_section "Output Limits"

# --------------------------------------------------------------------------
# Test: long output not truncated
# --------------------------------------------------------------------------
if test_start "long output (50 numbers) not truncated"; then
  acp_start
  acp_session_new "sonnet"
  acp_prompt "List numbers 1 through 50, separated by commas. Nothing else." 20
  # Count how many numbers appear
  NUM_COUNT=$(echo "$ACP_RESPONSE" | grep -oE '[0-9]+' | wc -l | tr -d ' ')
  if [ "$NUM_COUNT" -ge 45 ]; then
    test_pass "$NUM_COUNT numbers found"
  elif [ "$NUM_COUNT" -ge 20 ]; then
    test_warn "only $NUM_COUNT/50 numbers (possible truncation), tail: '$(echo "$ACP_RESPONSE" | tail -c 80)'"
  else
    test_fail "only $NUM_COUNT/50 numbers (likely max_output too low), response: '$(echo "$ACP_RESPONSE" | head -c 120)'"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
print_summary
echo "$TESTS_PASSED $TESTS_FAILED $TESTS_SKIPPED" > "$LOG_DIR/models-results.txt"
exit $TESTS_FAILED
