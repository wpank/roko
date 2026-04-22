# 26 - Enforcement and Runner Controls Redesign

Scope: turn the 05-01 audit and anti-pattern catalogs into executable controls. This is a plan for checks, CI gates, runner prompts, cherry-pick controls, and review checklists. It does not redesign provider/config/runtime internals; it blocks recurrence while those redesigns land.

## Control Architecture

Add three enforcement layers:

1. `scripts/roko-fitness-checks.sh`: fast grep/regex fitness checks. Runs locally, in runner batches, and in CI before expensive tests.
2. `scripts/layer_check.rs`: semantic architecture checks that need repo traversal, Rust-ish context, allowlist expiry, or cross-file ownership reasoning. It already runs in `.github/workflows/ci.yml` as `layer-check`; extend it instead of creating another Rust checker.
3. `scripts/product-path-proof.sh` plus proof manifests under `tmp/product-path-proofs/*.toml`: verifies "wired" claims by running or replaying live entry points and checking typed evidence, not just unit tests.

CI wiring:

- Add a `fitness-checks` job to `.github/workflows/ci.yml` before `test`: `bash scripts/roko-fitness-checks.sh`.
- Keep `layer-check`, but make architecture warnings that match this doc fail once the baseline is burned down.
- Add `product-path-proof` as a required job only when a PR changes dispatch, config, runtime, gate, prompt, feedback, ACP/chat/serve entry points, or docs status files.
- Store allowlisted baseline violations in `scripts/fitness/allowlist.toml`, each with `pattern_id`, `path`, `line_or_symbol`, `reason`, `owner`, and `expires`. No inline `grep -v random_path` drift.

## Fitness Checks

