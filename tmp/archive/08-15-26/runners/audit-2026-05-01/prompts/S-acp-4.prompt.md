# S-acp-4: Rename or remove stale dispatch wrappers

## Task
Inspect `roko-acp` for legacy wrapper functions that look like raw HTTP/SSE sites (`run_anthropic_cognitive_task`, `run_openai_cognitive_task`, etc.) but actually delegate to `ModelCallService::stream`. Rename them to clarify or delete if unused. Update fitness allowlist.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-acp-1. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/21-acp-protocol-completion.md` § ACP-4.

## Why
The fitness inventory still flags these wrappers as potential raw-HTTP sites. Their names suggest they own the wire dispatch. After D6, they delegate to `ModelCallService::stream`. Either rename to reflect that or delete if redundant.

## Read first

```bash
rg 'run_(anthropic|openai|claude_cli|gemini)_(cognitive_task|stream|agent|prompt)' crates/roko-acp/ -n
```

For each match:

- **Still has callers**: rename. The new name should be `dispatch_via_model_call_service` or just inline at the caller.
- **No callers**: delete.
- **Misleading name only**: rename to clarify.

## Exact changes

### Per wrapper:

If the wrapper just builds a `ModelCallRequest` and delegates:

```rust
// Before
async fn run_anthropic_cognitive_task(req: AcpPromptRequest) -> Result<...> {
    let model_req = ModelCallRequest { ... };
    self.model_call_service.stream(model_req).await
}
```

Two options:

(a) **Inline** at the (single) caller:

```rust
// in session.rs caller:
let model_req = ModelCallRequest { ... };
self.model_call_service.stream(model_req).await
```

(b) **Rename** if the wrapper has multiple callers or genuinely encapsulates request-construction logic:

```rust
async fn dispatch_acp_prompt_stream(req: AcpPromptRequest) -> Result<...> {
    let model_req = ModelCallRequest { ... };
    self.model_call_service.stream(model_req).await
}
```

The new name should be **provider-agnostic** (`dispatch_acp_prompt_stream`, not `dispatch_anthropic`).

### Update callers

Update every call site of the renamed function. Use whatever IDE / `cargo check` provides; the rename should propagate.

### Update fitness allowlist

```bash
ls scripts/fitness/allowlist.toml 2>&1
```

If S-ci-1 (allowlist scaffolding) has landed, edit `scripts/fitness/allowlist.toml`: remove (or shrink) any entries that allowlist `run_anthropic_cognitive_task` etc., since the names are now inert.

If S-ci-1 hasn't landed, log "fitness allowlist update pending S-ci-1."

## Write Scope
- `crates/roko-acp/src/bridge_events.rs`
- `crates/roko-acp/src/session.rs`
- `scripts/fitness/allowlist.toml` (only if exists)

## Read-Only Context
- `crates/roko-agent/src/model_call_service.rs`

## Verify

```bash
rg 'run_anthropic_cognitive_task|run_openai_cognitive_task|run_claude_cli_cognitive_task' crates/ -g '*.rs'
# Expect: 0 hits, OR all hits are #[deprecated] / inside doc comments

# Replacement names present
rg 'dispatch_acp_prompt|dispatch_via_model_call_service' crates/roko-acp/ -g '*.rs'
# Expect: appropriate hits if rename was the chosen path
```

## Acceptance Criteria

- No production code uses `run_*_cognitive_task` names.
- Renamed functions (if any) reflect provider-agnostic dispatch.
- Fitness allowlist updated (or noted as pending).

## Do NOT

- Do NOT bundle with S-acp-1/2/3.
- Do NOT introduce new dispatch logic during the rename.
- Do NOT keep both old and new names. Pick one; rename.
- Do NOT delete a function that has external callers (check first).
