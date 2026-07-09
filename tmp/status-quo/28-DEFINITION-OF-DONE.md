# Definition Of Done

**Date**: 2026-07-08 · HEAD `5852c93c05` on `main`. Cross-refs: [95-ENGINE-DRIFT.md](95-ENGINE-DRIFT.md), [75-SECURITY-AUTH-SCOPE-MATRIX.md](75-SECURITY-AUTH-SCOPE-MATRIX.md), [60-STATE-PERSISTENCE-LEDGER.md](60-STATE-PERSISTENCE-LEDGER.md).

This file defines what "fully migrated" means for the project. It is intentionally stricter than "compiled" or "has an API".

## Universal Done Criteria

- The default user path uses the feature.
- The feature emits durable events or state using the canonical path.
- It has a smoke or integration test that would fail if the feature silently became a stub.
- Docs, CLI help, demo/TUI labels, and API examples do not contradict the implementation.
- Legacy compatibility exists only behind an explicit adapter, flag, or documented read-order.
- Any old path left behind has an archive/migration rule.

## Runtime Done

- `roko plan run` performs real work by default or refuses unsupported execution.
- `roko plan run --engine runner-v2` and `--engine graph` have separate, truthful capability descriptions.
- Resume reads a snapshot, skips completed tasks, and appends new run events.
- A run has one canonical run ID, task ID vocabulary, and completion status across CLI, StateHub, JSONL, TUI, and serve.
- Gates cannot record positive learning from stub-pass verdicts.
- Agent dispatch records provider, model, routing reason, tool calls, cost, gate result, and feedback sink outcome.
- `research search` returns real Perplexity results and is covered by a live (non-mock) test.
- Tool dispatch preserves the full tool set on non-Claude providers (no PascalCase/snake_case alias stripping).

- A default `roko plan run` smoke test fails if any output contains the `task-output:stub:` marker.
- `roko resume` routes to a snapshot-capable engine (Runner v2), not hardcoded Graph.

## Security Done

- The relay proxy (`/relay/*`) is inside the auth stack; an unauthenticated GET/POST/DELETE/WS returns 401.
- No mutating `/api/*` route resolves to the `read` scope; a read-scoped key is rejected on any mutating route, enforced by a CI classifier test.
- ACP `write_file`/`edit_file`/`bash` call `request_permission` before executing; an unauthorized call is denied end-to-end.
- Safety SecretLeak and PathEscape post-checks `Block` the turn, not merely `Warn`.
- `custody verify` runs the real hash-chained audit and fails on a tampered chain (no false "OK").
- `config show --effective` and `config show` redact secret-typed fields; a seeded key does not appear in output.
- The deployed worker callback carries a scoped auth token; an unauthenticated callback is rejected.

## Graph/Cell Done

- No production graph task execution uses `TaskExecutorCell` dry-run behavior; the built `AgentCell` is registered or the path refuses.
- Graph cells for task, agent, compose, gate, memory, learning, and commit have real side effects or are marked unsupported.
- Graph snapshots can resume with the same semantics as Runner v2.
- Conditional edges and failed gates do not collapse into success.
- Graph events are visible through StateHub and serve streams.

## State Done

- `.roko/episodes.jsonl`, `.roko/learn/episodes.jsonl`, and `.roko/memory/episodes.jsonl` have one canonical writer and explicit legacy readers.
- `.roko/events.jsonl`, `.roko/runtime-events.jsonl`, and `.roko/state/events.json` have a documented relationship.
- `.roko/engrams.jsonl` and `.roko/signals.jsonl` are not both described as canonical; gate verdicts and dashboards read/write the same log (`Engram` is the noun — see D3/D13).
- `state-snapshot.json` is the canonical snapshot; serve reads it (not the missing `state/executor.json`).
- `events.jsonl` is not a `feed_tick` firehose; low-value ticks live in a separate stream or are trimmed.
- Cold-substrate archival moves (or dedups) rather than copies — the cold store is bounded across repeated hourly cycles.
- Daimon state has one canonical path.
- State migration is idempotent and dry-runnable.

## API/Frontend Done

- Each frontend endpoint appears in a generated or checked route manifest.
- The React app does not call the 4 known missing aliases (`share` vs `shared`, `bench/matrix`, `isfr/stream`, `ws/agents`); event field casing (camelCase vs snake_case) is consistent across serve and client.
- SSE and WS streams have replay/filter semantics documented and tested.
- `roko-serve` auth scopes are tested for public, read, write, secret, and admin routes.
- The OpenAPI route inventory agrees with the Axum route tree.

## Config Done

- `roko.toml`, global config, CLI flags, named env vars, and `ROKO__SECTION__KEY` overrides have a documented precedence order.
- New config fields exist in the core schema, CLI display, validation, migration, and docs.
- Deprecated aliases produce warnings or migration output.
- Secret-like env vars are never echoed in logs, errors, StateHub, or frontend payloads.

## Docs Done

- `README.md`, `CLAUDE.md`, `docs/v1`, `docs/v2`, `docs/v2-depth`, and this pack agree on engine defaults, safety behavior, route status, and chain/ISFR maturity.
- Stale tmp designs are either archived or clearly marked as historical.
- Every roadmap item links to a proof command or test.
- Counts in docs are dated and specify how they were measured, and use the canonical figures: 35 workspace members, `TOOL_COUNT=37`, ~270 serve routes, 10 providers, 10 TUI tabs, `Engram` (no `struct Signal`).

## Delete/Archive Done

- A module is not removed until a grep proves no runtime references remain.
- A demo is not archived until README/CI no longer point to it.
- A state path is not removed until migration tests cover old and new layouts.
- A stub is not kept in default build unless it returns a clear unsupported error.
