# Architecture Summary — Verification Baseline

## Current-State Anchors

The live durable kernel noun is **`Engram`**.

The six live core traits remain:

- `Substrate`
- `Scorer`
- `Gate`
- `Router`
- `Composer`
- `Policy`

The live transport implementation is `roko-runtime`'s generic `EventBus<E>`, with exactly two
live `RokoEvent` variants today:

- `PlanRevision`
- `PrdPublished`

`loop_tick()` is real, but parity wording should treat it as shared loop logic rather than a
single universal owner of all production runtime behavior.

## Planned / Target-State Only

These stay design material unless code evidence changes:

- generic kernel `Bus<E>` trait
- `Pulse`
- `Datum`
- demurrage as the governing durable-memory model
- `Worldview`
- `Custody`

## Audit-Normalized Workspace Baseline

Use the normalized audit phrasing below in parity notes:

- **36 workspace members**
- audit detail: **32 crates + 3 apps + 1 test crate**
- **322,088 Rust LOC**
- `roko-learn`: **42 modules**, **35,847 LOC**
- TUI: **~58K LOC**, wired, WebSocket-backed
- `roko-serve`: **200+ routes**, wired

## Post-Audit Reading Of Topic 00

- Docs `00-17`: mostly grounded, but several claims need narrower wording.
- Docs `18-22`: mixed; current-state sections and target-state mechanics need separation.
- Docs `23-29`: mostly future-work design material.
- Docs `30-35`: meta/planning material that must stop reading like implementation proof.

## Default Wording Bias

- Prefer `Engram` over legacy naming.
- Prefer `planned generic bus trait` over `shipped bus fabric`.
- Prefer `partial runtime ownership` over `fully wired universal loop`.
- Prefer `planning artifact` over `implementation gap` for synergy matrix and long roadmap material.
- Prefer `verification aid` over `proof` when talking about anchors and audit references.
- Prefer `dependency ordering` over staffed quarter-plan language when summarizing roadmap docs.
- Prefer `narrow editorial pass` over `multi-week delivery plan` when summarizing PU00 work.
