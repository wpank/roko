+++
id = "reg-3f5969"
kind = "regression"
title = "Graph dispatch drops selected playbook IDs; outcomes are recorded under synthetic task IDs"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-cli/graph-dispatch", "roko-learn/playbooks"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "gaps-md#playbook-selection-wired-at-dispatch----resolved"
discovered_from = "doc:tmp/work-management/01-gaps-md-audit.md"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs:983", "crates/roko-cli/src/graph_execution/feedback.rs:468", "crates/roko-cli/src/dispatch/prompt_cache.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = '''! grep -n 'playbook_ids: vec!\[\]' crates/roko-cli/src/graph_task_dispatch.rs'''

[[verify]]
command = '''! grep -nF 'format!("task-{}", receipt.task_id)' crates/roko-cli/src/graph_execution/feedback.rs'''
+++

GAPS.md recorded the playbook loop as closed: Runner-v2 stored the playbook IDs chosen at dispatch for each task and called `record_outcome` for them on gate pass or fail. On the Graph path:
- dispatch passes `playbook_ids: vec![]` (`crates/roko-cli/src/graph_task_dispatch.rs:983`);
- the feedback sink records the outcome against the synthetic ID `task-{task_id}` (`crates/roko-cli/src/graph_execution/feedback.rs:468-471`).
Playbooks are still loaded into prompt context (`crates/roko-cli/src/dispatch/prompt_cache.rs`), but the success/failure counters of the playbooks actually used are never updated.

Fix: carry the selected playbook IDs from prompt assembly through the attempt receipt and record outcomes against those IDs. Add a test in which a gate pass increments the selected playbook's counter.
