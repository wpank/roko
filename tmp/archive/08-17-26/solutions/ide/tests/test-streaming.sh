#!/bin/bash
# ============================================================================
# Streaming Protocol Tests — session/update notifications, chunk shapes
# ============================================================================
# Tests the actual streaming behavior of session/prompt — verifying that
# the IDE receives properly shaped session/update notifications.
# ============================================================================

source "$(dirname "$0")/lib.sh"

print_header "Streaming Protocol Tests"
print_env
check_prereqs

# --------------------------------------------------------------------------
print_section "Update Notifications"

# --------------------------------------------------------------------------
# Test: session/update notifications arrive during prompt
# --------------------------------------------------------------------------
if test_start "session/update notifications during prompt"; then
  acp_start
  acp_session_new "sonnet"
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":2,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"Say: hello world\"}]}}"
  UPDATES=0
  HAS_CHUNK=false
  HAS_FINAL=false
  while IFS= read -r -t 30 line <&4; do
    [ -z "$line" ] && continue
    # Count session/update notifications
    if echo "$line" | grep -q '"method":"session/update"'; then
      UPDATES=$((UPDATES + 1))
      # Check for agent_message_chunk
      if echo "$line" | grep -q 'agent_message_chunk\|agentMessageChunk'; then
        HAS_CHUNK=true
      fi
    fi
    # Final response
    if echo "$line" | grep -q '"id":2'; then
      HAS_FINAL=true
      break
    fi
  done
  if [ "$HAS_CHUNK" = true ] && [ "$HAS_FINAL" = true ]; then
    test_pass "$UPDATES update notifications"
  elif [ "$HAS_FINAL" = true ]; then
    test_warn "got final response but no chunk notifications ($UPDATES updates)"
  else
    test_fail "no final response received (updates=$UPDATES, has_chunk=$HAS_CHUNK)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: chunk notifications have correct shape
