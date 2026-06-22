# S-learn-D: Resolve event_subscriber + feedback_service overlap

## Task
`crates/roko-learn/src/event_subscriber.rs` and `crates/roko-learn/src/feedback_service.rs` overlap with `crates/roko-cli/src/runtime_feedback/`. Pick one canonical home; delete the duplicate.

## Runner Context
Runner audit-2026-05-01, group S. Depends on T2-17a, T2-17b. Wave 3.

## Source plan
`tmp/subsystem-audits/implementation-plans/25-learning-feedback-completion.md` § Phase E.

## Read first

```bash
rg 'pub use|pub fn|pub struct' crates/roko-learn/src/event_subscriber.rs crates/roko-learn/src/feedback_service.rs
rg 'event_subscriber::|feedback_service::' crates/ -g '*.rs' \
  | rg -v 'crates/roko-learn/'
```

Identify external callers of each.

## Decision tree

For each module:

- **No external callers**: delete the module + remove from `lib.rs`.
- **External callers, but `roko-cli/src/runtime_feedback/` does the job better**: migrate callers to `runtime_feedback::*`, then delete.
- **External callers, and module does something distinct that runtime_feedback doesn't**: rename to disambiguate (`event_subscriber` → `learn_event_subscriber`?). Document the contract.

## Exact changes

The audit suggests `feedback_service` overlaps with `runtime_feedback::FeedbackFacade`. Most likely outcome: migrate any consumers of `feedback_service::FeedbackService` to use `roko_cli::runtime_feedback::FeedbackFacade`, then delete `feedback_service.rs` from `roko-learn`.

For `event_subscriber`: depends on its actual purpose. If it's a different thing (e.g. an event-bus listener for cross-process events), keep it but rename. If it's a `FeedbackSink` impl that already exists in `runtime_feedback/`, delete.

## Write Scope
- `crates/roko-learn/src/event_subscriber.rs` (delete, rename, or migrate)
- `crates/roko-learn/src/feedback_service.rs` (delete or migrate)
- `crates/roko-learn/src/lib.rs` (mod declarations)
- (Caller crates that need migration)

## Verify

```bash
rg 'feedback_service|event_subscriber' crates/ -g '*.rs'
# Each remaining hit should be in the canonical home with documented purpose.
```

## Do NOT

- Do NOT keep both. Pick one.
- Do NOT bundle with other S-learn batches.
- Do NOT delete a module before migrating its callers.
- Do NOT introduce a third feedback subsystem.
