# Anti-patterns specific to audit-2026-05-01 batches

These are in addition to the 10 universal anti-patterns in `00-RULES.md`.
The universal ones apply to every batch; these tier-specific ones catch
common per-tier mistakes.

## T2 — Delete dead code

T2-1. **Don't add `#[deprecated]` and ship.** Just delete. Deprecation
is for public APIs with external consumers; this code has none.

T2-2. **Don't comment out files.** Delete or `git rm` them.

T2-3. **Don't keep "for reference."** Git history retains what you delete.

T2-4. **Don't add new wiring to "justify" a module marked dead.** If a
module has no callers, it goes; do not "rescue" it by writing a caller
in the same PR.

T2-5. **One module deletion per commit.** Easier to bisect a regression.
Bundle only obviously-cohesive groups (e.g. all 4 orphan files in T2-16,
since none are in `lib.rs`).

T2-6. **Always run the pre-deletion safety check.** See `00-RULES.md` §
"Pre-deletion safety check." If step 1 produces hits, the module is
**not** dead — re-evaluate the audit claim before deleting.

T2-7. **Phantom field deletions also remove `roko.toml` entries.** Don't
leave orphan TOML keys.

T2-8. **Phantom field deletions also remove `hot_reload.rs` diff arms.**
Otherwise the next config-reload compares to a missing field and fails.

T2-9. **Phantom field deletions also remove `compat.rs` migration arms.**
Old configs no longer migrate phantom fields.

T2-10. **TUI display rows for phantom fields also go.** Search
`crates/roko-cli/src/tui/views/config_meta.rs` etc.

## T3 — Security hardening

T3-1. **Don't disable `validate_bind_safety`.** It is the chokepoint
that prevents accidental public binds without auth. New entry points
must call it.

T3-2. **Don't add a route that bypasses auth without explicit reason.**
If a route must be public (health probe, HMAC-verified webhook),
document why in a doc-comment.

T3-3. **Don't lower a stricter limit to make a test pass.** The test
should fit within the limit, or use a fixture below it.

T3-4. **Don't `allow_origin(Any)` outside `unsafe_public_cors = true`.**
Permissive CORS is opt-in only.

T3-5. **Don't use raw `format!` for any wire protocol.** TOML, JSON, SSE
payloads use typed serializers.

T3-6. **Body limits are per-route override-able.** Don't lower the
global limit to fix a specific endpoint — exempt that endpoint.

T3-7. **`/health` (top-level) is a contract.** Returns 200
unconditionally. Do NOT change it. Use `/api/health` for discriminating
probes.

T3-8. **Rate limiting must identify clients correctly.** Behind a load
balancer all clients share one IP. Use API key when present, fall back
to peer IP. Don't use `tower::limit::RateLimitLayer` (global token
bucket); use `tower-governor` or equivalent.

T3-9. **WebSocket caps are independent of HTTP body limits.** Set both.

T3-10. **`PORT` env var changes only the port, not the bind.** Cloud
platforms that need `0.0.0.0` set `serve.bind = "0.0.0.0"` explicitly.

