# Architecture Summary — Verification Baseline

## Current-State Anchors

The live durable kernel noun is **`Engram`**.

The six live core traits are:

- `Substrate`
- `Scorer`
- `Gate`
- `Router`
- `Composer`
- `Policy`

The live transport implementation is `roko-runtime`'s generic `EventBus<E>`, with exactly two
live RokoEvent variants today: `PlanRevision` and `PrdPublished`.

`loop_tick()` is real, but parity wording should treat it as a shared helper rather than a single
universal owner of every production loop.

## Planned / Target-State Only

These remain design material unless code evidence says otherwise:

- generic kernel `Bus<E>` trait
- `Pulse`
- `Datum`
- demurrage as the governing durable-memory model
- `Worldview`
- `Custody`

## Workspace Baseline

- **36 workspace members**
- **322,088 Rust LOC**
- `roko-learn`: **42 modules**, **35,847 LOC**
- TUI: **~58K LOC**
- `roko-serve`: **200+ routes**

Use the exact `36 workspace members` and `322,088 Rust LOC` phrasing in parity files when fixing
stale scale claims.

## Post-Audit Reading Of Topic 00

- Docs `00-17`: mostly grounded, but several claims need narrower wording.
- Docs `18-22`: mixed; current-state sections and target-state mechanics need to be separated.
- Docs `23-29`: mostly future-work design material.
- Docs `30-35`: meta/planning material that must stop reading like live implementation status.

## Default Wording Bias

- Prefer `Engram` over legacy naming.
- Prefer `planned generic bus trait` over `shipped bus fabric`.
- Prefer `partial runtime ownership` over `fully wired universal loop`.
- Prefer `planning artifact` over `implementation gap` for synergy matrix and long roadmap material.
- Prefer `verification aid` over `proof` when talking about anchors and audit references.
- Prefer `dependency ordering` over staffed quarter-plan language when summarizing roadmap docs.
