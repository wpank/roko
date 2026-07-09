#!/usr/bin/env python3
"""
watch-log-formatter.py — parse claude CLI stream-json output and pretty-print.

Reads stdin line-by-line. Each line is either:
  1. A header line from run-migration.sh (e.g., "=== Migration run: ... ===")
  2. A JSON event from `claude --output-format stream-json`
  3. A `==>` prefix line from `tail -f multiple-files` mode (--multi)

For each event, prints a single human-readable line showing:
  - Timestamp (hh:mm:ss)
  - Event type (Read, Write, Edit, Bash, text chunk, tool_result, result)
  - Relevant details (file path, byte count, text snippet, cost)

Usage:
    tail -f log.log | python3 watch-log-formatter.py
    tail -f log1.log log2.log | python3 watch-log-formatter.py --multi
"""

import json
import sys
import time
import re
from datetime import datetime

ANSI = {
    "reset": "\033[0m",
    "bold": "\033[1m",
    "dim": "\033[2m",
    "red": "\033[31m",
    "green": "\033[32m",
    "yellow": "\033[33m",
    "blue": "\033[34m",
    "magenta": "\033[35m",
    "cyan": "\033[36m",
}


def c(color, text):
    if sys.stdout.isatty():
        return f"{ANSI.get(color, '')}{text}{ANSI['reset']}"
    return text


def now_str():
    return datetime.now().strftime("%H:%M:%S")


def truncate(text, n=100):
    text = str(text).replace("\n", " ")
    if len(text) > n:
        return text[: n - 3] + "..."
    return text


def format_tool_call(tool_name, tool_input):
    """Format a tool invocation as a single line."""
    tool_name = tool_name or "?"
    if not isinstance(tool_input, dict):
        return f"{tool_name} {truncate(tool_input, 80)}"

    if tool_name == "Read":
        path = tool_input.get("file_path", "?")
        offset = tool_input.get("offset")
        limit = tool_input.get("limit")
        range_suffix = ""
        if offset is not None or limit is not None:
            range_suffix = f" [lines {offset or 0}+{limit or '*'}]"
        return f"{c('cyan', 'Read')}     {path}{range_suffix}"

    if tool_name == "Write":
        path = tool_input.get("file_path", "?")
        content = tool_input.get("content", "")
        size = len(content) if isinstance(content, str) else 0
        return f"{c('green', 'Write')}    {path}  ({size} bytes)"

    if tool_name == "Edit":
        path = tool_input.get("file_path", "?")
        return f"{c('yellow', 'Edit')}     {path}"

    if tool_name == "Bash":
        cmd = tool_input.get("command", "?")
        desc = tool_input.get("description", "")
        suffix = f"  # {desc}" if desc else ""
        return f"{c('magenta', 'Bash')}     {truncate(cmd, 80)}{suffix}"

    if tool_name == "Glob":
        return f"{c('blue', 'Glob')}     {tool_input.get('pattern', '?')}"

    if tool_name == "Grep":
        pattern = tool_input.get("pattern", "?")
        path = tool_input.get("path", "")
        return f"{c('blue', 'Grep')}     {truncate(pattern, 40)}  {path}"

    # Unknown tool
    return f"{c('dim', tool_name)} {truncate(json.dumps(tool_input), 100)}"


def format_event(event):
    """Return a one-line summary string for a stream-json event, or None to skip."""
    if not isinstance(event, dict):
        return None

    etype = event.get("type", "")

    # Initial system event
    if etype == "system" and event.get("subtype") == "init":
        model = event.get("model", "?")
        session = event.get("session_id", "")[:8]
        return c("dim", f"[session {session}] model: {model}")

    # Assistant message (contains text and/or tool_use blocks)
    if etype == "assistant":
        msg = event.get("message", {})
        content = msg.get("content", [])
        lines = []
        if isinstance(content, list):
            for block in content:
                if not isinstance(block, dict):
                    continue
                btype = block.get("type", "")
                if btype == "text":
                    text = block.get("text", "").strip()
                    if text:
                        lines.append(f"{c('bold', 'text')}     {truncate(text, 200)}")
                elif btype == "tool_use":
                    tool_name = block.get("name", "?")
                    tool_input = block.get("input", {})
                    lines.append(format_tool_call(tool_name, tool_input))
        return "\n".join(lines) if lines else None

    # User message (tool results)
    if etype == "user":
        msg = event.get("message", {})
        content = msg.get("content", [])
        if isinstance(content, list):
            results = []
            for block in content:
                if not isinstance(block, dict):
                    continue
                if block.get("type") == "tool_result":
                    is_error = block.get("is_error", False)
                    result_content = block.get("content", "")
                    if isinstance(result_content, list):
                        text_parts = [
                            b.get("text", "")
                            for b in result_content
                            if isinstance(b, dict)
                        ]
                        result_content = " ".join(text_parts)
                    size = len(str(result_content))
                    if is_error:
                        results.append(
                            c(
                                "red",
                                f"  ↳ error: {truncate(result_content, 120)}",
                            )
                        )
                    else:
                        results.append(c("dim", f"  ↳ ok ({size} bytes)"))
            return "\n".join(results) if results else None

    # Final result event
    if etype == "result":
        subtype = event.get("subtype", "")
        cost = event.get("total_cost_usd", 0)
        duration = event.get("duration_ms", 0) / 1000
        turns = event.get("num_turns", 0)
        result_text = event.get("result", "")
        is_error = event.get("is_error", False)
        status_color = "red" if is_error else "green"
        status = "FAILED" if is_error else "DONE"
        summary = (
            f"{c('bold', c(status_color, status))}  "
            f"${cost:.4f}  {duration:.0f}s  {turns} turns"
        )
        if result_text:
            summary += f"\n       → {truncate(result_text, 200)}"
        return summary

    # Stream event (partial message chunks)
    if etype == "stream_event":
        ev = event.get("event", {})
        ev_type = ev.get("type", "")
        if ev_type == "content_block_delta":
            delta = ev.get("delta", {})
            dtype = delta.get("type", "")
            if dtype == "text_delta":
                text = delta.get("text", "")
                if text.strip():
                    # Skip noisy partial text deltas in non-verbose mode
                    return None
            elif dtype == "input_json_delta":
                return None
        return None

    # Unknown event
    return None


def main():
    multi = "--multi" in sys.argv
    current_file = None

    for raw_line in sys.stdin:
        line = raw_line.rstrip("\n")
        if not line.strip():
            continue

        # In --multi mode, tail -f emits "==> path <==" when switching files
        if multi and line.startswith("==>") and line.endswith("<=="):
            current_file = line[4:-4].strip()
            base = current_file.split("/")[-1].replace(".log", "")
            print()
            print(c("bold", c("magenta", f"━━━ {base} ━━━")))
            continue

        # Runner header lines (from spawn.sh) start with "===" or are short
        if line.startswith("==="):
            print(c("dim", line))
            continue
        if line.startswith("Error:") or line.startswith("error:"):
            print(c("red", line))
            continue

        # Try to parse as JSON
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            # Not JSON — print dimmed for debugging
            if line.strip():
                print(c("dim", f"  {truncate(line, 200)}"))
            continue

        formatted = format_event(event)
        if formatted is None:
            continue

        ts = now_str()
        prefix = f"[{ts}] "
        for ln in formatted.split("\n"):
            print(f"{c('dim', prefix)}{ln}")
        sys.stdout.flush()


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(0)
    except BrokenPipeError:
        sys.exit(0)
