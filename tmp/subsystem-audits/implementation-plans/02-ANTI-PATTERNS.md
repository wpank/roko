# 02 — Anti-Patterns and Do-Not-Do Rules

These rules apply to **every** plan in this folder. They distill failures
recurring in the 661-batch runner output, the 05-01 deep audit, and the prior
subsystem audits. A change that violates any of these is a reject regardless
of how clean the code looks.

If a plan in this folder seems to ask you to violate one of these rules, the
plan is wrong; pause and report.

---

## The Pre-Commit Gate

Before every commit, the following must pass with no warnings:

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

If any step fails, fix the issue or revert. Do not bypass with `--no-verify`.
Do not silence clippy with `#[allow(...)]` to ship.

For frontend changes:

```bash
cd demo/demo-app
yarn lint
yarn typecheck
yarn build
```

The user's rule: **always use `yarn`, not `npm`.**

---

## Global Anti-Patterns

### 1. Skeletons ≠ migrations

A new type / trait / module compiles. That is not the same as the runtime
using it. A migration is only complete when:

- The new type is constructed in the actual product path (not just tests).
- The old type is **removed** (or feature-gated and routed only from tests).
- A focused integration test asserts the old code path is unreachable.

Example violation: "I added `DispatchPlan`." But the chat REPL still uses
`dispatch_direct.rs`. The migration is **partial**, not done.

### 2. Unknown ≠ zero

Missing usage / cost / context / token counts must remain `None`
(`Option<u64>` / `Option<f64>` / `Option<RoutingContext>`). Never substitute
`0`, `0.0`, or `RoutingContext::default()` for "I didn't get a value."

Example violation: an Anthropic streaming response without a `usage` block
becomes `UsageObservation { input_tokens: Some(0), ... }`. This poisons
cost telemetry and routing learning.

The router has two APIs: `record_confidence_outcome(model, success)` for
confidence-only updates and `observe_multi_objective(ctx, ...)` for real
contextual updates. **Never** call the contextual API with synthetic
`RoutingContext::default()`. Use the confidence-only API instead and label it.

### 3. No silent fallback

Failed resolution / auth / capability / config load must produce a typed
error. It must **not**:

- Synthesize a default config and proceed.
- Fall back to another provider/model "just to keep going."
- Downgrade a `Rejected` to a `Skipped`.
- Convert a `Failed` stream event to a `Completed` event with empty content.

Example violation: `DispatchResolver` returns `Unvalidated` for a missing API
key, dispatch proceeds, the request hits the wire, the provider returns 401,
the user sees "model error." It should reject with `MissingApiKey { provider,
env_var }` before any wire activity.

### 4. Missing or invalid config → restricted

When a contract YAML, safety profile, or auth config is missing or invalid,
the answer is **fewer permissions**, not more. Production code paths must use
`AgentContract::restricted(role)` (or equivalent). Permissive fallback is
test-only; mark it explicitly.

Example violation: `contract_for_role()` returns `permissive()` when the YAML
is missing — granting broader permissions than the configured contract would
have.

### 5. No regex prompt scraping

Never use a regular expression on PTY output to detect command success,
failure, or completion. Use typed `CommandEvent` lifecycle events:

- `Started` — command launched
- `Output` — bytes emitted
- `Exited { code }` — process finished
- `SpawnFailed { reason }` — process never started
- `Cancelled` — user cancelled

The demo automation in `demo/demo-app/src/lib/scenario-runners/` is the
canonical violation; plan 26 fixes it.

### 6. No string-interpolated payloads

`format!`-built TOML, JSON, SSE, or wire payloads are wrong. Use
`toml::to_string_pretty(&struct)`, `serde_json::to_string(&struct)`,
`axum::Json(...)`, etc.

Example violations:

- `routes/agents.rs::create_agent` builds the manifest with `format!(r#"...prompt = {prompt}\n..."#, prompt = toml_quote(...))`. T3-27 replaces this with a structured serialization.
- ACP `session/update` was constructed by hand. Now uses a typed `SessionUpdate` struct.
- SSE events should be `data: {json}\n\n` written via `axum::response::sse::Event::data()`, not `format!`.

