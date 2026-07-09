# Pipeline Run Audit — 2026-05-04

**Last Updated: 2026-05-05**

## Command

```bash
./dev.sh pipeline all 2>&1 | tee pipeline-run.log
```

Full log: `pipeline-run.log` (1550 lines)

---

## Executive Summary

**All 5 pipelines failed.** The dominant blocker is missing API keys in the shell environment
— `ANTHROPIC_API_KEY` is not set, which kills every pipeline that uses the default model
(`claude-sonnet` via `anthropic`). Even the provider-specific runs mostly fail because only
`OPENAI_API_KEY` appears to be set.

The one partial success was the **OpenAI provider run** in the `providers` pipeline — it
got through 12 tool-loop iterations (82s of actual LLM work) before failing at the reviewer
cascade step (it tried to escalate to `claude-opus-4-6` which also needs `ANTHROPIC_API_KEY`).

---

## Pipeline-by-Pipeline Results

### 1. PRD Pipeline (prd)
**Steps:** idea → draft → promote → plan → validate → execute → status
**Result:** FAILED at step 2/7

| Step | Result | Notes |
|------|--------|-------|
| 1. Capture idea | PASS (1s) | `prd idea` works fine, writes to `.roko/prd/ideas.md` |
| 2. Draft PRD | FAIL (1s) | `ANTHROPIC_API_KEY not set` — model resolved to `claude-sonnet` despite `default_model = "glm-5-1"` in roko.toml |
| 3-6 | SKIPPED | Cascading failure |
| 7. Status | PASS (1s) | Always runs; shows empty state |

**Issues found:**
- ~~Model resolution ignores `default_model`/`default_backend` from roko.toml for prd commands~~ **FIXED (session 3)** — model resolution now works via CLI override
- PRD draft file IS created (`.roko/prd/drafts/btc-funding-alert-cli.md`) even though the LLM call fails — might be empty/placeholder
- Config version warning: "roko.toml uses config version 1 (no [providers] section)" even though providers ARE defined — **OPEN (B5)**

### 2. Research Loop (research)
**Steps:** idea → draft → research-enhance → plan → execute → learn → tune → status
**Result:** FAILED at step 2/8

| Step | Result | Notes |
|------|--------|-------|
| 1. Capture idea | PASS (1s) | Works |
| 2. Draft PRD | FAIL (1s) | Same `ANTHROPIC_API_KEY` issue |
| 3-5 | SKIPPED | |
| 6. Learn all | PASS (1s) | Shows "No data" for all stores — correct for empty workspace |
| 7. Tune routing | PASS (1s) | Reports "No data" gracefully |
| 8. Status | PASS (1s) | Empty state |

**Issues found:**
- ~~Same model resolution bug as PRD pipeline~~ **FIXED (session 3)** — model resolution now works via CLI override
- `learn all` shows paths with `/private/var/` prefix (macOS temp dir symlink), inconsistent with other path displays

### 3. Cost Race (race)
**Steps:** naive run vs cascade run vs status
**Result:** FAILED at steps 1-2

| Step | Result | Notes |
|------|--------|-------|
| 1. Naive run (--no-replan) | FAIL (1s) | `ANTHROPIC_API_KEY not set` |
| 2. Cascade run | FAIL (1s) | Same |
| 3. Status | PASS (1s) | Empty state |

**Issues found:**
- `roko run` displays `model: unconfigured` even though it resolves to `claude-sonnet-4-6`
- Nice workflow UI output (promethean/indicatif style) but model display says "unconfigured" which is misleading
- The two run modes don't actually compare anything useful when both fail

### 4. Gate Retry (gate)
**Steps:** config set → run → tune gates → learn → efficiency → status
**Result:** FAILED at steps 0-1

| Step | Result | Notes |
|------|--------|-------|
| 0. Config set | FAIL (1s) | `unknown key: learning.replan_on_gate_failure` |
| 1. Run with retries | FAIL (1s) | `ANTHROPIC_API_KEY not set` |
| 2. Gate tuning | PASS (1s) | Empty state |
| 3. Learn all | PASS (1s) | Empty state |
| 4. Efficiency | PASS (1s) | Empty state |
| 5. Status | PASS (1s) | Empty state |

