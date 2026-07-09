# CLI Workflow Gaps

## Current Pipeline: What Works

```
CAPTURE          GENERATE           EXECUTE           MONITOR
roko note    →   roko do/develop →  roko plan run  →  roko dashboard
roko prd idea    roko prd plan      (runner v2)       roko status
```

### Working Commands

| Command | What | Implementation | LOC |
|---------|------|---------------|-----|
| `roko note "text"` | Instant capture with tags | `commands/note.rs` | 50 |
| `roko do "prompt"` | Auto-classify → plan → execute | `commands/do_cmd.rs` | 1,117 |
| `roko develop "prompt"` | Force plan + approval + TUI | `commands/develop.rs` | 212 |
| `roko plan run <dir>` | Execute tasks.toml | `commands/plan.rs` | 1,771 |
| `roko plan generate` | Generate from prompt/PRD | `commands/plan.rs` | — |
| `roko prd idea "text"` | Legacy capture | `commands/prd.rs` | 962 |
| `roko prd draft new` | Draft PRD (agent-driven) | `commands/prd.rs` | — |
| `roko prd plan <slug>` | Generate tasks from PRD | `commands/prd.rs` | — |
| `roko research enhance-*` | Enhance documents with research | `commands/research.rs` | — |
| `roko think "topic"` | Read-only research | `commands/think.rs` | — |
| `roko dashboard` | Live TUI (F1-F7 tabs) | `commands/dashboard.rs` | — |

### Three Workflows

**Quick (simple tasks)**:
```bash
roko do "fix the login bug"
```

**Full (complex features)**:
```bash
roko note "cursor composer 2 — ACP agent, JSON-RPC"
roko prd draft new "cursor-backend"
roko research enhance-prd cursor-backend
roko prd plan cursor-backend
roko plan run .roko/prd/plans/cursor-backend/
```

**One-command (medium complexity)**:
```bash
roko develop "I want cursor composer 2 support"
```

## What's Missing

### Gap 1: `--context <path>` flag (doc 11)

**Problem**: Can't point roko at files/folders for context during plan generation.
Must copy-paste or rely on agent's file reading (which is broken — see 04-TOOL-DISPATCH-BROKEN.md).

**Design** (from `tmp/solutions/self-developing/11-context-sources-and-editor-integration.md`):
```bash
roko do --context ./crates/roko-agent/ "fix tool dispatch"
roko develop --context ./tmp/solutions/ "implement the self-dev UX"
roko plan generate --context ./docs/ "add auth"
```

**Implementation**:
1. New `context_loader.rs` (~150 lines):
   - Walk directory, respect `.gitignore`
   - Token budget with priority ranking (keyword match > recent > depth)
   - Format as XML context blocks
2. Add `--context` to `do_cmd.rs`, `develop.rs`, `plan.rs` (clap arg + inject into prompt)
3. ACP: Add `/context` slash command + pinned context sessions

**Files**:
- New: `crates/roko-cli/src/context_loader.rs`
- Modify: `crates/roko-cli/src/commands/do_cmd.rs` (add --context, ~20 lines)
- Modify: `crates/roko-cli/src/commands/develop.rs` (add --context, ~20 lines)
- Modify: `crates/roko-cli/src/commands/plan.rs` (add --context, ~20 lines)
- Modify: `crates/roko-acp/src/bridge_events.rs` (slash commands, ~200 lines)

### Gap 2: Plan Refinement Operations

**Problem**: Once a plan is generated, you can't iteratively improve it without
regenerating from scratch.

**Missing operations**:
- `roko plan --update <slug>` — merge new context into existing plan
- `roko plan --reframe <slug>` — rewrite plan from different angle
- `roko plan --split <slug>` — fork one plan into multiple smaller ones
- `roko plan --from-notes` — cluster accumulated notes → generate plans

**Design** (from `tmp/solutions/self-developing/05-idea-to-execution-flow.md`):