| Check | Lives in | Exact detector | Catches | Expected false positives / handling |
|---|---|---|---|---|
| Raw provider HTTP outside provider layer | `scripts/roko-fitness-checks.sh`; deeper owner check in `scripts/layer_check.rs` | `rg '(reqwest::Client::new|Client::builder|\\.post\\("https://api\\.(anthropic|openai)|/v1/(messages|chat/completions))' crates/ --type rust` then allow only `crates/roko-agent/src/provider/`, `crates/roko-agent/src/http.rs`, tests, and explicit allowlist entries | A1, A2, D1, F1, J1, J6, O1; 05-01/13, 14, 17, 18 | Generic HTTP clients in `roko-agent-server`, demos, GitHub MCP. Only fail provider-domain/API-message construction. Allow generic HTTP if it does not parse model responses or usage. |
| Duplicate SSE/provider stream parsers | `scripts/layer_check.rs` | Find `data:` parsing, `[DONE]`, `content_block_delta`, `message_delta`, `response.chunk()` loops outside provider/streaming modules | A2, G5, O1; 05-01/13, 18 | Non-model SSE from server events. Allow only if event type is not a provider model stream and file has `fitness.allow = "non_model_sse"` in allowlist. |
| Dangerous permissions | `scripts/roko-fitness-checks.sh` | `rg 'dangerously_skip_permissions\\s*[:=]\\s*true|with_dangerously_skip_permissions\\(true\\)' roko.toml crates/ scripts/ --type-add 'toml:*.toml' --type-add 'rust:*.rs'` | B1, I1, J5; 05-01/09, 15, 17, 22 | Tests that assert the flag behavior, local examples. Allowed only under tests/fixtures or local-only sample config with an expiry and reason. Root `roko.toml` is never allowed. |
| Env-var reads outside config/auth/secrets/provider boundaries | `scripts/roko-fitness-checks.sh` with path allowlist | `rg '(std::env::var(_os)?|env::var(_os)?)\\([^)]*(API_KEY|TOKEN|BASE_URL|ANTHROPIC|OPENAI|ZAI|CEREBRAS)' crates/ --type rust` | I2, I4, B3, F1; 05-01/05, 13, 17, 18, 22 | Legit reads in `roko-core/src/secrets`, `roko-core/src/config`, provider auth adapters, and CLI auth detection. Surface crates may request a resolved secret; they may not read provider env directly. |
| Unknown-to-zero telemetry | `scripts/roko-fitness-checks.sh`; semantic context in `scripts/layer_check.rs` | `rg 'unwrap_or\\((0|0\\.0)\\)|Usage::default\\(\\)|cost_usd:\\s*0\\.0|input_tokens:\\s*0|output_tokens:\\s*0|duration_(ms|secs):\\s*0' crates/ --type rust` scoped to usage/token/cost/duration/feedback/runtime/learning/provider files | C2, J3, AGENT-7; 05-01/13, 20 | Display formatting and true zero fixtures. Allow only in UI/display modules or tests named `zero_is_reported_zero`; collection boundaries must preserve `Option` or provenance. |
| Path-based modules | Extend existing `check_path_shared_modules` in `scripts/layer_check.rs` | Fail any `#[path = "..."]` in `crates/*/src` unless allowlisted; current known exceptions become expiring baseline debt | H4, F1, AGENT high-value check | Existing `roko-cli` inclusion of `scripts/layer_check.rs` and `roko-core` obs includes may require a temporary baseline. Any new `#[path]` fails immediately. |
| Sentinel success / fake success states | `scripts/layer_check.rs` plus grep prefilter | Prefilter: `rg '\"noop\"|success_noop|CommitDone\\s*\\{|process_success:\\s*true|artifact_valid|Complete \\{.*EndTurn' crates/ --type rust`; semantic check fails `CommitDone { hash: "noop" }`, `GenerationOutcome { process_success: true, artifact_valid: false }`, and normal completion emitted after known dispatch error | G2, G3, J4, AGENT-5; 05-01/13, 19, 21 | Tests documenting legacy behavior. Keep under tests only; production must use `CommitOutcome`, `ArtifactOutcome`, `StreamOutcome`, or equivalent typed status. |
| Function length and parameter count | New `scripts/fitness/function_size.rs` or integrated module in `scripts/layer_check.rs` using `syn` | Fail new or touched Rust functions over 200 nonblank lines or over 5 parameters; hard fail known hotspots over a ratcheted ceiling | E1, E2, J3; 05-01/08, 12, 17 | Generated code, tests, serde visitors, table builders. Allow with `#[allow(clippy::too_many_lines)]` only if also in fitness allowlist with expiry. Ratchet `dispatch_agent_with`, `bridge_events`, and `runner/types` downward by file/function budget. |
| Duplicate provider/gate/prompt dispatch owners | `scripts/layer_check.rs` | Provider: provider-domain HTTP and API body construction outside `roko-agent`. Gate: gate/rung string maps outside `roko-gate` registry. Prompt: ad hoc system prompt assembly in ACP/chat/serve/runner without `PromptAssemblyService`/`SystemPromptBuilder` call | A3, D1, H2, J1, J6, O1; 05-01/17, 18, 21 | Transitional wrappers that only adapt typed owner outputs to a surface protocol. Allow thin adapters if they do not own selection, execution, policy, or parsing. |
| Docs status drift | `scripts/docs-status-check.sh` called by fitness script | Scan `CLAUDE.md`, `tmp/subsystem-audits/**/*.md`, runner status docs for `Resolved|Done|Wired|LiveInAllProductPaths|ProvenByE2E`; require coverage vocabulary plus `Proof:` line naming command/test/manifest | I3, D2, H1, J2, AGENT-9; 05-01/10, 12, 17 | Historical audit quotations. Allow inside "Previous version" or "where it happened" sections; new status claims require proof. |
| Product-path proof | `scripts/product-path-proof.sh` and `tmp/product-path-proofs/*.toml` | Manifest fields: `claim`, `owner`, `entry_point`, `command`, `expected_event`, `expected_artifact`, `old_path_blocked`, `fitness_checks`. Script fails if command absent, output lacks typed evidence, or old path still reachable | D2, H1, H2, J2, J4, J6, L2; 05-01/12, 17 | Expensive live-provider checks. Support deterministic mock/provider replay only if it enters through the same product command and asserts the same typed event/result. |

