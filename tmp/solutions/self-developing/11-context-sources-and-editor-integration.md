# 11: Context Sources & Editor Integration

## Problem 1: "Point roko at stuff"

You want to say: "here are some folders and files — generate plans from them." Currently impossible. `prd plan` reads exactly one PRD markdown file. There's no way to pass arbitrary context.

### What Should Exist

```bash
# Point at a folder of specs
roko plan --context tmp/solutions/self-developing/

# Point at multiple sources
roko plan --context tmp/solutions/self-developing/ --context crates/roko-acp/src/ \
          "Implement the unified CLI UX from these specs"

# Point at specific files
roko plan --context 09-unified-cli-ux.md --context 01-model-config-ux.md \
          "Implement model auto-inference and the 3-verb CLI"

# Just a glob
roko plan --context "tmp/solutions/**/*.md" "Generate tasks for all of these"
```

---

## The Complete ACP Context Resolution Pipeline

### Types (crates/roko-acp/src/types.rs)

The `SessionPromptParams` struct is what Zed sends on every message:

```rust
pub struct SessionPromptParams {
    pub session_id: String,
    pub prompt: Vec<ContentBlock>,    // blocks of text, files, resource-links, images
    pub include_context: bool,        // if true: resolve @-mentions in text too
}
```

`ContentBlock` is the union of everything an editor can attach:

```rust
pub enum ContentBlock {
    Text { text: String },
    Resource { resource: ResourceRef },      // explicit file attachment (full content)
    ResourceLink { uri, name, description }, // reference-only attachment (linked, not embedded)
    Image { mime_type, data },
    Diff { path, diff },
}

pub enum ResourceRef {
    File { uri: String },   // file:// URIs from the editor
}
```

**Key difference between Resource and ResourceLink:**
- `Resource { resource: ResourceRef::File { uri } }` — Zed sends the file URI, roko resolves and reads it.
- `ResourceLink { uri, name }` — Zed sends a link without embedding the content. Roko resolves it the same way as Resource blocks (this is the fix applied in the `resource_link` crash session: `bridge_events.rs:3596-3608`).

### Entry Point (crates/roko-acp/src/bridge_events.rs, line ~990)

When Zed sends a prompt, `handle_session_prompt` runs:

```
params.include_context controls which path runs:

true  → resolve_context_items(&params.prompt, workdir)
        — reads Resource blocks AND resolves @-mentions in text
false → extract_resource_uris(&params.prompt) + read_file_context(uris, workdir)
        — reads Resource/ResourceLink blocks only; ignores @-mentions
```

```rust
// bridge_events.rs:1000-1046
let should_resolve_context = !is_slash_command && pipeline_template.is_none();

let file_context = if should_resolve_context {
    if params.include_context {
        resolve_context_items(&params.prompt, workdir).await     // line 1014
    } else {
        let uris = extract_resource_uris(&params.prompt);        // line 1016
        if uris.is_empty() {
            String::new()
        } else {
            read_file_context(&uris, workdir)                    // line 1020
        }
    }
} else {
    String::new()
};

// Then build system prompt and inject file_context + knowledge_context into it:
let mut full_system = session.build_system_prompt(workdir, &[], ...);
full_system = append_context(&full_system, &file_context);      // line 1041
full_system = append_context(&full_system, &knowledge_context); // line 1042
let messages = session.build_messages_array(&full_system, &prompt_text);
```

### resolve_context_items (bridge_events.rs:3571)

The main context resolution function. Called when `include_context = true`.

