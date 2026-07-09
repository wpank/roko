# 17 — Runner Review Gaps

Scope: runner outputs under `tmp/runners/arch`, `tmp/runners/converge`, `tmp/runners/converge-followup`, `tmp/runners/mega-parity`, `tmp/runners/post-parity`

This pass checked the runner instructions against the current code changes. The strongest pattern is that later runners knew the right anti-patterns, but their execution model still allowed exactly those anti-patterns to land as local "wiring" fixes.

## Findings

### HIGH: Runner rules banned raw provider HTTP, but raw provider HTTP still landed

`mega-parity/context-pack/00-RULES.md` and `post-parity/context-pack/00-RULES.md` both forbid raw provider HTTP in CLI/surface code. Current code still has direct provider clients in:

- ACP Anthropic streaming: `bridge_events.rs:1403-1600`;
- chat API dispatch: `chat_session.rs:501-625`;
- legacy direct dispatch: `dispatch_direct.rs` still present with `reqwest::Client::new()` hits from the grep scan.

The review gap is not that nobody wrote the rule. The gap is that the rule was not enforced as a repo-level invariant after cherry-picks.

2026-05-01 update: ACP Anthropic/OpenAI-compatible paths and
`ChatAgentSession::send_turn_api` have now been migrated to
`ModelCallService::stream`. The remaining raw-provider target in this finding is
legacy/direct dispatch plus serve/other compatibility paths. Keep this finding
open until the recurrence check is CI-blocking, because the original failure was
an enforcement failure, not just a missing local patch.

### HIGH: Runner rules banned dangerous permission defaults, but config reintroduced one

`post-parity/context-pack/00-RULES.md` says `dangerously_skip_permissions` defaults false and requires explicit opt-in. Current `roko.toml:1019-1021` sets it true.

This is the kind of issue a simple static check should catch. It should not rely on human audit after the fact.

### HIGH: Runner rules banned multiple session owners, but multiple dispatch owners remain

The codebase now has at least these active dispatch/protocol owners:

- `ChatAgentSession` CLI/API dispatch;
- ACP-specific Anthropic streaming dispatch;
- `dispatch_direct.rs` legacy direct dispatch;
- provider adapters / `ModelCallService`.

The session state may be moving toward one owner, but provider dispatch is still fragmented. The same concepts appear in multiple places: model slug, system prompt placement, messages array, API key resolution, HTTP client, usage extraction.

2026-05-01 update: chat and ACP are closer to thin consumers of
`ModelCallService::stream`, and shared prompt rendering now preserves
user/assistant boundaries for multi-turn history. `ChatAgentSession` still
performs API-key preflight for history semantics, and some adapters still receive
rendered prompt text rather than provider-native message arrays. The next
redesign should move auth/capability validation and provider-native message
handling into the provider/model-call owner instead of adding more surface
helpers.

### MEDIUM: Verification strategy encouraged local compile success over end-to-end proof

`converge-followup/context-pack/03-EXECUTION-STRATEGY.md` explicitly says not to run full workspace checks, not to run tests unless asked, and to prefer specific crate checks. `mega-parity/context-pack/05-NO-BUILD.md` and `post-parity/context-pack/00-RULES.md` go further and prohibit compilation/test commands.

That may have improved throughput, but it means integration regressions were structurally expected. Bugs like "Claude CLI selected but Anthropic API required" are not caught by per-crate type checking.

### MEDIUM: Review vetoes were descriptive, not executable

The runner packs contain good vetoes:

- no second provider resolution chain;
- no second prompt assembly path;
- no raw provider HTTP in CLI code;
- unknown usage is not zero;
- demo data must not look live;
- process success is not artifact success.

Current code still violates several. The missing piece is executable fitness functions in CI or pre-merge scripts. A markdown veto is not enough when hundreds of batches are cherry-picked.

### MEDIUM: Local write scopes fragmented ownership

Prompts constrained agents to small write scopes. That reduces merge conflicts, but it also prevents fixing root causes that cross module boundaries. For example, a batch touching ACP could not reasonably add streaming to the provider abstraction and update all consumers, so it patched Anthropic streaming locally.

The batch shape made duct-tape more likely: a narrowly scoped agent can only fix the symptom in front of it.

## Recommended Fitness Checks

Add automated checks before another runner wave:

1. `rg 'reqwest::Client::new\\(' crates/roko-cli crates/roko-acp crates/roko-serve` must allow only approved shared-client factories and tests.
2. `rg 'dangerously_skip_permissions\\s*=\\s*true' roko.toml crates/` must fail outside explicit test fixtures.
3. `rg 'ANTHROPIC_API_KEY|OPENAI_API_KEY|ZAI_API_KEY' crates/roko-cli crates/roko-acp` must fail outside config/auth/provider boundaries.
4. `rg '\"role\": \"system\"' crates/roko-cli crates/roko-acp` should flag direct Anthropic request construction outside adapters.
5. Static max function/file checks for known hotspots, especially `orchestrate.rs`, `bridge_events.rs`, and `chat_session.rs`.
6. End-to-end smoke checks for:
   - Claude CLI configured and no `ANTHROPIC_API_KEY`;
   - API key configured and Claude binary present but unauthenticated;
   - `/model bad-name` leaves previous model untouched;
   - terminal spawn failure closes with a typed error;
   - demo command failure comes from exit status, not prompt timeout.

## Fix Direction

Before more batch work, convert the markdown vetoes into executable gates. Then split future work by ownership boundary rather than by symptom. Provider dispatch should be fixed in the provider/model-call layer first, then ACP/chat should become thin consumers.
