+++
id = "gap-c2efe7"
kind = "gap"
title = "Runs started by a chat host have no name: the task is 'Request from chat host hermes', the commit subject a UUID, and /mcp takes no title"
status = "open"
triage = "verified"
severity = "p3"
goal = "hermes"
size = "M"
subsystem = ["roko-serve/mcp", "roko-cli/run", "roko-cli/graph_execution"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
source = "tmp/hermes-roko-v5/findings/fresh-guide.md items 22, 25 (fresh-guide, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: a newcomer's first session"
anchors = ["crates/roko-serve/src/routes/mcp.rs", "crates/roko-cli/src/run.rs::prompt_tasks_file", "crates/roko-cli/src/task_parser.rs::TaskMeta"]
links = { depends_on = [], blocks = [], related = ["bug-db89d4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_named_plan_titles_its_commit_branch_and_plan_set' crates/roko-cli/ && cargo test -p roko-cli --lib a_named_plan_titles_its_commit_branch_and_plan_set"

[[verify]]
command = "grep -rqw 'fn mcp_run_prompt_takes_a_title' crates/roko-serve/ && cargo test -p roko-serve --lib mcp_run_prompt_takes_a_title"
+++

## Problem

Runs that a chat host starts carry no name a person can recognise (fresh-guide items 22 and 25, 2026-10-15):

- The task of every `/mcp` `run_prompt` run is titled "Request from chat host hermes" (decision 9117 keeps the
  untrusted request out of the agent-facing title), and nothing else names the run: its plan's id is the run's UUID.
- The commit a person merges from such a run, and so the subject in their history, is
  "roko: b9f1efaa-1566-45a4-925c-f920cf041645/T1: Request from chat host hermes".
- The batch branch `roko/batch/<run>` has no description, and the run's summary has no title, so hosts fall back
  to the model's whole request ("Add a --version flag to the wordy CLI. Files: - wordy/cli.py: the argparse-based
  CLI. Its main(argv=None) builds an Arg…").
- The Hermes plugin already takes a `title` from its model and its UI, but roko's `/mcp` tools do not accept one
  (`run_prompt`'s schema has `additionalProperties: false`), so it keeps it to itself.

Expected: `run_prompt` (and `plan_generate`) accept an optional `title`. A run uses it, or with none a title derived
from its request (the first sentence, at most 72 characters), for: its plan (`[meta] title`), the run's summary
(`title`), the batch branch's description (`git config branch.<batch>.description`) and the subject of the commit a
person takes from it. The agent-facing task title of a chat run stays "Request from chat host …" (9117).

## Why it matters

A person reviewing and merging roko's work needs to know which run it was; a UUID and "Request from chat host hermes"
in their git history read as noise (goal `hermes`).

## Where

- `crates/roko-serve/src/routes/mcp.rs`: the `run_prompt` and `plan_generate` tool schemas and arguments.
- `crates/roko-serve/src/runtime.rs::PromptPlanOptions`, `crates/roko-cli/src/serve_runtime.rs`: the prompt run's
  options.
- `crates/roko-cli/src/run.rs::prompt_tasks_file`: the one-task plan of a prompt run (title "Request from chat host
  …" for `/mcp`).
- `crates/roko-cli/src/task_parser.rs::TaskMeta`: no plan title today.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::plan_set_entries` (the plan's title is its id),
  `crates/roko-cli/src/graph_execution/batch.rs` (the batch branch),
  `crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs::accept_attempt` (the accepted commit's subject is
  the task title).
- `crates/roko-serve/src/routes/runs.rs`: the run summary.

## Plan

1. `TaskMeta.title` (optional): a plan's name for people; never part of an agent's prompt.
2. `/mcp` `run_prompt` and `plan_generate` accept `title` (one line, at most 120 characters); `PromptPlanOptions`
   and `PromptRun` carry it; with none, derive it from the request's first sentence (72 characters).
3. The run uses it for the plan set entry, the batch branch's description, and the accepted commit's subject of a
   one-task plan; the summary reports `title` from the run's plan set.

## Done when

The `[[verify]]` tests pass: a chat run's plan has the given title (or the derived one), its accepted commit's subject
names it, its batch branch is described by it, and its summary reports it; the task's own title is unchanged.
