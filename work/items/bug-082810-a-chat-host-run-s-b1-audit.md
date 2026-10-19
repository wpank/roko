+++
id = "bug-082810"
kind = "bug"
title = "A chat-host run's B1 audit dispatches its suite author to the person's Claude CLI (roko-audit-b1 on claude_cli / claude-sonnet-4-6), though a chat host's work must use only tool-loop API models"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/audit"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "round-v7-hermes-pilot#C-p0-run2"
discovered_from = "agent:hermes-pilot (round v7 phase 0, run e578cf71, after T1 passed)"
anchors = ["crates/roko-cli/src/audit/b1.rs::FactoryAuthor"]
links = { depends_on = [], blocks = [], related = ["gap-3d37e8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_chat_host_runs_audit_uses_only_tool_loop_models' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_chat_host_runs_audit_uses_only_tool_loop_models"
+++

## Problem

Round v7, Undertow phase 0, run `e578cf71`, a run a Hermes chat started (roko f33fd217c).

- T1 passed on mid (glm-4.7). Twenty seconds later roko started the B1 audit's suite author:

  ```
  roko_agent::claude_cli_agent: agent started agent=roko-audit-b1 pid=6941 timeout_s=120
  roko_agent::claude_cli_agent: agent failed agent=roko-audit-b1 exit_code=127 elapsed_s=0
  ```

- `costs.jsonl` gained a row: model `claude-sonnet-4-6`, provider `claude_cli`, role `audit`, task T1, 0 tokens,
  `success: false`.
- It failed only because the round's harness fences off the `claude` binary. On a person's machine, a run started
  from chat would have launched their own Claude Code login, outside the run's cap and outside the data-model
  boundary.

## Why it matters

A chat host's work goes only to models roko's own tool loop runs (gap-cf427e, bug-a0432b): no person's CLI agent or
login, and every call under the run's cap and the boundary. The page and the plugin promise the person this ("not for
chat runs: its agent runs its own tools"). The audit breaks it silently. It also chooses a CLI agent for a job
(writing a test suite) that roko's own tool loop can do.

## Where

- `crates/roko-cli/src/audit/b1.rs`: `FactoryAuthor` dispatches the suite author through the shared agent factory,
  with whatever model the audit config chose.
- The B1 author is "a configured model of another family than the implementer's" (`AuditConfig::family_of`). With
  glm-4.7 implementing, it picked claude-sonnet-4-6, the ladder's top rung, a `claude_cli` model.

## Plan

1. For a run whose origin is a chat host, choose the B1 author only among the models `RokoConfig::tool_loop_models`
   lists, still of another family than the implementer's (gpt-5.4-mini for a glm-4.7 implementer here). With none,
   B1 is null, as decision 7102 already allows.
2. Apply the same filter to every audit phase that calls a model (A2's rerun is check-only; B2 and B3 if they call
   one).

## Done when

A chat-host run's audit never dispatches to a CLI provider. Checked by the test in `[[verify]]`.

## Notes

- Evidence:
  - `~/dev/undertow/.roko/learn/costs.jsonl`: the `role: audit` row at 2026-10-19T15:10:27Z;
  - `.roko/roko.log.2026-10-19`: `roko-audit-b1`.
- Related:
  - gap-cf427e, bug-a0432b: chat hosts use only tool-loop models;
  - gap-3d37e8: the B1 hidden suites.
