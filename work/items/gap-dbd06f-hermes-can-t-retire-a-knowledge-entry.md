+++
id = "gap-dbd06f"
kind = "gap"
title = "Hermes can't retire a knowledge entry: roko knowledge retire exists and works beside a serve, but roko serve has no route for it, so neither a chat nor the plugin's knowledge page can retire a lesson"
status = "open"
triage = "verified"
severity = "p3"
goal = "hermes"
size = "S"
subsystem = ["roko-serve/neuro"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 roko-learnguard closing bug-2ad4fa, the lead's decision (a p3 follow-up)"
discovered_from = "agent:roko-learnguard (round v7)"
anchors = ["crates/roko-serve/src/routes/aggregator.rs", "crates/roko-neuro/src/knowledge_store/retire.rs::KnowledgeStore::retire"]
links = { depends_on = [], blocks = [], related = ["bug-2ad4fa"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn knowledge_retire_route_retires_one_entry' crates/roko-serve/src && cargo test -p roko-serve knowledge_retire_route_retires_one_entry"
+++

## Problem

Hermes can't retire a knowledge entry. `roko knowledge retire <id> --reason "…"` exists (bug-2ad4fa) and works
beside a running serve, but `roko serve` has no route for it, so neither a Hermes chat nor the plugin's knowledge
page can retire one.

In round v7 the pilot retired phase 5's gamed lesson (`runtime_3c3b9654100195f1`) by hand:
1. stop the serve;
2. back up the store and delete its line;
3. restart the serve and check `/api/knowledge/search`.

## Why it matters

- A person curating what roko learned works from Hermes, where the run and its lessons are shown.
- Making them leave for a terminal, and know the CLI, means a bad lesson stays retrievable longer.
- Goal: hermes.

## Where

- `crates/roko-serve/src/routes/aggregator.rs`: the knowledge routes (`/knowledge/entries`, `/knowledge/search`).
- `crates/roko-neuro/src/knowledge_store/retire.rs::KnowledgeStore::retire`: what the route calls.
- `crates/roko-serve/src/routes/route_permissions.rs`, `crates/roko-serve/src/openapi.rs`, and
  `tools/http_route_inventory.snapshot.json`: a new route's permission, its documentation and the route inventory.

## Plan

1. Add `POST /api/knowledge/entries/{id}/retire`.
   - Body: `{ "reason": "…" }`.
   - It calls `KnowledgeStore::retire` with the caller as the actor.
   - It returns `id`, `found_in` and `already_retired`.
   - An unknown id is a 404, and an empty reason a 400.
2. Give it a write permission, an OpenAPI entry and the inventory snapshot.
3. Show retired entries and their relearning counts on `/api/knowledge/entries`, or on a `/knowledge/retired` view.
   `KnowledgeStore::retirements` has them.
4. Plugin side (plugin-wow's): a retire action on the knowledge page.
5. Test: `knowledge_retire_route_retires_one_entry`.

## Done when

- The `[[verify]]` passes.
- A retire through the serve hides the entry from `/api/knowledge/search` without a restart.

## Notes

- Filed by roko-learnguard when it closed bug-2ad4fa, as the lead decided (p3 follow-up).
