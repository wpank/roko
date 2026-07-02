# PRD-08 — CLI Redesign

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-25
**Crate**: `roko-cli` (replacement of existing subcommand surface)
**Prerequisites**: PRD-00 through PRD-07

---

## 0. Scope

This PRD specifies the redesigned `roko` CLI. The existing `prd / plan / research / dashboard / agent / job / config / serve / daemon / deploy / ...` subcommand surface is replaced with a workflow-centric command set, with verb sugar over `roko run <workflow>` for ergonomic one-liners.

The design goals:

1. **One-line entry** — typical operations should be a single command.
2. **Workflow-native** — every meaningful operation is a Workflow run (or a registry/admin operation).
3. **Discoverable** — `roko` alone shows a workspace dashboard summary; tab-completion enumerates workflows; `--help` is exhaustive.
4. **Scriptable** — every command supports `--json` for machine consumption; exit codes are meaningful.
5. **Interactive when needed** — workflows that require human input do so via prompts; `--non-interactive` errors instead of prompting (for CI).

---

## 1. Top-Level Command Surface

```
roko                                  # workspace summary (active workspace, recent runs, pending humans)
roko help [<command>]
roko version

# Workflow operations — the primary surface
roko run <workflow> [args]            # run a workflow
roko run cancel <run-id>              # cancel an active run
roko run respond <run-id> [args]      # answer a human-input prompt
roko run resume <run-id>              # resume a snapshotted run
roko run list [--status]              # active and recent runs
roko run show <run-id>                # detailed run inspection
roko run logs <run-id> [--follow]     # stream run logs
roko run replay <run-id>              # rerun with the same inputs

# Workflow registry
roko workflow                         # alias for `roko workflow list`
roko workflow list [--installed | --catalog]
roko workflow show <name>
roko workflow validate <name>
roko workflow new <name>
roko workflow edit <name>
roko workflow fork <source> <new>
roko workflow remove <name>
roko workflow capabilities <name>
roko workflow benchmark <name>

# Modules
roko module list
roko module show <name>
roko module install <ref>
roko module remove <name>
roko module new <name>

# Profiles (visual-gate2)
roko profile list
roko profile show <name>
roko profile new <name>

# Triggers
roko trigger list
roko trigger show <name>
roko trigger create <name> --kind <kind> --workflow <name> [args]
roko trigger edit <name>
roko trigger enable <name>
roko trigger disable <name>
roko trigger remove <name>
roko trigger test <name> [--payload <json>]
roko trigger logs <name>

# Workspaces
roko workspace                        # show active
roko workspace open <name|path>
roko workspace switch <name|path>
roko workspace new <path> [--template <name>]
roko workspace list
roko workspace recent
roko workspace info [<name>]
roko workspace template list

# Artifacts
roko artifact list [--kind <kind>] [--tag <tag>]
roko artifact show <id>
roko artifact lineage <id>
roko artifact get <id> [-o <path>]
roko artifact remove <id>
roko artifact export <id> <path>

# Marketplace (PRD-12)
roko market browse [--query <q>] [--tag <tag>]
roko market show <ref>
roko market install <ref>
roko market publish <local-name>
roko market fork <ref>

# Daemon / serve / dashboard
roko daemon start | stop | status | logs
roko serve [--port <p>]               # HTTP control plane (PRD-10)
roko tui                              # ratatui (PRD-09)

# Diagnostics
roko doctor                           # workspace health, capabilities, daemon, providers
roko status                           # machine-wide: active workspace, runs, daemon, costs
```

---

## 2. Verb Sugar

Common Workflow runs get top-level verb aliases for ergonomic one-liners. Every verb expands to `roko run <workflow>` with the same input/macro plumbing. Verb sugar exists for discoverability — `roko --help` lists them in their own section.

```
roko ingest <dir> [opts]              ≡  roko run doc-ingest --input source_dir=<dir>
roko deploy [target] [opts]           ≡  roko run deploy --macro target=<target>
roko research <topic> [opts]          ≡  roko run research-sweep --input topic=<topic>
roko review [pr|diff|<paths>] [opts]  ≡  roko run code-review --input ...
roko audit [opts]                     ≡  roko run security-audit
roko refactor <description> [opts]    ≡  roko run refactor-batch --input description=<description>
roko test [scope] [opts]              ≡  roko run test-run --input target=<scope>
roko build [opts]                     ≡  roko run build
roko backup [opts]                    ≡  roko run backup
roko gc [opts]                        ≡  roko run gc
roko watch <path> --workflow <name>   ≡  roko trigger create + immediate enable
roko cron <expr> --workflow <name>    ≡  roko trigger create + immediate enable
```