## CI Gate Behavior

Stage 0: baseline inventory. Add scripts in non-blocking mode for one PR. They print violations grouped by pattern id and compare against `scripts/fitness/allowlist.toml`.

Stage 1: no-new-violations. CI fails if a changed line introduces a new violation for raw provider HTTP, dangerous permissions, env reads, unknown-to-zero, path modules, sentinel success, or duplicate dispatch owner. Existing allowlisted debt can remain only with owner and expiry.

Stage 2: hotspot blocking. CI fails if touched hotspot functions grow or remain over their ratchet ceiling. Initial ceilings: current line count for each known hotspot; target 200 lines. Each redesign PR must lower at least one touched hotspot ceiling.

Stage 3: full blocking. Required before another parallel runner wave. All checks are blocking except documented external-provider product proofs, which can be nightly if they require credentials.

Stage 4: release gate. No expired allowlist entries, no root dangerous permissions, no raw provider HTTP outside provider adapters, no docs status claim without proof, and at least one product-path proof for each changed core owner.

## Runner Prompt Rules

Every runner prompt must include this preflight block:

```text
Before editing:
1. Name the owning layer for the behavior: provider, gate, prompt, config, runtime, telemetry, or surface adapter.
2. Run the ownership search requested by the task and record existing implementations.
3. State which old path will be deleted, blocked, or explicitly marked legacy.
4. If the correct owner is outside your write scope, stop and report "scope mismatch"; do not patch the surface.

Hard rules:
- No raw provider HTTP/SSE outside roko-agent provider/streaming code.
- No dangerous permission defaults or shared config bypasses.
- No provider API key env reads outside config/secrets/auth/provider boundaries.
- Unknown telemetry stays optional until display.
- No status in strings, boolean pairs, sentinels, debug/rendered output, or prompt scraping.
- No no-op success. Report Changed, NoopAlreadySatisfied, NoopSkipped, or Failed.
- "Wired" requires a product-path proof manifest and a check that blocks recurrence.
```

Runner result schema must replace freeform success:

```text
status = Changed | NoopAlreadySatisfied | NoopSkipped | PartialBlocked | Failed
owner = provider | gate | prompt | config | runtime | telemetry | surface
product_proof = path-or-none
fitness = pass | fail | not-run-with-reason
old_path = deleted | blocked | legacy-explicit | still-reachable
```

Runners may not update status docs to `Wired`, `LiveInAllProductPaths`, `RetiredOldPath`, or `ProvenByE2E` unless the result includes `product_proof` and `old_path` is not `still-reachable`.

## Merge and Cherry-pick Controls

Cherry-pick waves must be treated as new artifacts, not as the sum of branch-local checks.

Required merge procedure:

1. Before the wave, run `bash scripts/roko-fitness-checks.sh`, `cargo run -p roko-cli -- layer-check`, and relevant product proofs; save as the baseline.
2. Cherry-pick one batch at a time. After each pick or conflict resolution, run the fast fitness script. If it fails, revert only the cherry-pick under review or fix within that batch; do not defer to a later batch.
3. If conflicts touch provider dispatch, config, prompt assembly, gate registry, runtime ledger, telemetry, or docs status files, require owner review before continuing the wave.
4. After the wave, run full CI locally or in a merge branch: fitness, layer-check, fmt, clippy, workspace tests, and product proofs for touched paths.
5. Produce `tmp/runner-merge-reports/<wave>.md` with branch SHAs, conflict files, fitness delta, expired allowlist entries, product proofs, and remaining old paths.

Hard merge blocks:

- New raw provider HTTP or provider SSE parser outside `roko-agent`.
- Any `dangerously_skip_permissions = true` in shared config or production defaults.
- New direct provider env-var reads in ACP/chat/serve/runtime surface code.
- New unknown-to-zero conversion in provider/runtime/learning collection.
- New `#[path]` sharing outside an expiring allowlist.
- New sentinel success or no-op success in workflow/gate/artifact/stream outcomes.
- New docs claim of `Resolved`, `Wired`, or `Done` without coverage status and product proof.