### 7. No new dispatch path

The codebase already has four LLM dispatch paths: `ModelCallService`,
`DispatchResolver`, `dispatch_direct.rs` (legacy), and route-local
`reqwest::Client` constructions in `roko-serve`. **Adding a fifth is
forbidden.**

If you need new behavior:

- Extend `ModelCallService` (preferred).
- Extend `DispatchResolver` to validate / select / configure differently.
- Add a provider adapter behind the existing trait surface.

If a plan in this folder appears to ask for a new dispatch path, the plan is
wrong.

### 8. One item per commit

Each commit / PR addresses one numbered item (T0-1, T2-16, etc.) or one
mechanical slice (e.g. one provider parser migration). If you discover a
follow-up while working, **split it** into a new task. Do not expand scope.

Example violation: while implementing T3-27 (path traversal in agent creation),
you also "fix" CORS in the same commit. CORS gets its own commit (T3-28).

### 9. No `unwrap()` / `panic!()` / `expect()` in changed code

Existing `unwrap()`s in unrelated code stay. New or touched lines must use
typed errors:

- `.context()` from `anyhow` for top-level orchestration.
- Custom `thiserror` enums for crate-public error surfaces.
- `?` propagation, never `.unwrap()`.

Test code is exempt. Production code is not.

### 10. No unrelated edits

Do not refactor neighbors, add comments, change formatting in unmodified
lines, or "improve" adjacent code. The diff for one task should touch only
the files / lines required by that task.

Example violation: while wiring `KnowledgeIngestionSink::with_ingestor()`
(T4-29), you reformat 600 lines of `commands/plan.rs`. Reject the diff.

---

## Subsystem-Specific Anti-Patterns

### Dispatch / Provider

- **No raw `reqwest::Client::new()` for model dispatch in serve routes.** Use
  `state.model_call_service` or accept the route belongs in
  `roko-agent`-managed code.
- **No `ANTHROPIC_API_KEY` envvar reads in roko-cli or roko-acp.** Auth lives
  in `roko-agent`. Other crates pass through.
- **No `ProviderKind::ClaudeCli → Anthropic API` silent mapping.** ClaudeCli
  selection must use the CLI; Anthropic API selection must use the API; if
  the combination is invalid, return a typed error.
- **No "sticky" provider state across `/model` switches.** Each switch must
  be atomic; failed resolution must leave previous state intact.

### Config / Safety

- **No `runner.dangerously_skip_permissions = true` outside local override.**
  The strict validator catches this. Don't try to work around it.
- **No `permissive()` outside test code or explicit local override.** Mark
  test helpers `#[cfg(test)]`.
- **No env var reads bypassing the config layer.** New config goes in
  `RokoConfig` with serde defaults and an env-var binding.
- **No silent config schema drift.** Adding a field requires adding to
  `mask_secret_fields` (if secret), adding to the strict validator (if
  dangerous), and updating tests.
- **No serializing secrets to TOML / JSON without masking.** `mask_secret_fields`
  is the chokepoint.

### Runtime / Gates / Artifacts

- **No `_ => {}` catch-all in `Rung` matches.** Always exhaustive. Adding a
  new rung must cause a compile error in every match arm. T1-10 made this
  explicit; do not regress.
- **No "log alert and continue" for gate SPC alerts in correctness paths.**
  Either fail-closed (block) or fail-open with explicit user-facing notice.
  Don't silently log.
- **No commit reporting `noop` hash + `success` status.** Use
  `CommitOutcome::NoChanges` (T1 / R-track).
- **No artifact validity tracked beside process success.** It's a workflow
  outcome via `ArtifactOutcome`. Required artifact invalid → workflow not
  successful.

### Telemetry / Learning