```rust
pub(crate) async fn resolve_context_items(
    prompt: &[ContentBlock],
    workdir: &Path,
) -> String {
    let mut parts = Vec::new();

    for block in prompt {
        match block {
            // Case 1: Explicit file attachment from Zed's file tree
            ContentBlock::Resource { resource: ResourceRef::File { uri } } => {
                match resolve_file_uri(uri, workdir).await {
                    Ok(content) => parts.push(content),
                    Err(e) => warn!(...),
                }
            }
            // Case 2: Text block — scan for @-mentions
            ContentBlock::Text { text } => {
                for label in extract_at_mentions(text) {
                    match resolve_at_mention(&label, workdir).await {
                        Ok(content) => parts.push(content),
                        Err(e) => warn!(...),
                    }
                }
            }
            // Case 3: ResourceLink — treat like explicit file (the fix from this session)
            ContentBlock::ResourceLink { uri, name, .. } => {
                match resolve_file_uri(uri, workdir).await {
                    Ok(content) => parts.push(content),
                    Err(e) => warn!(...),
                }
            }
            // Images and diffs: silently skipped
            ContentBlock::Image { .. } | ContentBlock::Diff { .. } => {}
        }
    }

    parts.join("\n\n")
}
```

### resolve_file_uri (bridge_events.rs:3617)

Strips the `file://` prefix, validates that the path is within the workdir (security), reads the file, truncates at 32KB:

```rust
async fn resolve_file_uri(uri: &str, workdir: &Path) -> anyhow::Result<String> {
    let path_str = uri.strip_prefix("file://").unwrap_or(uri);
    let (rel_path, contents) = resolve_local_file_contents(Path::new(path_str), workdir).await?;
    Ok(format!("<file path=\"{}\">\n{}\n</file>", rel_path.display(), contents))
}
```

The `resolve_local_file_contents` function (bridge_events.rs:3670) enforces workdir containment by canonicalizing both paths and checking `starts_with`. Any path outside workdir returns an error.

### resolve_at_mention (bridge_events.rs:3627)

Dispatches based on the mention label:

| Label | Resolution |
|-------|-----------|
| `@branch-diff` or `@diff` | `git diff` output, truncated to 10KB |
| `@recent-commits` or `@git-log` or `@log` | `git log --oneline -20`, truncated to 10KB |
| `@status` or `@git-status` | `git status --short`, truncated to 10KB |
| `@<any other string>` | treated as a relative file path, resolved like a Resource block |

All git commands run with `current_dir(workdir)`.

### extract_at_mentions (bridge_events.rs:3703)

Parses `@label` from raw text. Rules:
- A `@` preceded by an alphanumeric, `_`, `-`, or `.` character is NOT a mention (prevents `foo@bar.com` matching).
- The label ends at whitespace, `@`, `,`, `;`, `:`, `!`, `?`, `)`, `]`, `}`, `<`, `>`.
- Leading/trailing punctuation is trimmed from the label.

Examples:
- `"fix @src/main.rs and @branch-diff"` → `["src/main.rs", "branch-diff"]`
- `"sent from foo@bar.com"` → `[]` (email address, skipped)

### How It Assembles into the System Prompt

After context is resolved, `build_system_prompt` (session.rs:484) builds the 9-layer prompt via `SystemPromptBuilder` (roko-compose), and then `append_context` from `knowledge.rs:122` splices the file context and knowledge store results onto the end of the system prompt. The final messages array sent to the model looks like:

```
system:    [role identity] + [conventions] + [domain] + ... [9 layers] + \n\n<file path="...">...\n<knowledge hits>
user:      <current prompt text>
assistant: (prior turns from conversation_history)
user:      <current prompt text again at the end>
```

### Multi-Turn Context Accumulation in ACP Sessions

Session history is managed in `AcpSession` (session.rs):

```rust
const MAX_HISTORY_TURNS: usize = 40;
const MAX_HISTORY_CHARS: usize = 64_000;
```

Every user/assistant turn is pushed via `push_user_turn` / `push_assistant_turn`, then trimmed FIFO. This means long sessions lose the oldest turns, not the newest.

For CLI-based providers (Claude CLI), `build_history_context_for_cli` (session.rs:603) renders history as XML:

```xml
<conversation_history>
<user>
<prior message>
</user>
<assistant>
<prior response>
</assistant>
</conversation_history>
```

**Gap:** File context attached in turn N is NOT automatically carried forward to turn N+1. If you attach `@src/lib.rs` in one message and ask a follow-up in the next message, the file is gone. The current design treats each turn's file context as ephemeral — injected into the system prompt for that turn only. To fix this, a `pinned_context: Vec<ResolvedFile>` field on the session would need to persist resolved files across turns.

