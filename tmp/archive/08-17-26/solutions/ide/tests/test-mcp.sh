#!/bin/bash
# ============================================================================
# MCP Integration Tests — tool discovery, invocation, error propagation
# ============================================================================
# Tests MCP server lifecycle and tool calling through the ACP layer.
# Requires: Nunchi IDE bridge running on localhost:6678
# Issues tested: #01 (silent failures), #04 (discovery timing/config)
# ============================================================================

source "$(dirname "$0")/lib.sh"

print_header "MCP Integration Tests"
print_env

check_prereqs

# Check MCP prerequisites
BRIDGE_AVAILABLE=false
MCP_AVAILABLE=false

if check_bridge; then
  BRIDGE_AVAILABLE=true
  echo -e "  ${PASS_MARK} Bridge reachable at $BRIDGE_URL"
else
  echo -e "  ${WARN_MARK} Bridge not reachable (start Nunchi IDE for MCP tests)"
fi

if [ -n "$NUNCHI_MCP" ] && [ -x "$NUNCHI_MCP" ]; then
  MCP_AVAILABLE=true
  echo -e "  ${PASS_MARK} nunchi-mcp binary: $NUNCHI_MCP"
else
  echo -e "  ${WARN_MARK} nunchi-mcp binary not found"
fi
echo ""

# --------------------------------------------------------------------------
print_section "MCP Server Standalone"

# --------------------------------------------------------------------------
# Test: nunchi-mcp responds to initialize
# --------------------------------------------------------------------------
if test_start "nunchi-mcp initialize response"; then
  if [ "$MCP_AVAILABLE" = true ]; then
    INIT_RESP=$(echo '{"jsonrpc":"2.0","method":"initialize","id":1,"params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}' \
      | BRIDGE_TOKEN="${BRIDGE_TOKEN:-$(cat "$BRIDGE_TOKEN_FILE" 2>/dev/null)}" BRIDGE_URL="$BRIDGE_URL" \
        timeout 5 "$NUNCHI_MCP" 2>/dev/null)
    if echo "$INIT_RESP" | grep -q '"serverInfo"'; then
      VERSION=$(echo "$INIT_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['result']['serverInfo']['version'])" 2>/dev/null) || true
      test_pass "v$VERSION"
    else
      test_fail "initialize response missing serverInfo field"
    fi
  else
    test_skip "nunchi-mcp binary not found"
  fi
fi

# --------------------------------------------------------------------------
print_section "MCP via ACP Session"

# --------------------------------------------------------------------------
# Test: MCP tools discovered when binary exists
# --------------------------------------------------------------------------
if test_start "MCP tool discovery (valid binary)"; then
  if [ "$MCP_AVAILABLE" = true ] && [ "$BRIDGE_AVAILABLE" = true ]; then
    BRIDGE_TOKEN=$(cat "$BRIDGE_TOKEN_FILE")
    MCP_PARAMS="\"mcpServers\":[{\"name\":\"nunchi\",\"transport\":{\"type\":\"stdio\",\"command\":\"$NUNCHI_MCP\",\"args\":[],\"env\":{\"BRIDGE_TOKEN\":\"$BRIDGE_TOKEN\",\"BRIDGE_URL\":\"$BRIDGE_URL\"}}}]"

    acp_start
    acp_session_new "sonnet" "$MCP_PARAMS"
    # Be very explicit about wanting a tool call
    acp_prompt "You have MCP tools. Use the nunchi_tools_list tool now. Do not explain, just call it." 60
    if [ "$ACP_TOOL_CALLS" -gt 0 ]; then
      test_pass "$ACP_TOOL_CALLS tool call(s)"
    elif echo "$ACP_RESPONSE" | grep -qi "no.*mcp.*tools\|no.*tools.*discovered"; then
      test_fail "MCP tools not discovered — discovery failed silently despite valid binary"
    elif echo "$ACP_RAW" | grep -q "tool_call"; then
      test_pass "tool_call event in stream"
    else
      test_warn "model didn't call tool (flaky — depends on model compliance)"
    fi
    acp_stop
  else
    test_skip "bridge or mcp binary unavailable"
  fi
fi

# --------------------------------------------------------------------------
# Test: MCP tool invocation returns data
# --------------------------------------------------------------------------
if test_start "MCP tool returns real data"; then
  if [ "$MCP_AVAILABLE" = true ] && [ "$BRIDGE_AVAILABLE" = true ]; then
    BRIDGE_TOKEN=$(cat "$BRIDGE_TOKEN_FILE")
    MCP_PARAMS="\"mcpServers\":[{\"name\":\"nunchi\",\"transport\":{\"type\":\"stdio\",\"command\":\"$NUNCHI_MCP\",\"args\":[],\"env\":{\"BRIDGE_TOKEN\":\"$BRIDGE_TOKEN\",\"BRIDGE_URL\":\"$BRIDGE_URL\"}}}]"

    acp_start
    acp_session_new "sonnet" "$MCP_PARAMS"
    acp_prompt "Call nunchi_tools_list. Tell me the exact type of the first tile." 60 3
    if echo "$ACP_RESPONSE" | grep -qi "code-agent\|widget\|custom"; then
      test_pass "got tile data"
    else
      test_warn "response didn't mention tile types: $(echo "$ACP_RESPONSE" | head -c 100)"
    fi
    acp_stop
  else
    test_skip "bridge or mcp binary unavailable"
  fi
