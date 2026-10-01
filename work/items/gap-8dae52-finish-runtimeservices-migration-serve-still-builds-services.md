+++
id = "gap-8dae52"
kind = "gap"
title = "Finish RuntimeServices migration: serve still builds services per call via ServiceFactory::build"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-execution/runtime-services", "roko-serve"]
created = 2026-09-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "gaps-md#6-non-plan-runtime-services-migration-245"
anchors = ["crates/roko-serve/src/state.rs:1029", "crates/roko-serve/src/service_factory.rs::ServiceFactory::build_with_runtime_services", "crates/roko-acp/src/session.rs:648", "crates/roko-execution/src/runtime_services.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'ServiceFactory::build(' crates/roko-serve/src/state.rs && grep -qE 'build_with_runtime_services|RuntimeServicesBuilder' crates/roko-serve/src/state.rs"
+++

#243 landed on 2026-09-05, and `RuntimeServicesBuilder` is used by `roko-cli` (`run.rs`, `chat_session.rs`, `commands/graph.rs`) and by `roko-execution`. GAPS.md still listed the remaining #245 rows as "blocked on #243". What is still open:
- `crates/roko-serve/src/state.rs:924` builds its service bundle with `ServiceFactory::build`.
- Not re-checked: whether ACP's `SessionManager` health registry and rate limiter, and chat's per-session `ChatFeedbackRuntime`, now come from the builder.
- The conformance tests proving that each profile activates its mandatory services exist only as unit tests in `runtime_services.rs`.

The audits counted 6 initialisation paths with very different subsystem sets.

Fix: move serve, and any remaining ACP or chat construction, onto the builder. Delete the duplicate `ServiceFactory::build` calls and add per-profile conformance tests.

Re-checked 2026-09-29 at d9e79e9d8: unchanged apart from line moves. Serve calls ServiceFactory::build at state.rs:1029, and ServiceFactory::build_with_runtime_services (service_factory.rs:401) has no caller. ACP's SessionManager builds its own ProviderHealthRegistry (roko-acp/src/session.rs:648, :1229), and chat builds ChatFeedbackRuntime per session (chat_session.rs:728); none of them comes from RuntimeServicesBuilder.

## Notes

- 2026-10-01 (wk-serve2): not migrated; blocked on prerequisite work, no code changed. Findings at BASE `ebdc0f5d5`:
  - Serve builds its bundle once, in `AppState::new` (`crates/roko-serve/src/state.rs:1039`), not per call. The
    defect is a second construction path, not per-call cost.
  - `ServiceFactory::build_with_runtime_services` (`service_factory.rs:426`, no callers) gives the feedback service
    no cascade journal ("These services journal nothing"). Moving serve onto it as it stands would drop the
    crash-safe journaling that bug-8a78e1 added; serve saves its router through that journal (`state.rs:1446`).
  - `RuntimeServicesBuilder::build` (`crates/roko-execution/src/builder.rs:439`) is not ready for serve: it loads two
    separate `ProviderHealthRegistry` instances for the same file (dispatch and feedback), and fills extensions,
    observation and guards with `for_test()` stubs unless they are injected.
  - Next steps, in order: (1) give the two `ServiceFactory` paths one shared body, so the runtime-services path
    journals like `build`; (2) let the builder take an injected health registry and share it between dispatch and
    feedback; (3) in `AppState::new`, build `RuntimeServices` with the journal-recovered router (`load_recovered_router`)
    and the registry serve already exposes, then call `build_with_runtime_services`; (4) per-profile conformance
    tests. ACP's `SessionManager` and chat's `ChatFeedbackRuntime` (2026-09-29 re-check) are separate follow-ups.
