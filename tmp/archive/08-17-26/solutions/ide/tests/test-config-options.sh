#!/bin/bash
# ============================================================================
# Config Options Tests — provider/model/effort defaults, switching, validation
# ============================================================================
# Tests the config option system: what gets returned in session/new,
# whether config/update actually changes behavior, and edge cases.
# ============================================================================

source "$(dirname "$0")/lib.sh"

print_header "Config Options Tests"
print_env
check_prereqs

# --------------------------------------------------------------------------
print_section "Config Option Enumeration"

# --------------------------------------------------------------------------
# Test: session/new returns all expected config option IDs
# --------------------------------------------------------------------------
if test_start "config options include provider, model, effort"; then
  acp_start
  acp_session_new "sonnet"
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/config/update\",\"id\":5,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"optionId\":\"provider\",\"newValue\":\"openai\"}}"
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":5'; then
      CONFIG_RESULT="$line"
      break
    fi
  done
  OPTION_IDS=$(echo "$CONFIG_RESULT" | python3 -c "
import sys,json
d=json.load(sys.stdin)
opts=d.get('result',{}).get('configOptions',[])
ids=[o.get('id','') for o in opts]
print(','.join(sorted(ids)))
" 2>/dev/null) || true
  if echo "$OPTION_IDS" | grep -q "provider" && echo "$OPTION_IDS" | grep -q "model" && echo "$OPTION_IDS" | grep -q "effort"; then
    test_pass "ids: $OPTION_IDS"
  else
    test_fail "missing expected options in '$OPTION_IDS' (need provider, model, effort)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: provider options include readiness description
# --------------------------------------------------------------------------
if test_start "provider options have readiness descriptions"; then
  acp_start
  acp_session_new "sonnet"
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/config/update\",\"id\":5,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"optionId\":\"provider\",\"newValue\":\"openai\"}}"
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":5'; then
      CONFIG_RESULT="$line"
      break
    fi
  done
  READINESS=$(echo "$CONFIG_RESULT" | python3 -c "
import sys,json
d=json.load(sys.stdin)
opts=d.get('result',{}).get('configOptions',[])
for o in opts:
  if o.get('id')=='provider':
    for v in o.get('options',[]):
      desc=v.get('description','')
      if 'Ready' in desc or 'not set' in desc or 'API key' in desc:
        print(f'{v[\"value\"]}={desc}')
        break
" 2>/dev/null) || true
  if [ -n "$READINESS" ]; then
    test_pass "readiness: $READINESS"
  else
    test_fail "no readiness descriptions found in provider options"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
print_section "Default Selection (BUG #03)"

# --------------------------------------------------------------------------
# Test: default provider selection is deterministic
# --------------------------------------------------------------------------
if test_start "default provider is consistent across sessions"; then
  PROVIDERS=""
  for i in 1 2 3; do
    acp_start
    acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/new\",\"id\":1,\"params\":{\"model\":\"sonnet\"}}"
    while IFS= read -r -t 10 line <&4; do
      if echo "$line" | grep -q '"id":1'; then
        PROV=$(echo "$line" | python3 -c "
import sys,json
d=json.load(sys.stdin)
for o in d.get('result',{}).get('configOptions',[]):
  if o.get('id')=='provider':
    print(o.get('currentValue',''))
" 2>/dev/null) || true
        if [ -z "$PROVIDERS" ]; then
          PROVIDERS="$PROV"
        else
          PROVIDERS="$PROVIDERS,$PROV"
        fi
        break
      fi
    done
    acp_stop
  done
  # Check if all three are the same
  FIRST=$(echo "$PROVIDERS" | cut -d, -f1)
  ALL_SAME=true
  for p in $(echo "$PROVIDERS" | tr ',' ' '); do
    if [ "$p" != "$FIRST" ]; then
      ALL_SAME=false
      break
    fi
  done
  if [ "$ALL_SAME" = true ]; then
    test_pass "always $FIRST (consistent)"
  else
    test_fail "non-deterministic providers: $PROVIDERS (BUG#03 HashMap ordering)"
  fi
fi

# --------------------------------------------------------------------------
# Test: default model matches config
# --------------------------------------------------------------------------
if test_start "default model matches roko.toml default_model"; then
  acp_start
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/new\",\"id\":1,\"params\":{\"model\":\"sonnet\"}}"
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":1'; then
      DEFAULT_MODEL=$(echo "$line" | python3 -c "
import sys,json
d=json.load(sys.stdin)
for o in d.get('result',{}).get('configOptions',[]):
  if o.get('id')=='model':
    print(o.get('currentValue',''))
" 2>/dev/null) || true
      DEFAULT_PROV=$(echo "$line" | python3 -c "
import sys,json
d=json.load(sys.stdin)
for o in d.get('result',{}).get('configOptions',[]):
  if o.get('id')=='provider':
    print(o.get('currentValue',''))
" 2>/dev/null) || true
      break
    fi
  done
  # The config has default_model = "sonnet" but the model param in session/new
  # is ignored (BUG#02), so the default comes from HashMap iteration
  if [ "$DEFAULT_MODEL" = "sonnet" ]; then
    test_pass "default=sonnet (as configured)"
  else
    test_warn "default=$DEFAULT_MODEL on provider=$DEFAULT_PROV (BUG#02/#03: model param ignored, HashMap ordering)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
print_section "Config/Update Behavior"

# --------------------------------------------------------------------------
# Test: switching provider changes available models
# --------------------------------------------------------------------------
if test_start "switching provider updates model options"; then
  acp_start
  acp_session_new "sonnet"
  # Switch to openai
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/config/update\",\"id\":5,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"optionId\":\"provider\",\"newValue\":\"openai\"}}"
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":5'; then
      OPENAI_MODELS=$(echo "$line" | python3 -c "
import sys,json
d=json.load(sys.stdin)
for o in d.get('result',{}).get('configOptions',[]):
  if o.get('id')=='model':
    vals=[v.get('value','') for v in o.get('options',[])]
    print(','.join(vals))
" 2>/dev/null) || true
      break
    fi
  done
  acp_drain
  # Switch to anthropic
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/config/update\",\"id\":6,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"optionId\":\"provider\",\"newValue\":\"anthropic\"}}"
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":6'; then
      ANTHROPIC_MODELS=$(echo "$line" | python3 -c "
import sys,json
d=json.load(sys.stdin)
for o in d.get('result',{}).get('configOptions',[]):
  if o.get('id')=='model':
    vals=[v.get('value','') for v in o.get('options',[])]
    print(','.join(vals))
" 2>/dev/null) || true
      break
    fi
  done
  if [ -n "$OPENAI_MODELS" ] && [ -n "$ANTHROPIC_MODELS" ] && [ "$OPENAI_MODELS" != "$ANTHROPIC_MODELS" ]; then
    test_pass "openai=[$OPENAI_MODELS] anthropic=[$ANTHROPIC_MODELS]"
  elif [ -n "$OPENAI_MODELS" ] && [ -n "$ANTHROPIC_MODELS" ]; then
    test_warn "models same for both providers: $OPENAI_MODELS"
  else
    test_fail "missing model data: openai='$OPENAI_MODELS' anthropic='$ANTHROPIC_MODELS'"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: config/update returns updated config options
# --------------------------------------------------------------------------
if test_start "config/update response includes all config options"; then
  acp_start
  acp_session_new "sonnet"
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/config/update\",\"id\":5,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"optionId\":\"effort\",\"newValue\":\"high\"}}"
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":5'; then
      OPT_COUNT=$(echo "$line" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print(len(d.get('result',{}).get('configOptions',[])))
" 2>/dev/null) || true
      break
    fi
  done
  if [ -n "$OPT_COUNT" ] && [ "$OPT_COUNT" -ge 3 ]; then
    test_pass "$OPT_COUNT config options returned"
  else
    test_fail "expected >=3 config options, got $OPT_COUNT"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: config/update with unknown optionId
# --------------------------------------------------------------------------
if test_start "config/update with unknown optionId"; then
  acp_start
  acp_session_new "sonnet"
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/config/update\",\"id\":5,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"optionId\":\"nonexistent_option\",\"newValue\":\"whatever\"}}"
  UNKNOWN_RESULT=""
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":5'; then
      UNKNOWN_RESULT="$line"
      break
    fi
  done
  if echo "$UNKNOWN_RESULT" | grep -q '"error"'; then
    test_pass "returns error for unknown option"
  elif echo "$UNKNOWN_RESULT" | grep -q '"result"'; then
    test_warn "silently accepted unknown option"
  else
    test_fail "no response to unknown optionId (UNKNOWN_RESULT empty)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: model change persists across prompts
# --------------------------------------------------------------------------
if test_start "model change persists in session"; then
  if [ "${QUICK:-false}" = true ]; then
    test_skip "--quick"
  else
    acp_start
    acp_session_new "sonnet"
    # Switch to openai provider first
    acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/config/update\",\"id\":5,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"optionId\":\"provider\",\"newValue\":\"openai\"}}"
    while IFS= read -r -t 10 line <&4; do
      if echo "$line" | grep -q '"id":5'; then break; fi
    done
    acp_drain
    # Set model to sonnet
    acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/config/update\",\"id\":6,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"optionId\":\"model\",\"newValue\":\"sonnet\"}}"
    while IFS= read -r -t 10 line <&4; do
      if echo "$line" | grep -q '"id":6'; then break; fi
    done
    acp_drain
    # Send a prompt and check usage (should be sonnet-level context)
    acp_prompt "Say: persistence_test" 30
    # Now re-check config to see if model is still sonnet
    acp_drain
    acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/config/update\",\"id\":7,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"optionId\":\"effort\",\"newValue\":\"medium\"}}"
    while IFS= read -r -t 10 line <&4; do
      if echo "$line" | grep -q '"id":7'; then
        PERSIST_MODEL=$(echo "$line" | python3 -c "
import sys,json
d=json.load(sys.stdin)
for o in d.get('result',{}).get('configOptions',[]):
  if o.get('id')=='model':
    print(o.get('currentValue',''))
" 2>/dev/null) || true
        break
      fi
    done
    if [ "$PERSIST_MODEL" = "sonnet" ]; then
      test_pass "model still sonnet after prompt"
    else
      test_fail "model changed to '$PERSIST_MODEL', expected 'sonnet'"
    fi
    acp_stop
  fi
fi

# --------------------------------------------------------------------------
print_section "Wire Format Documentation"

# --------------------------------------------------------------------------
# Test: document the full config/update wire format
# --------------------------------------------------------------------------
if test_start "config/update wire format documented"; then
  acp_start
  acp_session_new "sonnet"
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/config/update\",\"id\":5,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"optionId\":\"effort\",\"newValue\":\"low\"}}"
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":5'; then
      # Save raw response to log for documentation
      echo "$line" | python3 -c "import sys,json; print(json.dumps(json.load(sys.stdin),indent=2))" > "$LOG_DIR/config-update-response.json" 2>/dev/null
      if [ -f "$LOG_DIR/config-update-response.json" ]; then
        test_pass "saved to $LOG_DIR/config-update-response.json"
      else
        test_warn "could not save wire format to log dir"
      fi
      break
    fi
  done
  acp_stop
fi

# --------------------------------------------------------------------------
print_summary
echo "$TESTS_PASSED $TESTS_FAILED $TESTS_SKIPPED" > "$LOG_DIR/config-results.txt"
exit $TESTS_FAILED
