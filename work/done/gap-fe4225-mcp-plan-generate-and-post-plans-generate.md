+++
id = "gap-fe4225"
kind = "gap"
title = "MCP plan_generate and POST /plans/generate take the request only as prompt text, so a chat must copy a workspace spec into it word for word (Undertow phase 2: all 21 checks-table lines arrived as '|| …'); add a spec_path read by serve, as the CLI's --from-file does"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-serve/mcp", "roko-serve/plans"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "a1d12fc1c"
source = "round-v7-hermes-pilot#E-p2-plan"
discovered_from = "agent:hermes-pilot (round v7 phase 2, Hermes session 20261020_114839_c72166)"
anchors = ["crates/roko-serve/src/routes/mcp.rs::generate_plan", "crates/roko-serve/src/routes/plans/authoring.rs::generate_plan"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_generate_with_a_spec_path_plans_from_the_file' crates/roko-serve/src && cargo test -p roko-serve --lib -- plan_generate_with_a_spec_path_plans_from_the_file"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T16:18:11Z"
commit = "8a9127110"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Merged 1ae6cab88 (work/bug-16b798: 8a9127110). MCP plan_generate and POST /api/plans/generate take a spec_path; serve reads that workspace file (inside the workspace, at most 1 MiB, as `roko plan generate --from-file` does) and plans from its exact text, with any prompt's instructions after it; prompt is optional with a spec_path, and the tool's description tells the model to pass a spec's path, never its text. A path outside the workspace, a missing file or a directory is refused, and so is a call with neither. Verify at 1ae6cab88: plan_generate_with_a_spec_path_plans_from_the_file passes (the runtime gets the spec's exact bytes, a `| P2.01 | … |` table row untouched, then `---` and the prompt; `../other.md`, an absolute path outside, a missing file and a directory are refused), with mcp_plan_generate_takes_a_spec_path (the schema lists spec_path and requires nothing; an outside path and an empty call are refused). The Hermes plugin side (P-6) is plugin-wow's, told the field name. Branch: as bug-16b798."
+++

## Problem

The MCP tool `plan_generate` (and `POST /plans/generate`) takes the request only as `prompt` text. To plan from a spec
the person has written in the workspace, a chat must read the file and copy it into `prompt` word for word. In round v7
that copying is where fidelity slips:
- Baseline and phase 0: the chat reworded or summarised the request until it was told to pass the spec word for word.
- Phase 2 (5de243ec8, Hermes chat on gpt-5.4-mini, session `20261020_114839_c72166`): the 14,474-character spec reached
  roko as 14,572 characters. All 21 lines of the acceptance-checks table began with `||` instead of `|`, because the
  chat left one character of read_file's `N|` line prefix in place. roko's check parser still found the 20 checks, but
  it was close.

The copying also costs the chat about 3,800 output tokens for each plan (14.6 K characters), and its time.

`roko plan generate --from-file` already plans from a file on the CLI. The serve and MCP path has no equivalent.

## Why it matters

The pitch is that the person writes a spec with verifiable checks, and roko plans from it. The spec should reach the
planner byte for byte, whatever chat model is in between.

## Where

- `crates/roko-serve/src/routes/mcp.rs`: the `plan_generate` tool's schema and `generate_plan`.
- `crates/roko-serve/src/routes/plans/authoring.rs`: `generate_plan` (`POST /plans/generate`).
- The Hermes plugin's `roko_plan_generate` would then pass the path through (plugin follow-up).

## Plan

Add an optional `spec_path` to `plan_generate` and `POST /plans/generate`. It must be a path inside the workspace, read
by serve, and it follows the CLI's `--from-file` rules. When it is given, `prompt` is optional and carries only extra
instructions. Refuse paths outside the workspace.

## Done when

`plan_generate` with `spec_path` plans from the file's exact bytes. Checked by the test in `[[verify]]`.

## Notes

- Evidence: `tmp/hermes-roko-v7/pilot/evidence/p2-plan/chat-tool-call.json` (the prompt as the chat passed it).
