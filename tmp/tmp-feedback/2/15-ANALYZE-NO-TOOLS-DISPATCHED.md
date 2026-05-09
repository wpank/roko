# /analyze Says What It Would Do But Does Nothing

## Problem

`/analyze` in Zed ACP outputs:
```
🔬 Analyzing execution data
✓ Agent completed (1s, 137 bytes) [gpt54-mini]
I'll read the episodes file, compute the requested metrics, and write a markdown
analysis into .roko/research/execution-analysis.md.
```

No file is created. The agent said what it would do and stopped. Research INDEX.md shows
`_(none)_`.

## Root Cause

**Tool name alias mismatch: Claude CLI names passed to OpenAI provider.**

The research analyze command passes `allowed_tools: Some("Read,Write,Edit")` (line 683 of
`research.rs`). These are **Claude CLI PascalCase names**. But the OpenAI-compatible
provider filters tools using **canonical snake_case names** from the registry.

### The chain of failure:

```
research.rs:683     allowed_tools: Some("Read,Write,Edit")
                                ↓
agent_exec.rs       options.tools = Some("Read,Write,Edit")
                                ↓
openai_compat.rs:252  parse_allowed_tools_csv("Read,Write,Edit")
                      → HashSet{"Read", "Write", "Edit"}
                                ↓
openai_compat.rs:341-348  filter: allowed.contains(tool.name.as_str())
                          registry has: "read_file", "write_file", "edit_file"
                          allowed has:  "Read", "Write", "Edit"
                          → ZERO matches → empty tool list
                                ↓
tool_loop:            iterations=0, no tool calls, returns text only
```

### Log proof:
```
roko_cli::agent_exec: agent_exec: dispatching prompt model=gpt54-mini role=Some("researcher") provider=openai_compat prompt_len=315
roko_agent::tool_loop: tool_loop: stop — no tool calls, returning final text iterations=0 final_text_len=137 final_text_empty=false finish_reason=Some("stop") input_tokens=37628 output_tokens=38
```

Zero iterations, zero tool calls. The model had no tools to call.

### Why it works with Claude CLI but not OpenAI

The Claude CLI path builds `--tools Read,Write,Edit` directly — Claude CLI understands its
own PascalCase names natively. The OpenAI path must map through the tool registry, which uses
canonical `snake_case` names. The translation step is missing.

## Scope of Impact

This affects **every command that passes `allowed_tools` via the Claude CLI alias names**
when routed to a non-Claude provider (OpenAI, Gemini, Ollama, etc.):

```rust
// research.rs — all three research agent paths:
allowed_tools: Some("Read,Write,Edit")           // /analyze
allowed_tools: Some("Read,Write,Edit,Bash")      // /research
allowed_tools: Some("Read,Write,Edit,Bash,Glob") // /enhance-prd, /enhance-plan, /enhance-tasks
```

Every research command dispatched through gpt54-mini (or any non-Claude model) will have
zero tools available. The agent becomes text-only — it can only describe what it would do.

## The Alias Table

`crates/roko-core/src/tool/aliases.rs` has the mapping:

| Claude CLI name | Canonical name |
|-----------------|---------------|
| `Read` | `read_file` |
| `Write` | `write_file` |
| `Edit` | `edit_file` |
| `Bash` | `bash` |
| `Glob` | `glob` |
| `Grep` | `grep` |
| `WebFetch` | `web_fetch` |
| `WebSearch` | `web_search` |
| `Agent` | `task` |

## Fix

### Option A: Resolve aliases in `parse_allowed_tools_csv` (recommended, ~10 min)

**File:** `crates/roko-agent/src/provider/openai_compat.rs:252-260`

```rust
fn parse_allowed_tools_csv(csv: Option<&str>) -> Option<HashSet<String>> {
    use roko_core::tool::aliases::canonical_of_claude;

    let allowed: HashSet<String> = csv
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| {
            // Resolve Claude CLI aliases to canonical names.
            // If it's already canonical or unknown, pass through as-is.
            canonical_of_claude(name)
                .unwrap_or(name)
                .to_string()
        })
        .collect();
    (!allowed.is_empty()).then_some(allowed)
}
```

### Option B: Normalize at the call site (~5 min per caller, but fragile)

Change every `allowed_tools:` value in research.rs to use canonical names:
```rust
allowed_tools: Some("read_file,write_file,edit_file")
```

This works but is fragile — any new caller using Claude names will silently break.

### Option C: Accept both names in the filter (~5 min)

Add alias lookup in the filter itself:
```rust
.filter(|tool| {
    allowed.as_ref().is_none_or(|allowed| {
        allowed.contains(tool.name.as_str())
            || tool.claude_alias().is_some_and(|alias| allowed.contains(alias))
    })
})
```

**Recommendation: Option A** — fix it once in the parser so all callers work regardless
of which naming convention they use.

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-agent/src/provider/openai_compat.rs:252-260` | Resolve Claude aliases to canonical in `parse_allowed_tools_csv` |
| `crates/roko-core/src/tool/aliases.rs` | Already has `canonical_of_claude()` — no change needed |

## Priority

**P0** — This is the same class of bug as the search command (#13). Any non-Claude model
is completely unable to use tools in research commands. The agent has 37K tokens of context
but zero tools, so it just describes what it would do and exits. The fix is trivial (6 lines
in the parser) but the impact is total: all research, all analysis, all enhancement commands
are broken on OpenAI/Gemini/Ollama.
