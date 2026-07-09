#!/usr/bin/env python3
"""Inspect the last few events in a stream-json log file.

Usage: inspect-tail.py <log_file> [num_events]
"""
import sys
import json

path = sys.argv[1]
n = int(sys.argv[2]) if len(sys.argv) > 2 else 10


def trunc(s, n=80):
    s = str(s).replace("\n", " ")
    return s if len(s) <= n else s[:n-3] + "..."


# Read last N lines that look like JSON events (start with '{')
with open(path, "r", errors="replace") as f:
    lines = f.readlines()

events = [l.strip() for l in lines if l.strip().startswith("{")]
print(f"Total JSON events in log: {len(events)}")
print(f"Showing last {min(n, len(events))} events:")
print()

for line in events[-n:]:
    try:
        ev = json.loads(line)
        etype = ev.get("type", "?")
        if etype == "stream_event":
            se = ev.get("event", {})
            st = se.get("type", "?")
            if st == "content_block_delta":
                d = se.get("delta", {})
                dtype = d.get("type", "?")
                if dtype == "thinking_delta":
                    print(f"  thinking: " + repr(trunc(d.get("thinking", ""))))
                elif dtype == "text_delta":
                    t = d.get("text", "")
                    if t.strip():
                        print(f"  text: " + repr(trunc(t)))
                elif dtype == "input_json_delta":
                    pj = d.get("partial_json", "")
                    if pj.strip():
                        print(f"  json_delta: " + repr(trunc(pj, 60)))
            elif st == "content_block_start":
                cb = se.get("content_block", {})
                print(f"  block_start: type={cb.get('type', '?')}")
            elif st == "content_block_stop":
                print(f"  block_stop")
            elif st == "message_start":
                print(f"  message_start")
            elif st == "message_stop":
                print(f"  message_stop")
            else:
                print(f"  stream.{st}")
        elif etype == "assistant":
            msg = ev.get("message", {})
            for b in msg.get("content", []):
                btype = b.get("type", "?")
                if btype == "tool_use":
                    name = b.get("name", "?")
                    inp = trunc(json.dumps(b.get("input", {})), 100)
                    print(f"  TOOL: {name} {inp}")
                elif btype == "thinking":
                    t = b.get("thinking", "")
                    print(f"  THINKING block ({len(t)} chars): " + repr(trunc(t)))
                elif btype == "text":
                    t = b.get("text", "")
                    print(f"  text block: " + repr(trunc(t)))
        elif etype == "user":
            msg = ev.get("message", {})
            for b in msg.get("content", []):
                if b.get("type") == "tool_result":
                    content = b.get("content", "")
                    if isinstance(content, list):
                        content = " ".join(
                            c.get("text", "") if isinstance(c, dict) else str(c)
                            for c in content
                        )
                    is_error = b.get("is_error", False)
                    prefix = "ERROR" if is_error else "result"
                    print(f"  {prefix}: " + repr(trunc(content)))
        elif etype == "result":
            cost = ev.get("total_cost_usd", 0)
            is_err = ev.get("is_error", False)
            print(f"  RESULT: error={is_err} cost=${cost:.4f}")
        else:
            print(f"  {etype}")
    except Exception as e:
        print(f"  PARSE_ERR: {trunc(str(e))}")