Verb sugar accepts the same `--input`, `--macro`, `--from-file`, `--non-interactive`, etc. flags as `roko run`.

`roko chat [--agent <name>]` remains as direct interactive REPL — not a workflow because it's a stateful loop, not a finite computation.

---

## 3. Argument Conventions

### 3.1 Inputs

Workflow inputs map onto the Workflow's `input.schema`. CLI provides three ways to supply them:

```bash
# Per-arg (--input key=value, repeatable)
roko run doc-ingest \
    --input source_dir=tmp/ux-refresh \
    --input incremental=true

# Multiple keys via --inputs (TOML inline)
roko run doc-ingest --inputs '{ source_dir = "tmp/ux-refresh", incremental = true }'

# From file
roko run doc-ingest --from-file ingest.toml

# Stdin
cat ingest.toml | roko run doc-ingest --from-stdin
```

For positional args (verb sugar), the first positional fills the canonical input. `roko ingest <dir>` is equivalent to `roko run doc-ingest --input source_dir=<dir>`.

### 3.2 Macros

Macros are passed via `--macro key=value`, repeatable. Macro values are TOML-typed (booleans `true/false`, integers, floats, quoted strings).

```bash
roko run doc-ingest \
    --input source_dir=. \
    --macro enable_audit=true \
    --macro budget_usd=10.00 \
    --macro synthesizer_model="claude-opus-4-7"
```

### 3.3 Slot Filling

Slots fill via `--slot <slot_name>=<filling>` where the filling is a `module-ref@semver` or a path to an inline graph TOML.

```bash
roko run doc-ingest --slot researcher=academic-search@^1
roko run deploy --slot pre-deploy-check=./checks/staging-prep.toml
```

### 3.4 Universal Run Flags

```
--workspace <name>         # override active workspace for this command
--from-file <path>         # all inputs/macros/slots in one TOML file
--from-stdin               # read TOML config from stdin
--non-interactive          # never prompt; fail closed if human input needed
--detach                   # start the run, return run-id; daemon executes
--watch                    # follow the run output in this terminal
--dry-run                  # validate + estimate, don't execute
--budget <usd>             # override workflow budget
--deadline <duration>      # override workflow deadline
--json                     # JSON output (for scripting)
--quiet                    # only emit on errors
--verbose / -v             # more detail
--no-color                 # disable ANSI
--profile <name>           # use a saved invocation profile (saved arg sets)
```

---

## 4. The Default Behavior Model

### 4.1 Foreground vs. Detached

By default, `roko run` runs in the foreground: it prints live progress to the terminal until the run completes. Ctrl-C cancels.

`--detach` returns immediately with a run-id; the daemon executes; the user follows up via `roko run logs <run-id>` or the dashboard.

`--watch` is the default behavior — explicit only for clarity.

### 4.2 Human Input Handling

When a Workflow reaches a `HumanInput` node:
- **Interactive (TTY attached)**: prompt the user inline with the question and schema; user responds; run continues.
- **`--non-interactive`**: the run fails with `WorkflowError::HumanInputTimeout` immediately.
- **`--detach`**: the run pauses; the user receives a notification (CLI / dashboard / Slack via configured trigger); responds via `roko run respond`.

### 4.3 Confirmation Prompts

Pre-run, the CLI prints estimated cost/time and required capabilities. If interactive, it asks `Continue? [Y/n]`. If `--non-interactive`, it proceeds unless `confirm = always` in workspace policy.

The user can pre-approve common workflows via per-workflow policy in `workspace.toml`:

```toml
[workspace.workflows.confirm]
"doc-ingest" = "never"
"deploy"     = "always"
"*"          = "interactive"          # default
```

### 4.4 Error Handling

