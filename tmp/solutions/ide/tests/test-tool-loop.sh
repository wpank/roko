#!/bin/bash
# ============================================================================
# Tool Loop Tests — multi-tool calls, error handling, iteration limits
# ============================================================================
# Tests how roko handles tool calling loops — multiple iterations,
# tool errors, and the MAX_TOOL_ITERATIONS cap.
# Requires: bridge running
# ============================================================================

source "$(dirname "$0")/lib.sh"

print_header "Tool Loop Tests"
print_env
check_prereqs

BRIDGE_AVAILABLE=false
MCP_AVAILABLE=false

if check_bridge; then
  BRIDGE_AVAILABLE=true
  echo -e "  ${PASS_MARK} Bridge reachable"
else
  echo -e "  ${WARN_MARK} Bridge not reachable"
fi

if [ -n "$NUNCHI_MCP" ] && [ -x "$NUNCHI_MCP" ]; then
  MCP_AVAILABLE=true
  echo -e "  ${PASS_MARK} nunchi-mcp: $NUNCHI_MCP"
else
  echo -e "  ${WARN_MARK} nunchi-mcp not found"
fi
echo ""

# --------------------------------------------------------------------------
print_section "Multi-Tool Calls"

# --------------------------------------------------------------------------
# Test: agent makes multiple tool calls in sequence
# --------------------------------------------------------------------------
if test_start "multiple tool calls in one turn"; then
  if [ "$MCP_AVAILABLE" = true ] && [ "$BRIDGE_AVAILABLE" = true ]; then
    BRIDGE_TOKEN=$(cat "$BRIDGE_TOKEN_FILE")
    MCP_PARAMS="\"mcpServers\":[{\"name\":\"nunchi\",\"transport\":{\"type\":\"stdio\",\"command\":\"$NUNCHI_MCP\",\"args\":[],\"env\":{\"BRIDGE_TOKEN\":\"$BRIDGE_TOKEN\",\"BRIDGE_URL\":\"$BRIDGE_URL\"}}}]"

    acp_start
    acp_session_new "sonnet" "$MCP_PARAMS"
    # Ask for something that needs multiple tool calls
    acp_prompt "First call nunchi_tools_list to see what tools are available, then call nunchi_tiles_list to see tiles. Tell me both results." 90
    if [ "$ACP_TOOL_CALLS" -ge 2 ]; then
      test_pass "$ACP_TOOL_CALLS tool calls"
    elif [ "$ACP_TOOL_CALLS" -eq 1 ]; then
      test_warn "only 1 tool call (model may have combined)"
    else
      test_fail "expected >=2 tool calls, got $ACP_TOOL_CALLS"
    fi
    acp_stop
  else
    test_skip "bridge/mcp unavailable"
  fi
fi

# --------------------------------------------------------------------------
# Test: tool call with creation side effect
# --------------------------------------------------------------------------
if test_start "tool call creates a tile"; then
  if [ "$MCP_AVAILABLE" = true ] && [ "$BRIDGE_AVAILABLE" = true ]; then
    BRIDGE_TOKEN=$(cat "$BRIDGE_TOKEN_FILE")
    MCP_PARAMS="\"mcpServers\":[{\"name\":\"nunchi\",\"transport\":{\"type\":\"stdio\",\"command\":\"$NUNCHI_MCP\",\"args\":[],\"env\":{\"BRIDGE_TOKEN\":\"$BRIDGE_TOKEN\",\"BRIDGE_URL\":\"$BRIDGE_URL\"}}}]"

    acp_start
    acp_session_new "sonnet" "$MCP_PARAMS"
    acp_prompt "Create a new tile using nunchi_tiles_create with type 'widget' and initialState containing kind 'metric', title 'Test Metric', value '42', label 'count'. Report the tile ID you get back." 90
    if [ "$ACP_TOOL_CALLS" -ge 1 ]; then
      if echo "$ACP_RAW" | grep -q "tiles_create\|tiles\.create"; then
        test_pass "tiles_create called"
      else
        test_pass "$ACP_TOOL_CALLS tool call(s) made"
      fi
    else
      test_warn "no tool calls (model may not have understood, tool_calls=$ACP_TOOL_CALLS)"
    fi
    acp_stop
  else
    test_skip "bridge/mcp unavailable"
  fi
fi

# --------------------------------------------------------------------------
print_section "Tool Error Handling"