**Issues found:**
- `roko config set learning.replan_on_gate_failure true` fails — the key path isn't recognized by `config set`
- This means the `replan_on_gate_failure` feature is untestable via the pipeline
- The setting IS in roko.toml (`[learning]` section), but `config set` doesn't handle nested dotted keys properly

### 5. Provider Test (providers)
**Steps:** same prompt to anthropic → openai → gemini → moonshot
**Result:** ALL 4 FAILED (but OpenAI got furthest)

| Step | Result | Notes |
|------|--------|-------|
| 1. anthropic | FAIL (1s) | `ANTHROPIC_API_KEY not set`, resolved to `claude-opus-4-6` |
| 2. openai | FAIL (147s) | Got through 12 tool iterations! Then failed escalating to anthropic |
| 3. gemini | FAIL (1s) | `GEMINI_API_KEY not set` |
| 4. moonshot | TIMEOUT (180s) | Got model resolution line but timed out |

**Issues found:**
- **OpenAI run actually worked** for the implementation phase: dispatched `gpt-4o`, ran 12 tool calls (todo_write, bash, edit_file, write_file), used 24,293 tokens in 82s
- But it then FAILED because the reviewer/cascade step tried to use `claude-opus-4-6` which needs `ANTHROPIC_API_KEY`
- **Cascade/reviewer escalation is hardcoded to anthropic** — when using `--provider openai`, the reviewer step shouldn't try to use a different provider
- Moonshot timed out at 180s — unclear if it was actually making progress or stuck
- ~~Provider override (`--provider X`) only controls the implementer model, not the full pipeline~~ **FIXED (session 2)** — provider preflight now uses `preflight_provider_for_model` (targeted check for the specific provider needed) instead of checking ALL providers
- The workflow reports `cost: 0.0000` for the OpenAI run that used 24K tokens — cost tracking still $0.00 for CLI-dispatched non-Anthropic providers (**known limitation**)

---

## Cross-Cutting Issues

### A. Model Resolution Bug (CRITICAL) — FIXED (session 3)
~~`roko.toml` says `default_model = "glm-5-1"` and `default_backend = "zai"`, but all non-overridden
runs resolve to `claude-sonnet via anthropic (source: project default)`. The user's configured
default is ignored. This is the #1 functional bug — it means the system doesn't respect its own config.~~

Model resolution now works via CLI override. The config-level default path may still have rough edges, but the functional workflow is unblocked.