Exit codes:
- `0` — run succeeded.
- `1` — generic failure.
- `2` — bad CLI usage.
- `10` — workflow validation failed.
- `11` — capability denied.
- `12` — budget exceeded.
- `13` — deadline exceeded.
- `14` — human input timeout.
- `15` — cancelled.
- `16` — workspace error (registry / locked / not found).

`--json` errors are structured:

```json
{
  "ok": false,
  "exit_code": 11,
  "error": "CapabilityDenied",
  "module": "deploy-railway@1.0.0",
  "capability": "secrets.railway_token",
  "hint": "Run `roko config secret set railway_token` then retry."
}
```

---

## 5. Replacement of Existing Commands

The old surface is fully replaced. Migration is by aliases, not transformations:

| Old | New |
|---|---|
| `roko prd idea "<text>"` | `roko run prd-draft --input action=idea --input text="<text>"` |
| `roko prd draft new <slug>` | `roko run prd-draft --input action=draft --input slug=<slug>` |
| `roko prd draft edit <slug>` | `roko run prd-draft --input action=edit --input slug=<slug>` |
| `roko prd draft promote <slug>` | `roko run prd-draft --input action=promote --input slug=<slug>` |
| `roko prd plan <slug>` | `roko run prd-plan --input slug=<slug>` |
| `roko prd consolidate` | `roko run prd-consolidate` |
| `roko prd list` | `roko artifact list --kind prd` |
| `roko plan list` | `roko artifact list --kind plan` |
| `roko plan run <dir>` | `roko run plan-execute --input plan_dir=<dir>` |
| `roko plan generate --from-file <p>` | `roko run plan-from-doc --input source_file=<p>` |
| `roko plan validate <dir>` | `roko run plan-validate --input plan_dir=<dir>` |
| `roko research topic "<t>"` | `roko run research-sweep --input topic="<t>"` |
| `roko research enhance-prd <slug>` | `roko run prd-enrich --input slug=<slug>` |
| `roko research enhance-plan <slug>` | `roko run plan-enrich --input slug=<slug>` |
| `roko research analyze` | `roko run research-analyze` |
| `roko run "<prompt>"` | `roko run quick --input prompt="<prompt>"` |
| `roko deploy railway` | `roko deploy --target railway` |
| `roko deploy fly` | `roko deploy --target fly` |
| `roko worker` | `roko run worker` |
| `roko replay <hash>` | `roko run replay --input signal_hash=<hash>` |
| `roko inject <session> <payload>` | `roko run inject --input session=<s> --input payload=<p>` |
| `roko knowledge query "<t>"` | `roko run knowledge-query --input topic="<t>"` |
| `roko knowledge stats` | `roko artifact list --kind knowledge --json | <jq>` |
| `roko knowledge dream run` | `roko run dream-cycle` |
| `roko learn all` | `roko status learn` |
| `roko config init` | `roko workspace new <path>` |
| `roko config show` | `roko workspace info` |
| `roko config providers list` | `roko run providers-list` (workflow that lists configured providers) |
| `roko index build` | `roko run index-build` |
| `roko index search "<q>"` | `roko run code-search --input query="<q>"` |
| `roko explain <topic>` | `roko run explain --input topic=<topic>` |

A v1 transition release ships shims for the old commands that print:

```
DEPRECATED: `roko prd idea` is replaced by `roko run prd-draft --input action=idea`.
            Run with --no-deprecation-warnings to suppress.
```

The shims execute the new command transparently. The shim layer is removed in v2.

---

## 6. Discoverability

### 6.1 `roko` alone

```
$ roko
nunchi-dashboard  (active workspace at /Users/will/dev/nunchi/nunchi-dashboard)

Recent runs:
  ✓ wf_01HG... doc-ingest         12m ago    $1.84   100%   2/2 PRDs created
  ✗ wf_01HG... deploy-railway     1h ago     $0.30   error  smoke-test failed
  ⠿ wf_01HG... watch-on-src       running    -       -      file change pending

Pending:
  wf_01HG... doc-ingest awaiting human input on cluster (8m ago)

Triggers active: 4   Workflows: 47   Modules: 124   Cost today: $4.12

Get started:
  roko run <workflow>          run a workflow
  roko workflow list           browse available workflows
  roko trigger list            see active triggers
  roko tui                     open the TUI
  roko workspace switch <n>    change workspace
```

### 6.2 Tab Completion