fi

# --------------------------------------------------------------------------
print_section "MCP Error Cases (Issue #01)"

# --------------------------------------------------------------------------
# Test: nonexistent binary → should error (currently silent)
# --------------------------------------------------------------------------
if test_start "nonexistent MCP binary → structured error"; then
  acp_start
  MCP_BAD="\"mcpServers\":[{\"name\":\"bad\",\"transport\":{\"type\":\"stdio\",\"command\":\"/nonexistent/path/to/mcp\",\"args\":[],\"env\":{}}}]"
  acp_session_new "sonnet" "$MCP_BAD"

  if [ -z "$ACP_SESSION_ID" ]; then
    test_pass "session creation failed (correct behavior)"
  else
    # Check if warnings array contains the MCP error
    WARNINGS=$(echo "$ACP_NEW_RESULT" | python3 -c "
import sys,json
d=json.load(sys.stdin)
w=d.get('result',{}).get('warnings',[])
print('|'.join(w)) if w else print('')
" 2>/dev/null) || true
    if echo "$WARNINGS" | grep -qi "not found\|mcp.*error\|command.*not"; then
      test_pass "warning in session/new: $(echo "$WARNINGS" | head -c 80)"
    else
      # Session created with no warning — this is the bug
      acp_prompt "What tools do you have?" 30
      if echo "$ACP_RESPONSE" | grep -qi "no.*tools\|no.*mcp"; then
        test_fail "session created with /nonexistent/path/to/mcp — no error or warning (BUG #01)"
      else
        test_fail "session succeeded with nonexistent binary, got response: $(echo "$ACP_RESPONSE" | head -c 80)"
      fi
    fi
  fi
  acp_stop
fi

# --------------------------------------------------------------------------
# Test: MCP binary exits immediately → should error
# --------------------------------------------------------------------------
if test_start "MCP binary that exits → structured error"; then
  # Create a script that exits immediately
  TMP_MCP=$(mktemp /tmp/bad_mcp_XXXXXX)
  cat > "$TMP_MCP" << 'EOF'
#!/bin/bash
exit 1
EOF
  chmod +x "$TMP_MCP"

  acp_start
  MCP_EXIT="\"mcpServers\":[{\"name\":\"crashy\",\"transport\":{\"type\":\"stdio\",\"command\":\"$TMP_MCP\",\"args\":[],\"env\":{}}}]"
  acp_session_new "sonnet" "$MCP_EXIT"

  if [ -z "$ACP_SESSION_ID" ]; then
    test_pass "session creation failed (correct)"
  else
    acp_prompt "What tools do you have?" 30
    if echo "$ACP_RESPONSE" | grep -qi "no.*tools\|no.*mcp"; then
      test_fail "session created with crashing binary — no error surfaced (BUG #01: silent MCP failure)"
    else
      test_fail "session succeeded with immediately-exiting MCP binary, got response: $(echo "$ACP_RESPONSE" | head -c 80)"
    fi
  fi
  acp_stop
  rm -f "$TMP_MCP"
fi

# --------------------------------------------------------------------------
# Test: MCP binary with wrong bridge token
# --------------------------------------------------------------------------
if test_start "MCP with invalid bridge token"; then
  if [ "$MCP_AVAILABLE" = true ]; then
    acp_start
    MCP_BADTOKEN="\"mcpServers\":[{\"name\":\"nunchi\",\"transport\":{\"type\":\"stdio\",\"command\":\"$NUNCHI_MCP\",\"args\":[],\"env\":{\"BRIDGE_TOKEN\":\"wrong-token-12345\",\"BRIDGE_URL\":\"$BRIDGE_URL\"}}}]"
    acp_session_new "sonnet" "$MCP_BADTOKEN"
    acp_prompt "Call nunchi_tools_list" 30
    if [ "$ACP_TOOL_CALLS" -gt 0 ]; then
      # Tool was called but should have gotten auth error
      if echo "$ACP_RAW" | grep -qi "401\|unauthorized\|auth"; then
        test_pass "tool call failed with auth error"
      else
        test_warn "tool called but unclear if auth was checked"
      fi
    else
      test_warn "no tool call (might be discovery failure or model choice)"
    fi
    acp_stop
  else
    test_skip "nunchi-mcp binary unavailable"
  fi
fi

# --------------------------------------------------------------------------
print_summary
echo "$TESTS_PASSED $TESTS_FAILED $TESTS_SKIPPED" > "$LOG_DIR/mcp-results.txt"
exit $TESTS_FAILED