## Review Checklist

Use this checklist on PRs and runner wave reports:

| Area | Pass condition |
|---|---|
| Owner | The change modifies the canonical owner or a thin adapter. It does not add shared logic to a surface crate. |
| Duplicate path | At least one old provider/gate/prompt/config/runtime path is deleted, blocked, or explicitly legacy-only. |
| Dispatch | Model execution goes through `DispatchPlan`/`ModelCallService`/provider adapters, not raw HTTP or `dispatch_direct` fallback. |
| Safety | Dangerous permissions are false by default and local-only overrides require reason, expiry, and acknowledgement. |
| Config/env | Runtime uses resolved config/secrets; surface code does not synthesize providers or read provider env vars directly. |
| Telemetry | Unknown usage/cost/duration/context remains `None` or has provenance until UI display. |
| Outcomes | Gate, commit, stream, artifact, and workflow outcomes are typed; no caller special-cases `"noop"`, empty strings, or boolean pairs. |
| Size | Touched functions are under 200 lines and 5 params, or the PR reduces an allowlisted hotspot ceiling. |
| Product proof | A live entry point proves the behavior and observes a typed event/result/artifact. |
| Docs | Status uses `Built`, `WiredInOnePath`, `LiveInAllProductPaths`, `RetiredOldPath`, or `ProvenByE2E`, with a `Proof:` line. |
| Recurrence | A CI/fitness check fails if the anti-pattern returns. |

## Product-path Proof Manifests

Use one manifest per claim:

```toml
id = "dispatch-acp-streams-through-provider"
claim = "ACP streaming uses provider-owned stream API"
owner = "provider"
entry_point = "acp"
command = "cargo test -p roko-acp provider_stream_product_path -- --nocapture"
expected_event = "ModelStreamDelta { provider_id, final_model, usage: UsageObservation }"
expected_artifact = ""
old_path_blocked = ["crates/roko-acp/src/bridge_events.rs: no reqwest provider client"]
fitness_checks = ["raw-provider-http", "duplicate-sse-parser", "unknown-to-zero"]
```

Minimum proof set before claiming the 05-01 redesign fixed:

- ACP single-agent dispatch enters provider/model-call layer and preserves unknown usage.
- CLI chat `/model bad-name` leaves previous resolved model/provider untouched.
- Claude CLI configured without `ANTHROPIC_API_KEY` does not silently substitute Anthropic API.
- Workflow clean-tree commit returns typed `NoChanges`, not `CommitDone { hash: "noop" }`.
- Required unknown/judge/custom gate without implementation/config fails validation before execution.
- Generated artifact invalidity returns `ArtifactOutcome::Invalid`, not process success with side-field failure.
- Prompt assembly for chat/ACP/run consumes the shared prompt builder and playbook context.
- Demo terminal command success is derived from typed command/workflow events, not prompt timeout or transcript scraping.

## Acceptance Criteria

This enforcement redesign is implemented when:

1. `.github/workflows/ci.yml` has blocking `fitness-checks` and existing `layer-check` covers the semantic checks listed above.
2. `scripts/roko-fitness-checks.sh`, `scripts/docs-status-check.sh`, and `scripts/product-path-proof.sh` exist, are documented, and run without credentials for deterministic checks.
3. `scripts/fitness/allowlist.toml` contains only current baseline debt with owners and expiries; CI fails on new violations and expired entries.
4. Runner prompts and result schema include owner, old-path, no-op status, product proof, and fitness result fields.
5. Cherry-pick wave reports are required before merging runner waves and include post-merge fitness output.
6. Review checklist is added to the PR template or runner review packet.
7. No shared config or production default enables `dangerously_skip_permissions`.
8. Raw provider HTTP/SSE, unknown-to-zero telemetry, sentinel successes, and path-based module sharing cannot be newly introduced without a failing gate.
9. Status docs cannot claim `Resolved`, `Wired`, or `Done` without coverage vocabulary and product-path proof.