---

## How @-mentions Work: Zed → ACP → Model

1. In Zed's Composer, user types `@src/main.rs` or selects a file from the file tree.
2. Zed converts file-tree selections into `ContentBlock::Resource` or `ContentBlock::ResourceLink` blocks, and typed `@path` mentions are left in the `Text` block as-is.
3. Zed sends `session/prompt` with the assembled `Vec<ContentBlock>` and `include_context: bool`.
4. `handle_session_prompt` in `bridge_events.rs` calls `resolve_context_items` if `include_context` is true.
5. Each block type is dispatched to the appropriate resolver.
6. Resolved content is appended to the system prompt, NOT the user message.
7. The model sees `<file path="src/main.rs">...</file>` in its system prompt.

**When `include_context` is false:** Only explicit Resource/ResourceLink blocks resolve (not text @-mentions). This is the default when Zed does not pass `includeContext: true`.

### ResourceLink Fix (from this session)

Before the fix, `ContentBlock::ResourceLink` blocks fell through the match without resolving. Zed sends `resource_link` blocks when a file is "linked" (referenced without embedding). The fix adds an explicit arm in `resolve_context_items` at line 3596, and `extract_resource_uris` at line 3516 also now returns ResourceLink URIs alongside Resource URIs. Both cases now call `resolve_file_uri`.

---

## Design for `--context` Flag

### CLI Surface

```
roko plan --context <path>...   [--context-budget <tokens>] "prompt"
roko do   --context <path>...   "prompt"
roko run  --context <path>...   "prompt"
```

Multiple `--context` flags allowed. Each can be:
- A file path (`.md`, `.rs`, `.toml`, `.ts`, `.json`, `.py`, etc.)
- A directory (recursively walked)
- A glob pattern (shell-expanded or internally expanded)

### Folder Walking

No `walkdir` crate is currently used in the codebase. The implementation would use `walkdir` or `std::fs::read_dir` recursively:

```rust
struct ContextFile {
    path: PathBuf,
    content: String,
    token_estimate: usize,
}

fn load_context_files(
    paths: &[PathBuf],
    budget: usize,          // e.g. 50_000 tokens
    workdir: &Path,
) -> Vec<ContextFile> {
    let mut files = Vec::new();
    let mut used = 0usize;

    for root in paths {
        if root.is_dir() {
            collect_dir(root, workdir, budget, &mut used, &mut files);
        } else if root.is_file() {
            if let Ok((rel, content)) = read_capped(root, workdir, budget.saturating_sub(used)) {
                let toks = estimate_tokens(&content);
                used += toks;
                files.push(ContextFile { path: rel, content, token_estimate: toks });
            }
        }
        if used >= budget { break; }
    }

    files
}

fn collect_dir(
    dir: &Path,
    workdir: &Path,
    budget: usize,
    used: &mut usize,
    out: &mut Vec<ContextFile>,
) {
    // Skip known non-text directories
    let skip = ["target", "node_modules", ".git", ".roko", "dist", ".next"];
    if dir.file_name().map_or(false, |n| skip.contains(&n.to_str().unwrap_or(""))) {
        return;
    }

    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut paths: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort(); // deterministic order

    for path in paths {
        if *used >= budget { break; }
        if path.is_dir() {
            collect_dir(&path, workdir, budget, used, out);
        } else if is_text_file(&path) {
            if let Ok((rel, content)) = read_capped(&path, workdir, budget.saturating_sub(*used)) {
                let toks = estimate_tokens(&content);
                *used += toks;
                out.push(ContextFile { path: rel, content, token_estimate: toks });
            }
        }
    }
}

fn is_text_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md" | "rs" | "toml" | "ts" | "tsx" | "js" | "jsx" | "json" | "yaml"
             | "yml" | "txt" | "py" | "go" | "sh" | "env" | "lock")
    )
}
```

### Token Budgeting