- **No `Vec::push` to in-memory learning buffers without a drain path.**
  Every collect must have a periodic drain (T4-33: rotation; PG_01 already
  drained efficiency events).
- **No `EpisodeSink` write without `model` and `provider` populated.** T1-8
  fixed this; do not regress.
- **No `KnowledgeIngestionSink::at()` without `.with_ingestor()` in the
  product path.** Without an ingestor, the sink is write-only.
- **No new `FeedbackSink` without a `interested(event)` predicate.** This
  is the hot path; broad subscribers slow every event.
- **No JSONL writer without a rotation policy.** `>10 MiB` rotates with
  `.jsonl.1..5` ring (T4-33).

### Terminal / Demo

- **No prompt regex.** See anti-pattern #5.
- **No "wait for output to look idle" heuristics.** Use `Exited` event.
- **No leaked PTY processes / ZDOTDIR temp dirs on cancel.** The lifecycle
  events guarantee `Cancelled` fires; clean up there.
- **No "reconnect resets terminal state" hack.** Use a typed lifecycle state
  and refuse to attach an old generation to a new socket.

### Frontend (demo-app)

- **No `npm`.** Always `yarn`.
- **No prompt scraping in scenario runners.** Consume `CommandEvent`.
- **No polling on `GET /api/work/...` when SSE is available.** Subscribe to
  `GET /api/stream/{id}`.
- **No hardcoded localhost URLs.** Use `serve-url.ts` helper.

### Orchestrate.rs

- **No new helper appended to `orchestrate.rs`.** New code goes in a
  focused module under `crates/roko-cli/src/orchestrate/<topic>.rs` (or in
  another crate if cross-cutting). Plan 20 sets the structure.
- **No additional `dispatch_agent_with_<variant>()` clones.** If you need a
  different code path, extract a strategy/trait, do not clone the function.
- **No new parameter to functions with >8 parameters.** Use a typed request
  struct.

---

## "Looks Right but Isn't" Checklist

If you see one of these in a code review, push back. They look like progress
but are not.

| What you see | Why it's wrong | What to do |
|---|---|---|
| New type with `// TODO: wire` | Skeleton, not migration | Land it only if the wiring also lands in the same PR |
| `// Currently a no-op; will fill in later` | Silent fallback | Either implement or remove; never leave a no-op in product path |
| `Default::default()` for `RoutingContext` / `UsageObservation` | Unknown→zero | Use `Option<...>` or confidence-only API |
| `format!("data: {}\n\n", json)` | String-interpolated wire | Use `axum::response::sse::Event::data` |
| `if let Ok(_) = std::env::var(...)` | Hidden auth dependency | Read env in `roko-agent` config builder, not at call site |
| `match rung { Compile => ..., _ => skipped }` | Catch-all hides drift | Exhaustive arms with explicit reasons |
| `let _ = result;` after a fallible call | Silent failure | Propagate or `tracing::warn!` with context |
| New file under `roko-cli/src/` mounted via `pub mod` with no caller | Dead module | Wire it or don't add it |
| `"text"` / `"content"` / `"type"` literal strings in JSON paths | Hand-built protocol | Use the typed DTO |

---

## When to Stop and Ask

Stop and ask the user (or report blocker) when:

- The plan in this folder appears to violate one of these anti-patterns.
- Implementing the change would require a new dispatch path / runtime / state
  machine.
- A test that should pass is failing for reasons outside the plan's scope.
- A commit would touch >2 unrelated subsystems.
- An item's "do not do" list rules out the only viable approach you can see.

It is much better to pause than to ship a violation.

---

## Where to Look When Tempted

- "How does X work today?" → grep, then read the cleanest example in
  `01-CONTEXT.md`'s "Where to look for examples" table.
- "Is this in scope?" → re-read the originating plan's "Do not" section.
- "Is this dead code?" → run `rg <symbol> crates/` and ensure callers exist
  outside the same module / `#[cfg(test)]`.
- "Has someone already done this?" → check `git log --oneline | rg '<task-id>'`.
