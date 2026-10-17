+++
id = "gap-1db29e"
kind = "gap"
title = "The agents' network sandbox does not confine Mach IPC, so sandboxed code can have LaunchServices or another app fetch a URL"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-16
updated = 2026-10-16
source = "gap-0bd49a worker (claude-agent), 2026-10-16"
discovered_from = "gap-0bd49a: families/common/sandbox.py states the Mach IPC limit"
anchors = ["benchmarks/viabilitybench/families/common/sandbox.py::profile", "benchmarks/viabilitybench/driver/test_sandbox_net.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-0bd49a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_sandboxed_code_cannot_send_apple_events_or_open_urls' benchmarks/viabilitybench/driver/test_sandbox_net.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_sandbox_net.py -k test_sandboxed_code_cannot_send_apple_events_or_open_urls -q"
+++

## Problem

The agents' network sandbox (gap-0bd49a, `families/common/sandbox.py`) confines a process's own sockets:
`(deny network-outbound)` plus the loopback ports a rule admits. Sandboxed code can still ask an unconfined process
to fetch a URL for it, over Mach IPC. For example:

- `open -g https://...`: LaunchServices hands the URL to the default browser;
- `osascript -e 'tell application "Terminal" to do script "curl ... > /tmp/x"'`: an AppleEvent to an app that
  isn't sandboxed;
- a background `URLSession` (nsurlsessiond).

The agent then reads the result from a path it may read, such as `/tmp`. The module docstring states the limit ("a
system service the code asks over Mach IPC is not confined by it"), but nothing closes it.

## Why it matters

This is the pilot benchmark's validity (epic spec-567e52), and the same contamination route gap-0bd49a closed for
sockets: fetching the public repository's truth suites. AppleEvents to another app also need the responsible
process's Automation permission (TCC). Whether a benchmark session has it depends on the terminal or app that
started `vb run`.

## Where

- `benchmarks/viabilitybench/families/common/sandbox.py::profile`: builds the sandbox-exec profile for every agent
  process and for the census's runs of agent code.

## Current state

At gap-0bd49a's close, the profile is `(version 1) (allow default)`, plus file denies, plus the network rule. On
macOS 26.4, `(deny appleevent-send)` and `(deny lsopen)` both parse under `sandbox-exec`. Whether they stop the routes
above, and whether they break Claude Code, Codex, Roko, git, cargo or node inside the sandbox, has not been checked.
The Claude Code arm must keep its keychain read through `security`, which uses Mach IPC to securityd.

## Plan

1. Add `(deny appleevent-send)` and `(deny lsopen)` to the profile under every network rule. Consider
   `mach-lookup` denies for URL-loading services such as nsurlsessiond, and keep securityd reachable.
2. Re-run the arms' offline tests and the verifier CI (`ci/verify_verifiers.py`) to check that nothing they use
   breaks.
3. Add `test_sandboxed_code_cannot_send_apple_events_or_open_urls` in `driver/test_sandbox_net.py`. Under the
   sandbox, `osascript` and `open` must fail with a sandbox denial. Never run the unconfined control against a real
   app on the operator's machine: it would open a browser or prompt for Automation access. Use a target that has no
   side effect, or skip the control and assert on the denial message.

## Done when

- [ ] Sandboxed agent code cannot hand a URL to LaunchServices or send an AppleEvent to another app.
- [ ] The `[[verify]]` command passes.

## Notes

- Found while closing gap-0bd49a. That item's Done-when covers the agent's own connections, which this route
  doesn't use.
