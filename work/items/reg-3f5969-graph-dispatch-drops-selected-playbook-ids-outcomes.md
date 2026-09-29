+++
id = "reg-3f5969"
kind = "regression"
title = "Graph dispatch drops selected playbook IDs; outcomes are recorded under synthetic task IDs"
status = "done"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-cli/graph-dispatch", "roko-learn/playbooks"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "33e107da1"
source = "gaps-md#playbook-selection-wired-at-dispatch----resolved"
discovered_from = "doc:tmp/work-management/01-gaps-md-audit.md"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs:1539", "crates/roko-cli/src/graph_task_dispatch.rs:1735", "crates/roko-cli/src/graph_execution/feedback.rs::PlaybookSink::settle", "crates/roko-cli/src/dispatch/prompt_cache.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'playbook_ids: vec!\\[\\]' crates/roko-cli/src/graph_task_dispatch.rs && ! grep -nF 'format!(\"task-{}\", task.id)' crates/roko-cli/src/graph_task_dispatch.rs && ! grep -nF 'format!(\"task-{}\", receipt.task_id)' crates/roko-cli/src/graph_execution/feedback.rs && grep -q 'fn dispatch_outcomes_feed_knowledge_experiments_playbooks_and_episodes' crates/roko-cli/src/graph_task_dispatch.rs && cargo test -p roko-cli --lib dispatch_outcomes_feed_knowledge_experiments_playbooks_and_episodes"

[closed]
at = 2026-09-29
commit = "763596768"
by = "reconcile of the portal session's merges 2026-09-29 (static check)"
evidence = "763596768 (merged in 33e107da1): Graph dispatch credits the playbooks a prompt actually used and records outcomes under real task IDs; the problem patterns (playbook_ids: vec![] and synthetic task-{} IDs) are gone from graph_task_dispatch.rs and graph_execution/feedback.rs. Test dispatch_outcomes_feed_knowledge_experiments_playbooks_and_episodes."
+++

GAPS.md recorded the playbook loop as closed: Runner-v2 stored the playbook IDs chosen at dispatch for each task and called `record_outcome` for them on gate pass or fail. On the Graph path:
- dispatch passes `playbook_ids: vec![]` (`crates/roko-cli/src/graph_task_dispatch.rs:983`);
- the feedback sink records the outcome against the synthetic ID `task-{task_id}` (`crates/roko-cli/src/graph_execution/feedback.rs:468-471`).
Playbooks are still loaded into prompt context (`crates/roko-cli/src/dispatch/prompt_cache.rs`), but the success/failure counters of the playbooks actually used are never updated.

Fix: carry the selected playbook IDs from prompt assembly through the attempt receipt and record outcomes against those IDs. Add a test in which a gate pass increments the selected playbook's counter.

2026-09-29: re-verified at d9e79e9d8. Still open. There is a second synthetic-id write the item did not list: emit_feedback's W07 block (graph_task_dispatch.rs:1733-1736) also calls record_outcome with format!("task-{}", task.id). The current [[verify]] commands do not check it.
