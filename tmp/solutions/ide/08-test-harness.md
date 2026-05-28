# Test Harness — Reproducible ACP Testing

## Overview

This document provides a FIFO-based test harness for testing `roko acp` behavior.
All tests in this folder were validated using this approach.

## Core Pattern

```bash
#!/bin/bash
# FIFO-based ACP test harness
FIFO_IN=$(mktemp -u /tmp/roko_test_XXXXXX)
FIFO_OUT=$(mktemp -u /tmp/roko_test_XXXXXX)
mkfifo "$FIFO_IN" "$FIFO_OUT"

ROKO=/path/to/roko
CONFIG=/path/to/roko.toml

$ROKO acp --quiet --no-serve --config "$CONFIG" < "$FIFO_IN" > "$FIFO_OUT" 2>/tmp/test_stderr.log &
ACP_PID=$!
exec 3>"$FIFO_IN"  # Write handle
exec 4<"$FIFO_OUT"  # Read handle

# Helper: read until we find a line matching a pattern
read_until() {
  local pattern="$1"
  local timeout="${2:-15}"
  while IFS= read -r -t "$timeout" line <&4; do
    echo "$line"
    if echo "$line" | grep -q "$pattern"; then
      return 0
    fi
  done
  return 1
}

# Helper: drain buffered notifications
drain() {
  while IFS= read -r -t 3 line <&4; do :; done
}

# Helper: extract session ID from session/new response
get_session_id() {
  echo "$1" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['result']['sessionId'])"
}

# --- TEST CODE HERE ---

# Cleanup
exec 3>&-
exec 4<&-
kill $ACP_PID 2>/dev/null
rm -f "$FIFO_IN" "$FIFO_OUT"
```

## Test: Basic Session Flow

```bash
#!/bin/bash
source /path/to/harness.sh  # Or inline the pattern above

# Create session
echo '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{"model":"sonnet"}}' >&3
RESPONSE=$(read_until '"sessionId"')
SESSION_ID=$(get_session_id "$RESPONSE")
drain

# Send prompt
echo "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":2,\"params\":{\"sessionId\":\"$SESSION_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"Say exactly: TEST_PASS\"}]}}" >&3

# Collect streaming response
FULL=""
while IFS= read -r -t 30 line <&4; do
  TEXT=$(echo "$line" | python3 -c "
import sys,json
d=json.load(sys.stdin)
u=d.get('params',{}).get('update',{})
if u.get('sessionUpdate')=='agent_message_chunk':
  print(u['content']['text'], end='')
" 2>/dev/null)
  FULL="$FULL$TEXT"
  if echo "$line" | grep -q '"id":2'; then break; fi
done

if echo "$FULL" | grep -q "TEST_PASS"; then
  echo "PASS: Got expected response"
else
  echo "FAIL: Expected TEST_PASS, got: $FULL"
fi
```

## Test: MCP Tool Discovery

```bash
# Requires: bridge running on localhost:6678, bridge-token file exists
NUNCHI_MCP=/path/to/nunchi-mcp  # debug or release binary
BRIDGE_TOKEN=$(cat ~/.nunchi/bridge-token)

# Session with MCP
cat >&3 << EOF
{"jsonrpc":"2.0","method":"session/new","id":1,"params":{
  "model":"sonnet",
  "mcpServers":[{
    "name":"nunchi",
    "transport":{
      "type":"stdio",
      "command":"$NUNCHI_MCP",
      "args":[],
      "env":{"BRIDGE_TOKEN":"$BRIDGE_TOKEN","BRIDGE_URL":"http://127.0.0.1:6678"}
    }
  }]
}}
EOF

# Get session, drain
RESPONSE=$(read_until '"sessionId"')
SESSION_ID=$(get_session_id "$RESPONSE")
drain

# Ask to use tools
echo "{\"jsonrpc\":\"2.0\",\"method\":\"session/prompt\",\"id\":2,\"params\":{
  \"sessionId\":\"$SESSION_ID\",
  \"prompt\":[{\"type\":\"text\",\"text\":\"Call nunchi_tools_list and tell me how many tools there are.\"}]
}}" >&3

# Check for tool_call in stream
TOOL_CALLED=false
while IFS= read -r -t 60 line <&4; do
  if echo "$line" | grep -q '"sessionUpdate":"tool_call"'; then
    TOOL_CALLED=true
  fi
  if echo "$line" | grep -q '"id":2'; then break; fi
done

if [ "$TOOL_CALLED" = true ]; then
  echo "PASS: MCP tool was called"
else
  echo "FAIL: No tool call observed"
fi
```

## Test: Error Cases

```bash
# Wrong session ID
echo '{"jsonrpc":"2.0","method":"session/prompt","id":99,"params":{"sessionId":"sess_FAKE","prompt":[{"type":"text","text":"hi"}]}}' >&3
ERROR=$(read_until '"id":99')
if echo "$ERROR" | grep -q '"code":-32000'; then
  echo "PASS: Wrong session ID returns -32000"
fi

# Wrong method name
echo '{"jsonrpc":"2.0","method":"message/send","id":100,"params":{}}' >&3
ERROR=$(read_until '"id":100')
if echo "$ERROR" | grep -q '"code":-32601'; then
  echo "PASS: Unknown method returns -32601"
fi
```

## Common Pitfalls

| Pitfall | Solution |
|---------|----------|
| Session ID changes each run | Always extract from session/new response |
| Empty lines in stream | Skip empty lines before JSON.parse |
| Notifications arrive between request and response | Read until you see matching `id` |
| `head -n 1` misses notifications | Use while-loop reader |
| Shell arithmetic fails on JSON | Use python3 for JSON extraction |
| Process hangs after test | Always close FDs and kill PID in cleanup |

## Required for MCP Tests

Before running MCP tests, ensure:
1. The Nunchi IDE is running (provides the HTTP bridge on :6678)
2. `~/.nunchi/bridge-token` exists and is current
3. The nunchi-mcp binary is built (`cd nunchi-mcp && cargo build`)
4. The binary path in tests matches actual location (debug vs release)