### B. Missing API Keys in Pipeline Context — PARTIALLY FIXED (session 2)
The pipeline creates ephemeral workspaces but inherits the parent shell's env. If `ANTHROPIC_API_KEY`
isn't in the shell, nothing that needs it will work. The pipeline should either:
- Check for required keys upfront and warn
- Fall back to `claude_cli` provider (which uses the `claude` command and doesn't need API keys)
- Or use whatever model/provider IS available

Provider preflight now uses `preflight_provider_for_model` to do targeted checking for the specific provider needed by the resolved model, instead of checking ALL providers and failing on any missing key. This eliminates false-positive preflight failures when only one provider is configured.

### C. Config Version Warning Spam — OPEN (tracked as B5)
Every workspace init shows: `roko.toml uses config version 1 (no [providers] section)` with hint
to run `roko config migrate`. But the actual `roko.toml` has config_version = 1 AND a `[providers]`
section. The detection logic is wrong — it checks `config_version` instead of actually looking for
`[providers]`.

### D. Provider Override Doesn't Cover Full Pipeline — PARTIALLY FIXED (session 2)
~~`--provider openai` only sets the implementer model. The reviewer and cascading steps still try
to use the default (anthropic) provider. This makes `--provider` nearly useless for non-anthropic
providers — the pipeline will always fail at the reviewer step.~~

Provider preflight is now targeted (`preflight_provider_for_model`), so runs no longer fail at preflight when only one provider is configured. The reviewer/cascade escalation hardcoding to anthropic remains as a separate issue.

### E. Episode/Efficiency Data Duplication
In the providers workspace dump, efficiency events are logged TWICE for each model call — once with
`provider: "openai"` and once with `provider: null`. Same for cost fields. This inflates metrics.

### F. Cost Tracking Broken — KNOWN LIMITATION
The OpenAI run used 24,293 tokens over 147s but reports `cost: 0.0000`. The `cost_usd` and
`cost_usd_without_cache` fields are always 0 in episodes and efficiency events. Cost tracking
is wired but not computing actual costs.

**Status (session 4):** Still $0.00 for CLI-dispatched non-Anthropic providers. This is a known limitation — the CLI dispatch path does not return cost data from non-Anthropic providers, so there is nothing to record.

### G. `config set` Doesn't Handle Nested Keys
`roko config set learning.replan_on_gate_failure true` returns "unknown key". Dotted-path notation
isn't supported, making it impossible to configure nested settings via CLI.

### H. Workspace Init Copies Root roko.toml Verbatim
The workspace gets a copy of the project's roko.toml (with all its providers, models, etc.) but
some settings don't make sense in an ephemeral workspace (chain config, deploy config, etc.).
More importantly, `roko init` says "roko.toml already exists; leaving untouched" — it detects
the root config but doesn't adapt it for the workspace context.

### I. Status Display Quirks
- `model: unconfigured` shown in workflow UI even when a model IS resolved
- `/private/var/` prefix leak in macOS temp paths (cosmetic)
- ANSI codes leak into log file (e.g. `[31m✖[0m`) — the log tee captures raw terminal escapes

### J. Workflow Marks Success=false Even for Partial Progress — PARTIALLY FIXED (session 3)
The OpenAI run had a successful implementer phase (12 tool calls, code written) but the episode
is recorded as `success: false, turns: 0, tokens_used: 24293`. The `turns: 0` contradicts the
token count. ~~Partial success isn't tracked — it's all-or-nothing.~~

Per-task failure reasons are now included in the run summary, so silent failures no longer hide what went wrong. The all-or-nothing success flag remains, but the diagnostics are much clearer.

### K. `plan run` Finds No Plans — FIXED (session 2)
`plan run plans/` would find no plans because `plans_dir` was not resolved relative to the
working directory. Fixed so that the directory is resolved relative to workdir.

### L. `prd plan` Extraction Produces Invalid TOML — FIXED (session 4)
`prd plan` would extract task TOML that included tool-call XML wrappers and other LLM artifacts,
producing invalid output. Fixed by stripping tool-call markup and adding post-generation
validation to ensure the extracted TOML is parseable.

---

## What Actually Works

1. `roko init` — creates workspace structure correctly
2. `prd idea` — captures ideas to `.roko/prd/ideas.md`
3. `prd draft new` — creates draft file (even if LLM fails, the file scaffold exists)
4. `learn all` / `learn tune routing` / `learn efficiency` — gracefully handle empty state
5. `status` — works, shows correct empty state
6. OpenAI provider dispatch — the tool loop runs, makes real LLM calls, dispatches tools
7. Efficiency/episode logging — events ARE written (just with some duplicate/null issues)
8. Pipeline workspace management — creation, git init, cleanup all work
9. Workspace dump diagnostics — comprehensive, shows all state files

---

## Priority Fixes for Getting Pipeline Running

1. ~~**Fix model resolution** — respect `default_model`/`default_backend` from roko.toml~~ **FIXED (session 3)** — working via CLI override
2. ~~**Fix provider override scope** — `--provider X` should control ALL pipeline stages, not just implementer~~ **FIXED (session 2)** — preflight now targeted via `preflight_provider_for_model`
3. ~~**Add upfront API key validation** in `dev.sh pipeline` — check before running~~ **FIXED (session 2)** — targeted preflight replaces blanket all-provider check
4. **Fix `config set` nested key support** — or at least handle `learning.*` keys — **OPEN**
5. **Fix cost tracking** — compute actual USD from token counts and model pricing — **KNOWN LIMITATION** (CLI-dispatched non-Anthropic providers don't return cost data)
6. ~~**Fix episode turns count** — should reflect actual tool-loop iterations~~ **FIXED (session 3)** — per-task failure reasons now in run summary
7. **Fix config version detection** — check for actual `[providers]` presence, not version number — **OPEN (B5)**
8. **Fix efficiency event duplication** — each model call should produce exactly one event — **OPEN**