# --------------------------------------------------------------------------
if test_start "chunk notifications contain text delta"; then
  acp_start
  acp_session_new "sonnet"
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":2,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"Say exactly: CHUNK_TEST\"}]}}"
  CHUNK_TEXT=""
  while IFS= read -r -t 30 line <&4; do
    [ -z "$line" ] && continue
    if echo "$line" | grep -q 'agent_message_chunk\|agentMessageChunk'; then
      # Extract delta text
      DELTA=$(echo "$line" | python3 -c "
import sys,json
d=json.load(sys.stdin)
u=d.get('params',{}).get('update',{})
# Text lives in content.text, not delta
c=u.get('content',{})
print(c.get('text','') if isinstance(c,dict) else '')" 2>/dev/null) || true
      if [ -n "$DELTA" ]; then
        CHUNK_TEXT="${CHUNK_TEXT}${DELTA}"
      fi
    fi
    if echo "$line" | grep -q '"id":2'; then break; fi
  done
  if echo "$CHUNK_TEXT" | grep -q "CHUNK_TEST"; then
    test_pass "assembled: $(echo "$CHUNK_TEXT" | head -c 40)"
  elif [ -n "$CHUNK_TEXT" ]; then
    test_warn "got chunks but missing expected text: $(echo "$CHUNK_TEXT" | head -c 40)"
  else
    test_fail "no chunk text received (CHUNK_TEXT empty)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: final response has stop_reason
# --------------------------------------------------------------------------
if test_start "final response contains stopReason"; then
  acp_start
  acp_session_new "sonnet"
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":2,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"Say: done\"}]}}"
  STOP_REASON=""
  while IFS= read -r -t 30 line <&4; do
    [ -z "$line" ] && continue
    if echo "$line" | grep -q '"id":2'; then
      STOP_REASON=$(echo "$line" | python3 -c "import sys,json; print(json.load(sys.stdin).get('result',{}).get('stopReason',''))" 2>/dev/null) || true
      break
    fi
  done
  if [ "$STOP_REASON" = "end_turn" ]; then
    test_pass "stopReason=end_turn"
  elif [ -n "$STOP_REASON" ]; then
    test_pass "stopReason=$STOP_REASON"
  else
    test_fail "no stopReason in final response (STOP_REASON empty)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: usage_update notification shape
# --------------------------------------------------------------------------
if test_start "usage_update has size and used fields"; then
  acp_start
  acp_session_new "sonnet"
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":2,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"Say: hi\"}]}}"
  USAGE_DATA=""
  while IFS= read -r -t 15 line <&4; do
    [ -z "$line" ] && continue
    USAGE=$(echo "$line" | python3 -c "
import sys,json
d=json.load(sys.stdin)
u=d.get('params',{}).get('update',{})
if u.get('sessionUpdate')=='usage_update':
  size=u.get('size','')
  used=u.get('used','')
  print(f'{size},{used}')
" 2>/dev/null) || true
    if [ -n "$USAGE" ]; then
      USAGE_DATA="$USAGE"
    fi
    if echo "$line" | grep -q '"id":2'; then break; fi
  done
  if [ -n "$USAGE_DATA" ]; then
    SIZE=$(echo "$USAGE_DATA" | cut -d, -f1)
    USED=$(echo "$USAGE_DATA" | cut -d, -f2)
    if [ "$SIZE" -gt 0 ] 2>/dev/null && [ "$USED" -gt 0 ] 2>/dev/null; then
      test_pass "size=$SIZE used=$USED"
    else
      test_warn "usage data present but invalid: size=$SIZE used=$USED"
    fi
  else
    test_fail "no usage_update notification received"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
print_section "Thinking/Extended Output"

# --------------------------------------------------------------------------
# Test: thinking chunks appear with extended thinking (if supported)
# --------------------------------------------------------------------------
if test_start "thinking_chunk notifications with effort=high"; then
  if [ "${QUICK:-false}" = true ]; then
    test_skip "--quick"
  else
    acp_start
    acp_session_new "sonnet"
    # Set effort to high to trigger thinking
    acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/config/update\",\"id\":5,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"optionId\":\"effort\",\"newValue\":\"high\"}}"
    while IFS= read -r -t 10 line <&4; do
      if echo "$line" | grep -q '"id":5'; then break; fi
    done
    acp_drain
    # Prompt that should trigger thinking
    acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":6,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"What is 15 * 23? Think step by step.\"}]}}"
    HAS_THINKING=false
    HAS_RESPONSE=false
    while IFS= read -r -t 45 line <&4; do
      [ -z "$line" ] && continue
      if echo "$line" | grep -q 'thinking_chunk\|thinkingChunk'; then
        HAS_THINKING=true
      fi
      if echo "$line" | grep -q '"id":6'; then
        HAS_RESPONSE=true
        break
      fi
    done
    if [ "$HAS_THINKING" = true ]; then
      test_pass "thinking chunks received"
    elif [ "$HAS_RESPONSE" = true ]; then
      test_warn "response received but no thinking chunks (model may not use extended thinking)"
    else
      test_fail "no response received (has_thinking=$HAS_THINKING, has_response=$HAS_RESPONSE)"
    fi
    acp_stop
  fi
fi

# --------------------------------------------------------------------------
print_section "Session Update Types"

# --------------------------------------------------------------------------
# Test: enumerate all session update types in one conversation
# --------------------------------------------------------------------------
if test_start "all expected update types present"; then
  acp_start
  acp_session_new "sonnet"
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":2,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"Tell me a one-sentence joke.\"}]}}"
  declare -A UPDATE_TYPES
  while IFS= read -r -t 30 line <&4; do
    [ -z "$line" ] && continue
    if echo "$line" | grep -q '"method":"session/update"'; then
      UTYPE=$(echo "$line" | python3 -c "
import sys,json
d=json.load(sys.stdin)
u=d.get('params',{}).get('update',{})
print(u.get('sessionUpdate','unknown'))
" 2>/dev/null) || true
      if [ -n "$UTYPE" ]; then
        UPDATE_TYPES["$UTYPE"]=1
      fi
    fi
    if echo "$line" | grep -q '"id":2'; then break; fi
  done
  TYPES_FOUND="${!UPDATE_TYPES[*]}"
  # We expect at minimum: agent_message_chunk, usage_update
  if [[ "$TYPES_FOUND" == *"agent_message_chunk"* ]] && [[ "$TYPES_FOUND" == *"usage_update"* ]]; then
    test_pass "types: $TYPES_FOUND"
  elif [[ "$TYPES_FOUND" == *"agent_message_chunk"* ]]; then
    test_warn "has chunks but no usage_update: $TYPES_FOUND"
  else
    test_fail "missing expected update types, found: '$TYPES_FOUND' (need agent_message_chunk + usage_update)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: multiple prompts accumulate message history
# --------------------------------------------------------------------------
if test_start "multi-turn context preserved"; then
  if [ "${QUICK:-false}" = true ]; then
    test_skip "--quick"
  else
    acp_start
    acp_session_new "sonnet"
    # Set provider to openai first (BUG#03: HashMap may pick wrong default provider)
    acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/config/update\",\"id\":10,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"optionId\":\"provider\",\"newValue\":\"openai\"}}"
    while IFS= read -r -t 10 line <&4; do
      if echo "$line" | grep -q '"id":10'; then break; fi
    done
    acp_drain
    # Then set model to sonnet (only valid after provider is openai)
    acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/config/update\",\"id\":11,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"optionId\":\"model\",\"newValue\":\"sonnet\"}}"
    while IFS= read -r -t 10 line <&4; do
      if echo "$line" | grep -q '"id":11'; then break; fi
    done
    acp_drain
    # First turn: establish a fact
    acp_prompt "The answer to my math problem is 7742. Just say OK, got it." 30
    sleep 0.5
    acp_drain
    sleep 0.5
    # Second turn: ask for the fact back
    acp_prompt "What number did I mention in my math problem? Reply with just the number." 30 3
    if echo "$ACP_RESPONSE" | grep -q "7742"; then
      test_pass "context preserved across turns"
    else
      test_fail "context lost, expected '7742' in response, got: $(echo "$ACP_RESPONSE" | head -c 80)"
    fi
    acp_stop
  fi
fi

# --------------------------------------------------------------------------
print_section "Response Format"

# --------------------------------------------------------------------------
# Test: final response has expected fields
# --------------------------------------------------------------------------
if test_start "final response shape"; then
  acp_start
  acp_session_new "sonnet"
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":2,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"Say: test\"}]}}"
  FINAL=""
  while IFS= read -r -t 30 line <&4; do
    if echo "$line" | grep -q '"id":2'; then
      FINAL="$line"
      break
    fi
  done
  FIELDS=$(echo "$FINAL" | python3 -c "
import sys,json
d=json.load(sys.stdin)
r=d.get('result',{})
fields=list(r.keys())
print(','.join(sorted(fields)))
" 2>/dev/null) || true
  # Expect at minimum: stopReason
  if echo "$FIELDS" | grep -q "stopReason"; then
    test_pass "fields: $FIELDS"
  else
    test_fail "missing stopReason in response fields: '$FIELDS'"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: session/new response has config_options
# --------------------------------------------------------------------------
if test_start "session/new returns configOptions"; then
  acp_start
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/new\",\"id\":2,\"params\":{\"model\":\"sonnet\"}}"
  NEW_RESULT=""
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":2'; then
      NEW_RESULT="$line"
      break
    fi
  done
  HAS_CONFIG=$(echo "$NEW_RESULT" | python3 -c "
import sys,json
d=json.load(sys.stdin)
r=d.get('result',{})
co=r.get('configOptions',[])
print(len(co))
" 2>/dev/null) || true
  if [ -n "$HAS_CONFIG" ] && [ "$HAS_CONFIG" -gt 0 ] 2>/dev/null; then
    test_pass "$HAS_CONFIG config options returned"
  else
    test_warn "no configOptions in session/new response (count=$HAS_CONFIG)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: session/new returns modes
# --------------------------------------------------------------------------
if test_start "session/new returns modes"; then
  acp_start
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/new\",\"id\":2,\"params\":{\"model\":\"sonnet\"}}"
  NEW_RESULT=""
  while IFS= read -r -t 10 line <&4; do
    if echo "$line" | grep -q '"id":2'; then
      NEW_RESULT="$line"
      break
    fi
  done
  MODES=$(echo "$NEW_RESULT" | python3 -c "
import sys,json
d=json.load(sys.stdin)
r=d.get('result',{})
m=r.get('modes',{})
modes=m.get('availableModes',[])
current=m.get('currentModeId','')
print(f'{len(modes)},{current}')
" 2>/dev/null) || true
  MODE_COUNT=$(echo "$MODES" | cut -d, -f1)
  MODE_CURRENT=$(echo "$MODES" | cut -d, -f2)
  if [ -n "$MODE_COUNT" ] && [ "$MODE_COUNT" -gt 0 ] 2>/dev/null; then
    test_pass "$MODE_COUNT modes, current=$MODE_CURRENT"
  else
    test_warn "no modes in response (count=$MODE_COUNT)"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
print_summary
echo "$TESTS_PASSED $TESTS_FAILED $TESTS_SKIPPED" > "$LOG_DIR/streaming-results.txt"
exit $TESTS_FAILED
