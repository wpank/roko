# Safety & Agent System: Goals

## End State

Safety contracts and tool permissions derived from role config files. Fail-closed by default. Recovery actions actively invoked. Every provider goes through unified dispatch.

## Key Properties

- **Contracts from role config**: Tool permissions, invariants, governance rules read from role TOML.
- **Fail-closed default**: Missing contract = restricted fallback, not permissive.
- **Recovery actions wired**: Contract violations trigger retry/downgrade/abort/alert based on config.
- **One dispatch path**: All providers go through ModelCallService → ToolDispatcher → SafetyLayer.
- **Budget enforcement on all paths**: Per-task and per-session cost limits.

## What Exists Today

- 8 JSON contracts (`.yaml` extension, parsed via `serde_json`) — architect, auditor, auto-fixer, implementer, researcher, reviewer, scribe, strategist
- 6 `ProviderKind` backends registered in `adapter_for_kind()` + secondary agents (Ollama, ExecAgent) outside the adapter registry
- ToolDispatcher with layered pipeline (validation → tool selector → task filters → capability auth → safety + contract → hook chain → handler → truncate → scrub → recovery)
- Critical: `contract_for_role()` in `safety/mod.rs` fails open on missing JSON asset (uses `AgentContract::permissive`)
- Recovery actions wired at dispatcher level (`check_recovery()` called after each tool result) but NOT at orchestrator task level
- Optional safety budget (`None` by default in `SafetyLayer::with_defaults()`)
- 14 secret scrub patterns across 9 categories in `scrub.rs`
- Hook chain (`SafetyHookChain`) and tool selector (`ToolSelector`) built but optional — not attached by default

## From v2 UX Showcase (9 Scenarios)

- **PermissionScope panel** (right rail): 11 scope rows with auto/ask/deny tri-state toggles: File reads (auto, 412), Searches (auto, 38), Network fetches (deny), Edits in src/ (ask, 14), Edits in services/ (ask, 1), Deletions (deny), Shell·safe (auto, 22), Shell·network (ask, 2), Shell·write disk (ask, 4), git commit (ask), git push (deny). Per-scope call count.
- **PermissionRequest cards** (pipeline, incident, architect): Inline permission prompts with title, description, scope tags (e.g. "single edit", "auto-revert if tests fail", "tagged for fast review", "new dir keys/", "writes 2 files", "no network"), and action buttons.
- **Mode→safety mapping** (architect, follow): Mode change enforces restrictions — "mode → Architect · code edits disabled", "mode → Research · read-only · no writes".
- **Auto-revert scope** (incident): Permission allows "auto-revert if tests fail" — gate failure triggers rollback.
- **Handoff scope tags** (architect): When switching Architect→Code — "new branch: experiment/limiter-failsafe", "scoped to gateway/middleware/", "auto-commit per step".
- **Per-worktree isolation** (tournament): Each parallel agent in its own worktree with its own permission scope.

### Data Feeds Required
- `PermissionScope` — per-scope: id, label, allowed (auto/ask/deny), call_count
- `PermissionRequest` — title, description, scope_tags, options (name, kind: allow/reject)
- `ModeSafetyMapping` — mode → restricted_actions (e.g. architect → no edits)
- `WorktreeSafetyScope` — per-worktree: agent_id, branch, allowed_paths, denied_actions

## Gap

- Change contract loading to fail-closed
- Wire recovery action dispatch
- Make safety budget mandatory
- Connect contracts to role config system
- Per-action permission scoping with auto-approve toggles
- Mode→safety mapping (architect=read-only, research=no-edits)
- Per-worktree safety scope for parallel agents