The `estimate_tokens` function already exists in `roko-compose/src/prompt.rs`. Budget should be:
- Default: 50,000 tokens (leaves room for prompt + output on a 200K context model)
- Configurable via `--context-budget`
- Files are included in priority order; once budget is exhausted, remaining files are summarized (one-sentence each) rather than dropped entirely

### Priority Ranking

When multiple files compete for the budget, rank by:
1. Files whose name appears in the user's prompt text (keyword match)
2. Files modified most recently (`metadata().modified()`)
3. Files closer to the top of the directory tree (shorter path)
4. Everything else alphabetical

### Context Format in the Prompt

Each file gets wrapped in an XML tag, consistent with how `resolve_file_uri` already does it:

```
<file path="tmp/solutions/09-unified-cli-ux.md">
... content ...
</file>

<file path="crates/roko-acp/src/session.rs">
... content ...
</file>
```

Prepended with a short header: `The following files were provided as context via --context:`

### Deduplication

If `--context src/lib.rs` AND `@src/lib.rs` appear in the same invocation, include once. Key by canonical path.

---

## Design for Multi-Turn Context Accumulation in ACP Sessions

### The Current Gap

Context from file attachments in turn N does not survive to turn N+1. The `conversation_history` stores only the text content of each turn, not the resolved file blobs. This means:

