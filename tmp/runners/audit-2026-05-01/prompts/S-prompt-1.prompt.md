# S-prompt-1: Audit prompt construction sites

## Task
Inventory every prompt construction site in the codebase. For each, note whether it uses the full `SystemPromptBuilder` (✓), uses a partial builder, or inlines a prompt string. Output a table to `logs/S-prompt-1-audit.md`.

## Runner Context
Runner audit-2026-05-01, group S. No dependencies. Wave 1.

## Source plan
`tmp/subsystem-audits/implementation-plans/30-prompt-assembly-completion.md` § PA-1.

## Why
Plan 30 PA-2..6 migrate inline-string sites to the builder. Without an inventory, we don't know what to migrate.

## Exact changes

This batch produces an audit document, not a code change.

### 1. Discover sites

```bash
rg 'SystemPromptBuilder|system_prompt =|format!\([^)]*system|let prompt = format!|fn build_(initial_)?prompt|build_prompt' crates/ -g '*.rs' -n
```

For each match, classify:

- **Full builder**: uses `SystemPromptBuilder::new()` and chains all 9 layers (or all needed for that mode).
- **Partial builder**: uses `SystemPromptBuilder::new()` but only some layers, or uses raw string composition for some layers.
- **Inline string**: uses `format!`, `String::from("...")`, raw string concatenation.

### 2. Output to `tmp/runners/audit-2026-05-01/logs/S-prompt-1-audit.md`

```markdown
# S-prompt-1: Prompt construction sites audit

Generated: 2026-05-01

## Summary

| Status | Count |
|---|---|
| Full builder | N |
| Partial builder | M |
| Inline string | K |

## Sites

| Site | File | Line | Status | Notes |
|---|---|---|---|---|
| `dispatch_agent_with` | `crates/roko-cli/src/orchestrate.rs` | ~14XXX | ✓ Full | Canonical site |
| `chat_inline::build_initial_prompt` | `crates/roko-cli/src/chat_inline.rs` | ~XX | ⚠ Partial | Identity + role inline; missing context layer |
| `acp::session::build_prompt` | `crates/roko-acp/src/session.rs` | ~XX | ⚠ Partial | TODO migration |
| `commands::research::build_prompt` | `crates/roko-cli/src/commands/research.rs` | ~XX | ✗ Inline | format! with hardcoded research instructions |
| `commands::conductor::spawn` | `crates/roko-cli/src/commands/conductor.rs` | ~XX | ✗ Inline | Hardcoded conductor system prompt |
| ...

## Migration order recommendation

1. Chat REPL initial prompt (S-prompt-2): partial → full builder.
2. Research command (separate batch): inline → builder.
3. Conductor spawn (separate batch): inline → builder.
4. ACP session (separate batch): partial → full.
```

### 3. Determine VCG auction status

```bash
rg 'vcg_auction|VcgAuction|vickrey' crates/roko-prompt/ crates/roko-cli/ crates/roko-learn/ -g '*.rs'
```

Note in the audit doc whether the VCG auction module exists, has callers, and should be deleted (PA-5 territory).

## Write Scope
- `tmp/runners/audit-2026-05-01/logs/S-prompt-1-audit.md` (new)

## Read-Only Context
- All `crates/`

## Verify

```bash
ls tmp/runners/audit-2026-05-01/logs/S-prompt-1-audit.md

wc -l tmp/runners/audit-2026-05-01/logs/S-prompt-1-audit.md
# Should be > 30 lines (real inventory)
```

## Acceptance Criteria

- Audit doc lists every prompt construction site with status + recommendation.
- Migration order recommended based on observed scope.
- VCG auction status noted.

## Do NOT

- Do NOT migrate any site in this batch (S-prompt-2 onwards do).
- Do NOT bundle with other S-prompt batches.
- Do NOT skip the VCG auction inventory.
- Do NOT change any source file.