Shell completions ship for bash, zsh, fish via `roko completions <shell>`. Completions enumerate:
- Workflow names (resolved from registry).
- Module names.
- Workspace names.
- Run IDs (recent; via local cache).
- Macro keys for the workflow at the cursor position.
- Artifact IDs (recent).

### 6.3 `--help` Hierarchies

Every command supports `--help` and `-h`. Top-level `--help` shows command groups; subcommand `--help` shows full flag list with examples. Workflow-specific help: `roko run <workflow> --help` shows the workflow's macros, slots, input schema with descriptions.

---

## 7. Configuration File Hierarchy

Three layers, deep-merged:

1. **Workspace**: `<workspace>/workspace.toml` (PRD-01) — top precedence.
2. **User**: `~/.roko/config.toml` — middle.
3. **Built-in defaults** — bottom.

CLI flags override config. Environment variables (`ROKO_*`) override config but are overridden by flags.

The user-level `~/.roko/config.toml` carries:

```toml
[ui]
default_view  = "tui"           # "cli" | "tui" | "dashboard"
color         = "auto"
density       = "comfortable"   # "compact" | "comfortable" | "spacious"

[runs]
default_detach   = false
default_confirm  = "interactive"
recent_history_n = 50

[notifications]
on_completion = ["desktop"]
on_failure    = ["desktop", "slack"]
on_human_input = ["desktop", "slack"]
slack_channel = "#roko-runs"
```

---

## 8. Profiles (Saved Argument Sets)

Frequent invocations can be saved as profiles in `<workspace>/.roko/profiles/<name>.toml`:

```toml
[profile]
name        = "ux-refresh"
workflow    = "doc-ingest"

[profile.input]
source_dir  = "tmp/ux-refresh"
incremental = true

[profile.macros]
enable_audit         = true
enable_web_research  = true
budget_usd           = 10.00

[profile.slots]
researcher = "academic-search@^1"
```

Run with `roko run --profile ux-refresh` or `roko ingest --profile ux-refresh`. Profiles are shareable, forkable, and may be marketplace artifacts in v1.1.

---

## 9. Dashboard / TUI Linkage

The CLI prints clickable URLs (when terminal supports OSC 8 hyperlinks) and `://` references that other surfaces understand:

```
$ roko run doc-ingest ...
Run id: wf_01HGZK7B...
Dashboard: http://localhost:6677/runs/wf_01HGZK7B...
TUI: roko tui --run wf_01HGZK7B...
```

`roko tui --run <id>` jumps directly to the run inspector.

---

## 10. Acceptance Criteria

| Criterion | Verification |
|---|---|
| `roko run <workflow>` runs in foreground with live progress on TTY. | Manual test on `doc-ingest`. |
| `roko run --detach` returns immediately; run continues in daemon. | `roko run logs <id>` shows progress after detach. |
| All verb-sugar commands map to a workflow run with correct input. | Equivalence test: `roko ingest <d>` and `roko run doc-ingest --input source_dir=<d>` produce identical run config. |
| `roko --json` produces valid, schema-conforming JSON for all commands. | Schema tests. |
| Tab completion enumerates workflows, modules, workspaces, run-ids. | Manual test in zsh + fish. |
| `roko run <workflow> --help` displays workflow-specific macro/slot/input documentation. | Help-output test. |
| Deprecation shims for old commands work and emit warning. | Shim test. |
| Workspace override (`--workspace`) operates on a non-active workspace without `cd`. | Cross-workspace test. |
| `--non-interactive` fails on human-input workflows; interactive prompts otherwise. | Two-mode test. |
| Pre-run confirmation honors `[workspace.workflows.confirm]` policy. | Policy matrix test. |

---

## 11. Open Questions

- Should `roko run` accept multiple workflows in one invocation as a quick chain? (`roko run a b c` ≡ chain a → b → c) — defer to v1.1; declarative trigger chains cover most use cases.
- Should there be a `roko alias` mechanism for users to define personal verb sugar? Yes — store in user config; cheap to implement.
- Should the CLI auto-launch the TUI for very long-running workflows? Optional via `[ui.long_running_threshold_s]` in user config.
- Output formatting: do we want a "machine-friendly progress" mode (`--progress=ndjson`) so other tools can consume? Yes; spec is straightforward.