- Turn 1: user attaches `@src/lib.rs`, asks a question → file injected into system prompt
- Turn 2: user asks a follow-up → file gone, model has no memory of its content (unless the assistant's reply in turn 1 quoted it)

### Proposed: Pinned Context

Add a `pinned_context: Vec<PinnedFile>` field to `AcpSession`:

```rust
pub struct PinnedFile {
    pub uri: String,
    pub rel_path: PathBuf,
    pub content: String,        // already resolved + truncated
    pub pinned_at: DateTime<Utc>,
}

pub struct AcpSession {
    // ... existing fields ...
    pub pinned_context: Vec<PinnedFile>,
}
```

When a file is resolved in `resolve_context_items`, the session can optionally pin it. The session exposes two operations:
- `pin_file(uri, content)` — adds to pinned list
- `unpin_file(uri)` — removes

On each `build_messages_array` call, pinned files are always injected into the system prompt regardless of whether the current turn has attachments. The injection budget is shared with the per-turn context, with pinned files getting priority.

### Pinning Protocol in ACP

New config option surfaced in the session panel: `context_persistence: "pinned" | "per_turn"`.

When set to `"pinned"`, every file attachment is auto-pinned. When `"per_turn"` (current default), files only apply to the current message.

Alternatively, a `/pin @file` slash command would be more explicit:
- `/pin @src/lib.rs` — pin this file for the rest of the session
- `/unpin @src/lib.rs` — remove it
- `/pinned` — list what is pinned

### Context Accumulation for `/prd` and `/task` Workflows

A richer form of accumulation is needed for the PRD/task synthesis workflow. This requires a `ContextSession` object (separate from the ACP session's conversation history):

```rust
// .roko/context-sessions/<id>.json
pub struct ContextSession {
    pub id: String,
    pub title: Option<String>,
    pub raw_notes: Vec<NoteEntry>,      // freeform text added via /note or roko note
    pub file_refs: Vec<PathBuf>,        // file/folder paths added via /context
    pub derived_clusters: Vec<Cluster>, // agent-suggested groupings (awaiting confirmation)
    pub prds: Vec<String>,              // generated PRD slugs
    pub task_plans: Vec<String>,        // generated task plan slugs
    pub updated_at: DateTime<Utc>,
}

pub struct NoteEntry {
    pub text: String,
    pub tags: Vec<String>,
    pub added_at: DateTime<Utc>,
    pub source_turn: Option<usize>,
}
```

Commands:
- `roko note "text"` — appends to the active context session
- `roko context add <path>` — adds file/folder reference
- `roko prd --from-session` — runs the synthesis pipeline on accumulated material
- `/note`, `/context`, `/prd` in ACP — same operations

---

## ACP (Zed) Integration — Full Picture

### What Already Works

1. `roko acp` starts a stdio ACP server that Zed connects to
2. `session/prompt` receives all Zed content blocks including file attachments
3. Resource blocks (`ContentBlock::Resource`) resolve file content into the system prompt
4. ResourceLink blocks (`ContentBlock::ResourceLink`) now also resolve (fix applied this session)
5. Text @-mentions resolve when `include_context: true` is passed
6. Conversation history is maintained in `AcpSession` (40 turns max, 64K chars max)
7. Knowledge store and playbooks are automatically queried on every non-slash prompt

### How to Configure Zed

In Zed settings (`~/.config/zed/settings.json`):
```json
{
  "agent": {
    "profiles": {
      "roko": {
        "provider": "acp",
        "binary": "roko",
        "args": ["acp", "--workdir", "."],
        "name": "Roko"
      }
    }
  }
}
```

### Slash Commands in ACP

Slash commands (`/plan-generate`, `/prd-idea`, etc.) are handled specially:
- `should_resolve_context` is set to `false` for slash commands (line 1000)
- File context and knowledge lookups are skipped
- The command text is passed directly to the slash command handler
- No conversation history is pushed for slash command turns

This is intentional: slash commands are control-plane operations, not agent turns.

### What's Missing for Full "Context" Workflow in Zed

1. **`include_context: true` is not sent by default.** Zed would need to be configured to always pass this, or roko's ACP server should default to treating all text @-mentions as context even when `include_context` is false. Current behavior: text @-mentions only resolve when `include_context: true`.

2. **No `/pin` command.** Files don't persist across turns.

3. **Slash commands don't use file attachments.** If you type `/plan-generate implement X` while `@spec.md` is attached, the spec file is not passed to plan generation.

4. **No `/context` slash command.** No way to add a folder to the session from the Zed panel.

5. **No `/models` command.** No discoverability of available models from the editor.

### Proposed New Slash Commands

```
/note <text>               — append note to active context session (alias /prd-idea)
/context <path>            — add file or folder to session context
/plan <text>               — generate plan using current session context + @-mentioned files
/do <prompt or slug>       — generate plan if needed, then execute it
/models                    — list configured models
/pin                       — list/manage pinned files
```

### Making Slash Commands Use File Context

In `handle_slash_command` (bridge_events.rs), the fix is to pass the resolved context from the current prompt blocks into the command handler:

```rust
// When a slash command is invoked, check if there are file attachments
// and pass them as context to the command.
let slash_context = if !params.prompt.is_empty() {
    resolve_context_items(&params.prompt, workdir).await
} else {
    String::new()
};
// Pass slash_context into the plan-generate / prd-idea / etc. handlers
```

---

## File Changes Needed

| File | Change |
|------|--------|
| `crates/roko-cli/src/commands/plan.rs` | Add `--context` flag; call `load_context_files` before plan generation |
| `crates/roko-cli/src/commands/do_cmd.rs` | Same `--context` flag |
| `crates/roko-cli/src/plan_generate.rs` | Accept `Vec<ContextFile>` and inject into generation prompt |
| `crates/roko-acp/src/session.rs` | Add `pinned_context: Vec<PinnedFile>` field; `pin_file`/`unpin_file` methods |
| `crates/roko-acp/src/bridge_events.rs` | Pass @-file context to slash command handlers; inject pinned context into every turn |
| `crates/roko-acp/src/types.rs` | No changes needed |

## Implementation Priority

### Phase 1: CLI `--context` flag
1. Add `walkdir` crate (or use recursive `std::fs::read_dir`)
2. Add `--context` to `roko plan` subcommand in `commands/plan.rs`
3. Implement `load_context_files` with dir walking, token budgeting, and priority ranking
4. Inject context XML into the plan generation prompt

### Phase 2: ACP/Zed slash commands
5. Pass resolved file context into slash command handlers
6. Add `/note`, `/context`, `/do`, `/models` slash commands

### Phase 3: Multi-turn persistence
7. Add `pinned_context` to `AcpSession`
8. Add `/pin` slash command
9. Implement `ContextSession` store for PRD synthesis workflow
