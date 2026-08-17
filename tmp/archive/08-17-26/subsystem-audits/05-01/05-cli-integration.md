# 05 — CLI Integration Gaps

## HIGH: CLI chat and ACP are completely disconnected

The CLI chat system (`chat_inline.rs`, `chat_session.rs`) and the ACP system (`roko-acp/`) are two independent workflows:

| Feature | CLI Chat | ACP |
|---------|----------|-----|
| Session store | In-memory per REPL | `.roko/sessions/{id}.json` |
| Provider dispatch | Claude CLI subprocess OR API | API (Anthropic/OpenAI-compat) |
| Conversation history | Session-scoped | Persistent per session |
| Auth | `auth_detect.rs` probing | ANTHROPIC_API_KEY env var |
| Pipeline | None (single-turn) | express/standard/full workflow |

No way to:
- Resume an ACP session from CLI (`roko chat --session <acp-id>`)
- Share auth tokens between CLI and ACP
- Use ACP's workflow pipeline from CLI chat

---

## MEDIUM: `CursorAcp` provider defined but not usable in CLI chat

**File:** `crates/roko-core/src/agent.rs:151`

```rust
AgentBackend::Cursor => ProviderKind::CursorAcp,
```

`CursorAcp` is defined as a `ProviderKind` and referenced in `dispatch_v2.rs`, but the chat session only checks:

```rust
fn is_cli_provider(&self) -> bool {
    self.model_selection.provider_kind == ProviderKind::ClaudeCli.label()
}
```

Any non-ClaudeCli provider falls through to the HTTP API path. If a user configures Cursor as their provider, the chat session will try to call it as an OpenAI-compatible HTTP endpoint, which is wrong — Cursor uses the ACP protocol.

---

## MEDIUM: Auth detection doesn't know about ACP

**File:** `crates/roko-cli/src/auth_detect.rs`

Auth detection probes for:
1. Claude CLI (`claude --version`)
2. ANTHROPIC_API_KEY env var
3. ZAI_API_KEY (Zhipu)
4. OPENAI_API_KEY

No ACP-specific detection. If roko is running inside an ACP-compatible editor, it should detect that auth is handled by the editor's protocol.

---

## LOW: ACP pipeline phases hardcoded to Claude CLI

**File:** `crates/roko-acp/src/runner.rs`

When the workflow engine runs (express/standard/full templates), it spawns agents via `ClaudeCliAgent`. The Strategist, Implementer, AutoFixer, and Reviewer phases all use Claude CLI subprocess execution.

This means:
- Workflow pipeline always needs `claude` CLI installed
- Can't use API-only providers for pipeline phases
- `ANTHROPIC_API_KEY` won't work for the pipeline (only for single-prompt non-workflow path)

---

## LOW: `roko chat` and `roko acp` can't coexist

Both claim exclusive access to stdio/stdout. Running `roko chat` in a terminal while Zed has `roko acp` active in the same workspace creates competing session states.
