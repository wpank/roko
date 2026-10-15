+++
id = "gap-89f985"
kind = "gap"
title = "Plan runs never create playbooks (only roko bench does), so L-play never acts on a real run: write one from each verified plan task"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-cli/runtime_feedback", "roko-learn/playbook"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-16"
anchors = ["crates/roko-cli/src/run.rs::extract_bench_playbook", "crates/roko-learn/src/playbook.rs::extract_playbook_from_episode", "crates/roko-learn/src/playbook.rs::PlaybookStore", "crates/roko-cli/src/runtime_feedback/verified_knowledge.rs::VerifiedKnowledgeSink", "crates/roko-cli/src/graph_execution/plan_runner.rs::build_graph_feedback_facade"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn verified_plan_task_writes_a_playbook' crates/roko-cli/ && cargo test -p roko-cli verified_plan_task_writes_a_playbook"
+++

## Problem

No Graph plan run creates a playbook. The only producer is `extract_bench_playbook`
(`crates/roko-cli/src/run.rs:1090`), used by `roko bench` (and a serve bench path, `serve_runtime.rs:191`). The plan
path only credits playbooks that already exist (`graph_task_dispatch/feedback.rs:606-622`, W07). So L-play (credit,
the 20% withhold arm, topic-matched injection) only acts on playbooks made elsewhere: the repo's 28 playbooks date
from 2026-09-24 (dreams), and a new workspace has none.

Seen live (A05 §4 S2; E2b LM2): after 10 tally runs on 2026-10-14, `.roko/learn/playbooks/` was empty, with no L-play
decision rows and no playbook exposures, in all 7 real workspaces.

## Why it matters

Goal `learning` (S02 L7/L8, which assumed plan runs produce playbooks). Will's decision 2 (2026-10-15) lists "create
playbooks on plan runs". NEW-FINDINGS finding 16. The papers say playbooks are learned from verified work (D5 8-22,
8-23). Without a producer, the withhold arm has nothing to withhold and β for L-play (gap-03fcd0) can never be measured.

## Where

- `crates/roko-cli/src/run.rs::extract_bench_playbook`: the existing producer (tool calls from the output text or the
  latest learning episode).
- `crates/roko-learn/src/playbook.rs::extract_playbook_from_episode` and `PlaybookStore` (locked writes, 4103).
- `crates/roko-cli/src/runtime_feedback/verified_knowledge.rs::VerifiedKnowledgeSink`: the model for a sink that
  writes only from gate-verified passes (`FeedbackEvent::TaskVerified`).
- `crates/roko-cli/src/graph_execution/plan_runner.rs::build_graph_feedback_facade`: where the sink registers.
- `crates/roko-cli/src/dispatch/prompt_builder.rs`: topic-matched injection (4212, `MIN_TOPIC_OVERLAP`).

## Current state

Selection, credit and the withhold arm are done and tested (4103, 4120, 4212). The producer is the missing piece.

## Plan

1. Add a playbook sink on the Graph facade that reacts to `TaskVerified` (a gate-backed pass with a learning label of
   success) and builds a playbook from that attempt's tool calls with `extract_playbook_from_episode`, keyed by the
   task's topic terms (title and description).
2. Admission: only verified passes; skip a playbook whose topic terms and steps match an existing one (credit that one
   instead); cap new playbooks per run (for example 3); write nothing on a frozen run (`[learning] frozen`).
3. Record a `playbook_created` row (or the knowledge-written equivalent) so the census and dashboards can count them.
4. Test `verified_plan_task_writes_a_playbook`: a fake-provider plan run whose task passes its verify leaves one
   playbook in `.roko/learn/playbooks/`; a failed or unverified task leaves none; a frozen run leaves none.

## Done when

The `[[verify]]` command passes, and a second run of a plan with an overlapping task title injects the playbook (one
`playbook` exposure row, or a withheld one on L-play's arm).

## Notes

- Regression-gated admission (D5 8-23, GRASP-style held sets) is not part of this item.
- Keep the producer cheap: no model call. Dreams stay held (decision 12).