```bash
# Capture notes over time
roko note "tool dispatch is broken in research"
roko note "need to add gemini CLI backend"
roko note --tag ux "ANTHROPIC_API_KEY warnings everywhere"

# Cluster and synthesize
roko plan --from-notes          # groups notes by topic, generates plan per cluster
roko plan --from-notes --tag ux # only notes tagged "ux"

# Refine existing plan
roko plan --update provider-fixes --context ./tmp/tmp-feedback/2/
roko plan --reframe provider-fixes  # different approach
```

**Implementation**:
- `--from-notes`: Read `.roko/notes/`, cluster by tag/similarity, generate plan per cluster
- `--update`: Load existing tasks.toml + new context → re-generate preserving done tasks
- `--reframe`: Re-generate from scratch with different framing prompt
- `--split`: Split plan by task groups, create sub-plans

**Files**:
- Modify: `crates/roko-cli/src/commands/plan.rs` (add subcommands/flags)
- New: `crates/roko-cli/src/plan_refine.rs` (refinement logic)

### Gap 3: `roko plan "prompt"` Direct Mode

**Problem**: `plan` exists only as subcommands (`list/show/run/generate`).
Can't just type `roko plan "add auth"` to generate a plan.

**Design** (from `tmp/solutions/self-developing/09-unified-cli-ux.md`):
```bash
roko plan "add user authentication"  # generates plan, shows it, waits for approval
```

This is the 3-verb model: **note** (capture) → **plan** (synthesize) → **do** (execute).

**Implementation**:
- Add positional `[PROMPT]` arg to `PlanCmd` in clap
- If prompt is provided and no subcommand, route to `plan generate` flow
- Show plan approval screen (reuse from `develop.rs`)

**Files**:
- Modify: `crates/roko-cli/src/commands/plan.rs` (add prompt mode, ~100 lines)
- Modify: `crates/roko-cli/src/main.rs` (clap routing)

### Gap 4: TOML Self-Healing (doc 02, 05)

**Problem**: Plan generation with weak models often produces invalid TOML.
Currently retries 3x with the same model, then gives up.

**Missing**:
- Auto-escalate to stronger model on TOML validation failure
- Pass previous error to retry prompt (model sees what it got wrong)
- Self-heal missing `[[task.verify]]` fields with defaults
- Show raw output (first 2000 chars) when agent crashes

**Files**:
- Modify: `crates/roko-cli/src/prd.rs:1224` (retry loop + escalation, ~60 lines)
- Modify: `crates/roko-cli/src/prd.rs:1169` (show output on crash, ~3 lines)
- Modify: `crates/roko-cli/src/prd.rs:1240` (pass prev error to retry, ~5 lines)
- Modify: `crates/roko-cli/src/prd.rs:2071` (validate_and_fix, ~30 lines)

### Gap 5: Zero-Knowledge Onboarding (doc 04)

**Problem**: No path from "I installed roko" to "roko is self-developing."

**Missing**:
- `roko setup` wizard (auto-detect auth, prompt for model, write config, run doctor)
- Enhanced doctor (provider_usable, default_model_configured checks)
- Auto-generate provider blocks from detected env vars
- "Next steps" prompts after operations

**Implementation status**: `roko setup` command exists (`commands/setup.rs`) but needs
verification of completeness.

### Gap 6: Model Discovery (doc 08)

**Problem**: Can't discover available models without reading `roko.toml`.

**Missing**:
- `roko models list` (shows configured + builtin, availability status)
- Fuzzy matching on typo (suggest closest matches via Jaro-Winkler)
- Shell completion for model names

**Depends on**: Builtin model registry (doc 01) — P0 blocker.

## Priority Order

1. **Tool dispatch fix** (04-TOOL-DISPATCH-BROKEN.md) — blocks research + all non-Claude paths
2. **`--context` flag** — most impactful single feature for plan quality
3. **TOML self-healing + model escalation** — blocks weak-model plan generation
4. **Plan refinement** (`--from-notes`, `--update`) — completes the capture→synthesize loop
5. **`plan "prompt"` direct mode** — UX polish
6. **Onboarding** — blocks new user adoption
