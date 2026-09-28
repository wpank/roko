+++
id = "gap-8dae52"
kind = "gap"
title = "Finish RuntimeServices migration: serve still builds services per call via ServiceFactory::build"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-execution/runtime-services", "roko-serve"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "gaps-md#6-non-plan-runtime-services-migration-245"
anchors = ["crates/roko-serve/src/state.rs:924", "crates/roko-execution/src/runtime_services.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

#243 landed on 2026-09-05, and `RuntimeServicesBuilder` is used by `roko-cli` (`run.rs`, `chat_session.rs`, `commands/graph.rs`) and by `roko-execution`. GAPS.md still listed the remaining #245 rows as "blocked on #243". What is still open:
- `crates/roko-serve/src/state.rs:924` builds its service bundle with `ServiceFactory::build`.
- Not re-checked: whether ACP's `SessionManager` health registry and rate limiter, and chat's per-session `ChatFeedbackRuntime`, now come from the builder.
- The conformance tests proving that each profile activates its mandatory services exist only as unit tests in `runtime_services.rs`.

The audits counted 6 initialisation paths with very different subsystem sets.

Fix: move serve, and any remaining ACP or chat construction, onto the builder. Delete the duplicate `ServiceFactory::build` calls and add per-profile conformance tests.