T3-11. **Path canonicalization happens AFTER `create_dir_all`** for
`create_*` operations (path doesn't exist yet beforehand).

T3-12. **`Path::strip_prefix` is not a containment check.** It does not
resolve symlinks. Use `canonicalize().starts_with` instead.

T3-13. **`toml_quote()` does not prevent table injection** (the
`[malicious]\n` line in a prompt). Use structured serialization.

## T4 — Feedback loops

T4-1. **No new sink without a real consumer.** A sink that writes and
is never read is dead.

T4-2. **No `Default::default()` for `RoutingContext` / `UsageObservation`.**
Use `Option`. Confidence-only is the fallback.

T4-3. **No "shadow mode" learners.** If a learner doesn't influence
anything, delete or activate it.

T4-4. **No JSONL append without rotation.** All writers respect the
size bound (T4-33).

T4-5. **No `interested(event)` returning `true` for everything.** Hot
path; selective subscription. Sinks subscribe to the specific event
variants they consume.

T4-6. **Provider parser migrations: one provider per commit.** Easier
to bisect.

T4-7. **JSON missing field** = `None`; **JSON `null`** = `None`;
**JSON `0`** = `Some(0)`. The distinction matters for cost telemetry.

T4-8. **`/model` switch must be atomic.** Build everything in a temp
struct; commit to `self` only after success.

## T5 — Architectural extraction

T5-1. **Pure mechanical moves first; behavior changes second.** When
extracting a function, the first commit moves code with no logic
change. Later commits refactor.

T5-2. **No "incremental" public API leaks.** A new `pub fn` introduced
as part of an extraction must be the *intended* permanent surface, not
a transient shim. If transient, prefix with `pub(crate)` or `pub(super)`.

T5-3. **Compatibility adapters are the deletion list.** Every "compat
shim" landed must have a tracked deletion follow-up. No permanent compat.

T5-4. **One slice per commit.** Slices must build between commits;
`git bisect` should land on the slice that introduced a regression.

T5-5. **Don't add `pub` to internal helpers during a move.** If a
helper is pulled into the new module and not needed externally, keep
it `fn` (private) or `pub(super)`.

T5-6. **Don't change function signatures of helpers called by moved
code.** If a helper is called from inside the moved block, the new
module imports it; the helper itself stays put.

T5-7. **Don't introduce `async fn` where the original was sync, or
vice versa.** Mechanical move, not refactor.

T5-8. **Don't merge two slices into one commit** even if they share a
helper.

T5-9. **Don't add new error variants in extraction PRs.** If the
function returned `anyhow::Error`, the new module returns
`anyhow::Error`. Future PR introduces typed errors.

T5-10. **Don't drop logs.** Every `tracing::info!` / `warn!` / `error!`
in the moved block stays in the new module verbatim.

T5-11. **Don't change variable names** in moved code. `let cfg =` stays
`let cfg =`. Renames go in a follow-up PR.

T5-12. **`dispatch_direct.rs` deletion is staged.** Step 1: feature-gate.
Step 2: migrate production callers to `ModelCallService`. Step 3:
verify default build excludes the module. **Don't** delete the module
file in this runner; that's a follow-up after CI fitness flags it green
for 30 days.

## S — Subsystem cross-cutting

S-1. **No fifth dispatch path** (universal #7, repeated for emphasis).

S-2. **`Unvalidated` diagnostics are a smell.** Any `DispatchResolver`
return that's `Unvalidated` should become a typed error in plan 21
ACP-1 / S-acp1.

S-3. **No env-var read outside the config layer.** Consumers receive
resolved values; they don't read env vars themselves.

S-4. **`load_config()` returns `ValidatedConfig`** after S-config phase 3.
Callers receive `ValidatedConfig`; they call `.config()` to get the
underlying `RokoConfig`.

S-5. **Ledger writes are correctness-critical.** A gate verdict / artifact
outcome write failure surfaces as `WorkflowOutcome::LedgerFailure`, not
a log-and-continue.

S-6. **Recovery actions are invoked from the dispatch failure path.**
S-safety-2 wires this. Don't define new actions; wire existing YAML.

S-7. **Demo automation: success = `Exited.code`, not regex on output.**
S-term batches must not introduce new prompt regexes.

S-8. **Each gate runner returns one of `Passed`, `Failed`, `Skipped`,
`Error`.** Never "I don't know." Never "Passed unless catastrophic."

S-9. **Cognitive layer cleanup deletes pheromones entirely** (~68K LOC).
Don't migrate features into neuro just to keep the ideas alive. If a
feature is real, it has callers; if it has no callers, it goes.

S-10. **CI fitness allowlist requires owner / reason / expiry.** Don't
allowlist a category by regex; each entry is a specific file/pattern
pair.

## How to recover from a stuck batch

If you find that the prompt's described state doesn't match the current
worktree (e.g. a function moved by 200 lines, a file is gone, a different
helper now exists):

1. **Don't improvise.** Don't try to figure out the new state and apply
   the change anyway.
2. **Re-grep** for the named symbol. Capture the new line number.
3. **Read the linked plan** in `tmp/subsystem-audits/implementation-plans/`
   for context.
4. **If the change is still applicable**: update your batch log with
   the new line numbers and proceed. The plan is the source of truth;
   the prompt is a faster summary.
5. **If the change is no longer applicable**: log "task no longer
   applicable: <reason>" and exit with status `obsolete`.
6. **If the change is partially applicable**: log "task partially
   applicable: <reason>" and apply only the parts that still make
   sense. Note the gap so the issue tracker can be updated.

The runner respects exit codes: `obsolete` and `partial` are tracked
distinctly from `success`. Don't fake success.