# --------------------------------------------------------------------------
# Test: tool call returns error from bridge
# --------------------------------------------------------------------------
if test_start "tool error propagates to model"; then
  if [ "$MCP_AVAILABLE" = true ] && [ "$BRIDGE_AVAILABLE" = true ]; then
    BRIDGE_TOKEN=$(cat "$BRIDGE_TOKEN_FILE")
    MCP_PARAMS="\"mcpServers\":[{\"name\":\"nunchi\",\"transport\":{\"type\":\"stdio\",\"command\":\"$NUNCHI_MCP\",\"args\":[],\"env\":{\"BRIDGE_TOKEN\":\"$BRIDGE_TOKEN\",\"BRIDGE_URL\":\"$BRIDGE_URL\"}}}]"

    acp_start
    acp_session_new "sonnet" "$MCP_PARAMS"
    # Ask to create a tile with invalid params — should trigger an error that the model sees and reports
    acp_prompt "Call nunchi_tiles_create with type='widget' but without any initialState. Tell me exactly what error you get back." 60
    if [ "$ACP_TOOL_CALLS" -ge 1 ]; then
      if echo "$ACP_RESPONSE" | grep -qi "error\|invalid\|required\|kind\|must"; then
        test_pass "model reports tool error"
      else
        test_warn "tool called but model didn't report error clearly (response: $(echo "$ACP_RESPONSE" | head -c 60))"
      fi
    else
      test_warn "no tool call attempted (tool_calls=$ACP_TOOL_CALLS)"
    fi
    acp_stop
  else
    test_skip "bridge/mcp unavailable"
  fi
fi

# --------------------------------------------------------------------------
print_section "Context Window"

# --------------------------------------------------------------------------
# Test: usage_update shows context consumption
# --------------------------------------------------------------------------
if test_start "usage_update reports context window"; then
  acp_start
  acp_session_new "sonnet"
  acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":2,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"Say: hi\"}]}}"
  USAGE_SIZE=""
  USAGE_USED=""
  while IFS= read -r -t 15 line <&4; do
    [ -z "$line" ] && continue
    USAGE=$(echo "$line" | python3 -c "
import sys,json
d=json.load(sys.stdin)
u=d.get('params',{}).get('update',{})
if u.get('sessionUpdate')=='usage_update':
  print(f\"{u.get('size','')},{u.get('used','')}\")
" 2>/dev/null) || true
    if [ -n "$USAGE" ]; then
      USAGE_SIZE=$(echo "$USAGE" | cut -d, -f1)
      USAGE_USED=$(echo "$USAGE" | cut -d, -f2)
    fi
    if echo "$line" | grep -q '"id":2'; then break; fi
  done
  if [ -n "$USAGE_SIZE" ] && [ -n "$USAGE_USED" ]; then
    test_pass "size=$USAGE_SIZE used=$USAGE_USED"
  else
    test_fail "no usage_update received (size='$USAGE_SIZE', used='$USAGE_USED')"
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: context grows across turns
# --------------------------------------------------------------------------
if test_start "context grows across multiple turns"; then
  if [ "${QUICK:-false}" = true ]; then
    test_skip "--quick"
  else
    acp_start
    acp_session_new "sonnet"
    # First turn
    acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":2,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"Say: turn1\"}]}}"
    USAGE1=""
    while IFS= read -r -t 15 line <&4; do
      U=$(echo "$line" | python3 -c "import sys,json; d=json.load(sys.stdin); u=d.get('params',{}).get('update',{}); print(u.get('used','')) if u.get('sessionUpdate')=='usage_update' else print('')" 2>/dev/null) || true
      [ -n "$U" ] && USAGE1="$U"
      if echo "$line" | grep -q '"id":2'; then break; fi
    done
    acp_drain
    # Second turn
    acp_send "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":3,\"params\":{\"sessionId\":\"$ACP_SESSION_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"Say: turn2\"}]}}"
    USAGE2=""
    while IFS= read -r -t 15 line <&4; do
      U=$(echo "$line" | python3 -c "import sys,json; d=json.load(sys.stdin); u=d.get('params',{}).get('update',{}); print(u.get('used','')) if u.get('sessionUpdate')=='usage_update' else print('')" 2>/dev/null) || true
      [ -n "$U" ] && USAGE2="$U"
      if echo "$line" | grep -q '"id":3'; then break; fi
    done
    if [ -n "$USAGE1" ] && [ -n "$USAGE2" ] && [ "$USAGE2" -gt "$USAGE1" ]; then
      test_pass "turn1=$USAGE1 turn2=$USAGE2 (grew)"
    elif [ -n "$USAGE1" ] && [ -n "$USAGE2" ]; then
      test_warn "turn1=$USAGE1 turn2=$USAGE2 (didn't grow?)"
    else
      test_fail "missing usage data (turn1='$USAGE1', turn2='$USAGE2')"
    fi
    acp_stop
  fi
fi

# --------------------------------------------------------------------------
print_summary
echo "$TESTS_PASSED $TESTS_FAILED $TESTS_SKIPPED" > "$LOG_DIR/toolloop-results.txt"
exit $TESTS_FAILED
