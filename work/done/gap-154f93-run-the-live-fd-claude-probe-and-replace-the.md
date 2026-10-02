+++
id = "gap-154f93"
kind = "gap"
title = "Run the live fd_claude probe and replace the invented modelUsage fixture with its saved output"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "509e3e807"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-ccarm's report on gap-c4f364, merged in 157f1d434)"
anchors = ["benchmarks/viabilitybench/driver/run_cli.py", "benchmarks/viabilitybench/driver/test_run_cli.py", "benchmarks/viabilitybench/arms/fd_claude.toml"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-c4f364", "gap-8be530", "gap-f253cf"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/driver/testdata/fd_claude_probe.json && grep -qw 'def test_the_recorded_probe_parses' benchmarks/viabilitybench/driver/test_run_cli.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_cli.py -k test_the_recorded_probe_parses -q"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T18:37:15Z"
commit = "509e3e807"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-02T16:02:41Z"
forced = false
evidence = "Gate 3a on work/backlog-batch-3 (merged into main as 509e3e807; main differs from the gated tree only in work/ files): cargo check --workspace --tests, clippy -D warnings, nextest --lib 11,514 passed over 10 crates, golden-path canaries 13/13 (7 targets), roko-agent sse_replay 1/1; every [[verify]] passes (lib tests named in each verify passed; integration tests run by target; static parts rc=0)."
+++

## Problem

gap-c4f364 built the `fd_claude` arm (`driver/run_cli.py`, merged in 157f1d434) and was closed without its live probe. Its notes say: "Not done: the live probe. No real `claude` ran in this push." Until the probe runs, two things rest on guesses:

- **The cost fixture.** The `result` event in `driver/test_run_cli.py` (:38 onward) was written by hand in "Claude Code 2.1.282's stream-json shape". The U′ headline sums `modelUsage` (run_cli.py:44, :415), so a wrong field name would null or miscount every fd_claude cost.
- **The per-task keychain login.** run_cli.py points `CLAUDE_CONFIG_DIR` at a per-task directory and sets `CLAUDE_SECURESTORAGE_CONFIG_DIR` to empty (:315), to keep the default keychain entry name. The variable is undocumented (:32), so only a live session shows whether a per-task config directory still finds the subscription login.

## Why it matters

Pilot benchmark (epic spec-567e52): fd_claude is the frontier baseline of H1 and H2. Its cost and its isolation must be observed, not assumed.

## Where

`driver/run_cli.py`: the `probe` subcommand (:445), the environment (:117, :315) and the `modelUsage` parser (:415). The fixture is in `driver/test_run_cli.py`.

## Current state

At e43d3a033 the probe exists and has never run against a real `claude`. Every fd_claude test uses the invented fixture and a fake `claude`.

## Plan

1. Off-hours, run `benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/driver/run_cli.py probe --arm fd_claude --allow-network`. It costs a few cents under the subscription (D42).
2. Keep its JSON: the `system`/`init` event and the `result` event, with the CLI version. Scrub account and session identifiers, and save the result as `driver/testdata/fd_claude_probe.json`.
3. Add `test_the_recorded_probe_parses`. The parser reads the saved `modelUsage`, and the init event shows the pinned model and no MCP servers. Drop the invented fixture, or keep it only where the saved one can't serve.
4. Record whether the per-task config directory found the login, and change the environment if it didn't.

## Done when

- [ ] A real probe's output is saved in the repo, and the fd_claude tests parse it.
- [ ] The `[[verify]]` command passes.

## Notes

- This makes one real call on the subscription. Confirm D42 (subscription terms) first.
- The probe is also where to confirm that the arm has no web tools (gap-f253cf).

## Progress

2026-10-02 (wave 3, static worker on `work/gap-625195-2`): implemented at 0889b3bb0. Will approved the one
subscription run on 2026-10-02 (D42); the coordinator closes the item.

- The probe already ran behind the arm's egress proxy and sandbox (`egress.EgressProxy`, PK20), so `run_cli.py` needed
  no egress change. `agent_env.build` passes the operator's USER and LOGNAME (here both set).
- Attempt 1 ended "Not logged in · Please run /login" after 52 ms, before any model call ($0, empty `modelUsage`). The
  entry name was right: Claude Code 2.1.282 reads `security find-generic-password -a "$USER" -w -s "Claude
  Code-credentials"`, and `CLAUDE_SECURESTORAGE_CONFIG_DIR=""` drops the hash suffix. The per-task HOME was the cause:
  macOS finds the login keychain through HOME, and `security default-keychain` under the session's HOME finds none.
  So the per-task config directory did **not** find the login as the arm stood.
- Environment change: with `credentials = "keychain"` the session's `.vb-bin` holds a `security` wrapper that runs
  /usr/bin/security under the operator's HOME; Claude Code finds `security` on its PATH, and the session keeps its own
  HOME. The agent's shell can run the wrapper too, as it could already run /usr/bin/security on the login keychain by
  path (the same-uid limit in `agent_env`).
- Attempt 2, the one allowed retry, passed every check: signed in, `claude-opus-5-5`, no MCP server, no web tool, no
  memory, and only the plugins Claude Code ships (`agents-md@builtin`, `telemetry@builtin`; the `no_plugins` check now
  ignores `path = "builtin"`). It answered `READY` in 1 turn. U′ = R = $0.0389196 API-equivalent, $0 billed.
- Hosts: 8 CONNECTs to api.anthropic.com:443 admitted; one to http-intake.logs.us5.datadoghq.com:443 (Claude Code's
  log intake) refused, which the session did not need. The default allowlist stands; the arm file says so.
- `driver/testdata/fd_claude_probe.json` is the probe's report with session ids, paths and the user name scrubbed.
  `test_the_recorded_probe_parses` reads its `modelUsage` and init event. The invented RESULT stays for the two-model
  and cache-TTL cases, and the test checks its field names against the recorded event (they are a subset).
- Verify passes in the bench venv; the whole bench suite passed (467 passed, 7 skipped).
