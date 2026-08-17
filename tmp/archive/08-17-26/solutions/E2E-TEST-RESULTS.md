# End-to-End Test Results

Date: 2026-04-28
Workspace: `/tmp/roko-e2e-1777398304`
Binary: release build from `wp-arch2` branch

## Summary

| Command | Result | Severity |
|---|---|---|
| `roko init` | PASS | — |
| `roko status` | PASS (but cost is $-0.0000) | Low |
| `roko doctor` | PASS | — |
| `roko learn all` | Shows "empty" despite having data | Medium |
| `roko config providers list` | PASS (identifies available providers) | — |
| `roko config models list` | PASS | — |
| `roko "prompt"` (one-shot) | BROKEN: ignores --model, no tools, no context | Critical |
| `roko` (interactive, piped) | BROKEN: tries anthropic_api, fails | Critical |
| `roko run "prompt"` | BROKEN pre-migrate: wrong provider | Critical |
| `roko run "prompt"` (post-migrate) | PARTIAL: uses Claude CLI but gate always fails | High |
| `roko config migrate` | PASS (fixes provider routing) | — |
| `roko prd idea` | PASS | — |
| `roko prd list` | PASS | — |
| `roko prd draft new` | PASS: uses Claude CLI, tools, workspace | — |
| `roko prd plan` | PASS: generates tasks.toml (12 tasks) | — |
| `roko plan list` | PASS | — |
| `roko plan validate` | FINDS REAL ISSUES (missing role, bad model names) | — |

## Critical Findings

### 1. Provider Routing Is Broken By Default (S1)

**Before `config migrate`:**
- `roko run` always tries `anthropic_api` even though `claude` CLI is available
- `roko "prompt"` falls back to `zai/glm-5.1` OpenAI-compat but provides no tools/context
- `--model` flag is completely ignored
- Interactive `roko` fails immediately with "Missing API key"

**After `config migrate`:**
- `roko run` correctly routes to `claude_cli`
- But the default config schema v1 has no `[providers]` section, so all fresh workspaces
  are broken until `config migrate` is run

**Root cause:** The default `roko.toml` emitted by `roko init` uses schema v1 which has
`[agent] command = "claude"` but the `roko run` WorkflowEngine reads provider config from
`[providers]` and `[models]` which don't exist. The one-shot path has a separate fallback
that finds `zai` keys in the environment.

### 2. Shell Gate Always Fails (Bug Found)

**Symptom:** `roko run` with `[[gate]] kind = "shell" program = "true"` always reports
gate failure, triggering infinite autofix loops.

**Root cause (from agent investigation):**
- `orchestrate.rs:7522` converts `GateConfig::Shell` to the string `"shell"`
- `gate_service.rs:65-80` has NO match case for `"shell"` in `gate_for_name()`
- Falls through to `_ => None`, which immediately creates `GateVerdict { passed: false }`
- The actual `ShellGate` implementation is never instantiated

**Impact:** Every `roko run` with the default gate config will: succeed at agent dispatch →
fail at gate → spawn autofix agent → fail at gate again → halt. This wastes 100% of
autofix agent cost ($0.50-2.00 per run) doing nothing useful.

### 3. Cost Tracking Shows $0.00 For Everything (S4)

**Evidence:** `prd plan` ran for 471s and cost $1.46 (from Claude billing). Roko logged:
- efficiency.jsonl: `cost_usd: 0.0, input_tokens: 0, output_tokens: 0`
- episodes: `$0.0000`
- status: `Total: $-0.0000`

**Root cause:** The episode logger records timestamps and success/failure but never extracts
token/cost metadata from the Claude CLI `stream-json` response. The `AgentResult.usage` is
populated with `wall_ms` only.

### 4. `learn all` Reads Wrong Path

**Evidence:** `.roko/learn/efficiency.jsonl` has 22 entries. `roko learn all` says "empty".

**Root cause:** The `learn` command reads from a different expected path or format than
what the efficiency logger writes. Likely a data-dir mismatch between where events are
written vs. where the read commands look.

### 5. PRD/Plan Generates Greenfield For Existing Functionality

**Evidence:** Asked to build "config file parser" for a project that already has `roko.toml`
loaded by the binary. Agent generates a plan to create `roko-config` crate with 22 structs.

**Context:** In a blank test project, this is arguably correct — there's no existing config
crate to wire into. But in the main roko repo (like the user's earlier demo), this becomes
the "duplicate crate" problem. The plan generator has no mechanism to check whether the
generated architecture overlaps with existing code.

### 6. Plan Validation Catches Schema Drift

**Evidence:** `plan validate` correctly identifies:
- All 12 tasks missing required `role` field
- Model hints (`haiku`, `sonnet`) don't match configured model names

**This is good!** Validation works. But the plan generator doesn't know about these
constraints, so it always generates invalid plans that would fail `plan run`.

### 7. `--model` Flag Is Ignored By Most Paths

**Evidence:** `--model gpt-4o`, `--model glm-5-1` — response always comes from glm-5.1 via
zai (one-shot path) or fails with anthropic_api (run/interactive path).

**Root cause:** The `--model` CLI flag is parsed but not propagated to the workflow engine's
model selection. The engine has its own provider/model resolution that ignores CLI overrides.

### 8. Bash Hook Blocks Plan Verification

**Evidence:** During `prd plan`, the agent tried to run `python3 -c "..."` to validate the
generated TOML. The Bash hook blocked it with "BLOCKED: git switch forbidden in plan worktrees".

**Root cause:** The user has a pre-tool hook that blocks Bash commands. The hook regex is
too broad — it catches any Bash use, not just `git switch`. The agent falls back to Read
tool (correct behavior), but cannot validate its own output.

## What Works Well

1. **`roko init`** — creates proper workspace structure, detects project domain
2. **`roko doctor`** — correct health diagnostics
3. **`roko prd draft new`** — successfully invokes Claude CLI with tools, reads workspace
4. **`roko prd plan`** — generates well-structured tasks.toml with DAG dependencies
5. **`roko plan validate`** — catches real schema issues in generated plans
6. **`roko config providers/models`** — accurate provider inventory
7. **`roko config migrate`** — correctly upgrades v1→v2 config
8. **Runtime events logging** — 34 events correctly logged with proper lifecycle
9. **Prompt composition** — engrams.jsonl shows proper section composition with VCG auction

## What's Broken (Priority Order)

1. **Provider routing without migration** — fresh workspaces can't use Claude CLI
2. **Shell gate always fails** — `gate_for_name("shell")` has no match case
3. **One-shot/interactive paths** — no system prompt, no tools, no context, no history
4. **Cost tracking** — $0 for everything, learning system has no signal
5. **`--model` flag ignored** — CLI args don't reach the engine
6. **`learn all` path mismatch** — reads wrong location for efficiency data
7. **Plan generator outputs invalid tasks** — missing `role`, wrong model names
8. **Bash hook too broad** — blocks agent self-validation

## Recommendations

1. **Fix `roko init` to emit schema v2** — include `[providers.claude_cli]` in default config
2. **Fix gate_for_name to handle "shell"** — one-line fix in `gate_service.rs`
3. **Fix the interactive/one-shot paths** — this is the M0 work in FINAL-SOLUTION.md
4. **Parse token/cost from Claude CLI stream-json** — extract from `result` event
5. **Wire --model flag through to engine** — check CLI arg in model resolution
6. **Fix learn/efficiency path mismatch** — align read/write paths
7. **Add `role` field to plan generator prompt** — or make it optional in validator
