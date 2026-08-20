# 66 — Context Sources and Editor Integration

**Priority**: P3 — `--context` flag exists on `roko do` but is missing from `roko run`, `roko develop`, and `roko chat`; URL sources and editor context push are not supported
**Size**: L (3 phases, each independently shippable)
**Crates**: `crates/roko-cli` (`src/main.rs`, `src/commands/do_cmd.rs`, `src/commands/develop.rs`, `src/context_loader.rs`), `crates/roko-neuro` (`src/context.rs`), `crates/roko-acp` (`src/types.rs`, `src/bridge_events.rs`)
**Depends on**: None

---

## Background

Roko has a working context-injection mechanism: `context_loader.rs` loads files, globs, and directories up to a 50K character budget and formats them as `<file path="...">content</file>` XML blocks. The `roko do` command exposes a `--context <PATH>` flag that feeds these blocks into the prompt. However, the same `--context` flag is missing from `roko run`, `roko develop`, and `roko agent chat`. Users who learn the flag on `do` expect it everywhere, and are surprised when `roko run "Fix bug" --context src/lib.rs` fails with "unexpected argument."

There is also no support for URL context sources. A user cannot run `roko do "Fix issue" --context https://github.com/org/repo/issues/42` to include the issue text in the prompt. The `context_loader.rs` function handles file paths and globs but has no URL detection or fetch logic.

The ACP server (`roko-acp`) implements JSON-RPC 2.0 over stdio for Cursor/Zed/JetBrains/Neovim. Its `SessionPromptParams` struct (in `types.rs` line 484) has `session_id`, `prompt`, and `include_context`. It has no mechanism for the editor to push structural context (open files, cursor position, diagnostics) alongside a prompt. This means editors cannot tell roko what file is open or where the cursor is.

The three gaps are independent and can be shipped separately. Phase A (uniform `--context` + URL sources) is highest priority because it involves only the CLI. Phase B (ACP context push protocol) requires a protocol extension. Phase C (VS Code extension) is a separate TypeScript project.

## Current State

### Phase A — Uniform `--context` and URL sources

1. **`--context` on `roko do`** — `crates/roko-cli/src/main.rs` lines 384–386:
   ```rust
   #[arg(long = "context", value_name = "PATH")]
   context: Vec<PathBuf>,
   ```
   Dispatched to `cmd_do` at line 2681, threaded to `do_cmd.rs`.

2. **`cmd_do` context loading** — `crates/roko-cli/src/commands/do_cmd.rs` lines 215–234. When `context` is non-empty, calls `roko_cli::context_loader::load_context_files(context, DEFAULT_BUDGET, workdir)` and wraps the result in `<context>...</context>`. Appends to the task prompt string.

3. **`--context` on `plan generate`** — `crates/roko-cli/src/main.rs` lines 1555–1557. Same `Vec<PathBuf>` pattern. Wired at `commands/plan.rs:1030`.

4. **`Command::Run`** — `crates/roko-cli/src/main.rs` line 2639. When dispatched without `--serve`/`--share`, calls `commands::do_cmd::cmd_do(...)` with a hard-coded `Vec::new()` for the context argument (line 2661). The `Run` variant itself has no `context` field in its struct.

5. **`Command::Develop`** — `crates/roko-cli/src/main.rs` line 2698. Dispatches to `commands::develop::cmd_develop()`. No `context` field in its struct. `develop.rs` does not call `load_context_files`.

6. **`context_loader.rs`** — `crates/roko-cli/src/context_loader.rs`. `load_context_files(paths, budget, workdir)` iterates paths: detects glob patterns (contains `*/?/[`), resolves relative paths, recurses into directories skipping `target/`, `node_modules/`, `.git/`, `.roko/`, `dist/`, `__pycache__`. Skips binary file extensions (png, jpg, pdf, zip, etc.). Returns formatted XML blocks. No URL detection.

7. **`ContextSource` enum** — `crates/roko-neuro/src/context.rs` line 77. Variants: `KnowledgeEntry`, `Episode`, `InlineFile`, `RecentSignal`, `SymbolSignature`, `AntiPattern`, `Verification`, `TaskBrief`, `PriorTaskOutput`, `PlanBrief`, `ResearchMemo`, `Invariants`, `CrossPlanContext`, `PrdExtract`, `Decomposition`, `SiblingTasks`, `Pheromone`. No `Url` variant.

8. **`[context]` config section** — Does not exist in `crates/roko-core/src/config/schema.rs`. No `always_include` paths are supported.

### Phase B — ACP context push protocol

9. **`SessionPromptParams`** — `crates/roko-acp/src/types.rs` line 484:
   ```rust
   pub struct SessionPromptParams {
       pub session_id: String,
       pub prompt: Vec<ContentBlock>,
       #[serde(default)]
       pub include_context: bool,
   }
   ```
   No `context` field for editor-provided structured metadata.

10. **`handle_session_prompt`** / **`handle_session_prompt_inner`** — `crates/roko-acp/src/bridge_events.rs` lines 1646 / 1670. Reads `params.session_id` and `params.prompt`. Does not read any editor context beyond the prompt text.

## Implementation Plan

### Phase A1: Add `--context` to `Command::Run`

In `crates/roko-cli/src/main.rs`, update the `Run` variant of `Command` (line 2639) to add:

```rust
/// Additional context files/dirs/globs/URLs to include in the prompt.
#[arg(long = "context", value_name = "PATH")]
context: Vec<PathBuf>,
```

In the `Command::Run` match arm (line 2639–2665), change the `cmd_do` call to pass `context` instead of `Vec::new()`:

```rust
// Before (line 2661):
Vec::new(),
// After:
context,
```

The `context` field must also be destructured in the match arm pattern:
```rust
Command::Run { prompt, workdir, serve, share, provider, max_retries, context } => {
```

### Phase A2: Add `--context` to `Command::Develop`

In `crates/roko-cli/src/main.rs`, update the `Develop` variant (line 2698) to add:

```rust
#[arg(long = "context", value_name = "PATH")]
context: Vec<PathBuf>,
```

Update the `Command::Develop` match arm (line 2698–2707) to pass `context` to `cmd_develop`. Update `commands/develop.rs` to accept `context: Vec<PathBuf>` and, if non-empty, call `load_context_files` and prepend the result to the plan-generation prompt (same pattern as `cmd_do` lines 215–234).

### Phase A3: URL detection and fetching in `context_loader.rs`

Add URL detection to `load_context_files` in `crates/roko-cli/src/context_loader.rs`. A path is a URL if its string representation starts with `http://` or `https://`:

```rust
fn is_url(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.starts_with("http://") || s.starts_with("https://")
}
```

For URL paths, call `reqwest::Client::new().get(url).send().await` (or the sync variant if `load_context_files` stays sync), read the response body as text, and convert HTML to plain text. Since `reqwest` is already a dependency in `roko-cli`, use it. For HTML-to-text conversion, use the existing `html2text` crate if it is already in scope, or implement a minimal tag-stripping function.

Wrap the fetch in a fallback: if the fetch fails (network error, non-2xx status, non-text content-type), emit `eprintln!("warning: skipping URL {url}: {error}")` and skip that source. Apply the same 50K budget accounting as for files.

The output format is the same XML block: `<file path="{url}">fetched content</file>`.

Since `load_context_files` is currently synchronous, this either requires making it `async` (preferred, since the callers in `do_cmd.rs` are already in async context) or calling `tokio::task::block_in_place` around the fetch.

### Phase A4: Add `ContextSource::Url` variant

In `crates/roko-neuro/src/context.rs`, add a `Url` variant to `ContextSource` (after line 151):

```rust
/// Content fetched from a URL.
Url {
    /// The fetched URL.
    url: String,
    /// Final URL after redirects.
    resolved_url: Option<String>,
},
```

Update the `source_priority`, `source_family`, and `dedup_similar_chunks` match arms in `context.rs` to handle the new variant (give it the same priority as `InlineFile`).

### Phase A5: Add `[context]` config section for default includes

In `crates/roko-core/src/config/schema.rs`, add a `ContextConfig` struct and a `context` field to `RokoConfig`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ContextConfig {
    /// Paths always included in every prompt (relative to workdir).
    #[serde(default)]
    pub always_include: Vec<String>,
    /// Character budget for context loading (default: 50_000).
    #[serde(default = "default_context_budget")]
    pub default_budget: usize,
}

fn default_context_budget() -> usize { 50_000 }
```

Add `pub context: ContextConfig` to `RokoConfig`. In `cmd_do` (and `cmd_develop` after Phase A2), load `config.context.always_include` paths and prepend them to the `context` vec before calling `load_context_files`. This does not change the existing `--context` behavior.

### Phase B1: Add `EditorContext` to `SessionPromptParams`

In `crates/roko-acp/src/types.rs`, add:

```rust
/// Structured editor context pushed by the IDE alongside a prompt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditorContext {
    /// Path of the file open in the editor.
    #[serde(default)]
    pub active_file: Option<String>,
    /// Line number of the cursor (1-based).
    #[serde(default)]
    pub cursor_line: Option<u32>,
    /// Selected text range.
    #[serde(default)]
    pub selection: Option<EditorSelection>,
    /// All files currently open in the editor.
    #[serde(default)]
    pub open_files: Vec<String>,
    /// Diagnostic messages from the editor's language server.
    #[serde(default)]
    pub diagnostics: Vec<EditorDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorSelection {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorDiagnostic {
    pub file: String,
    pub line: u32,
    pub message: String,
    pub severity: String, // "error", "warning", "info", "hint"
}
```

Extend `SessionPromptParams`:

```rust
pub struct SessionPromptParams {
    pub session_id: String,
    pub prompt: Vec<ContentBlock>,
    #[serde(default)]
    pub include_context: bool,
    /// Optional editor context from the IDE.
    #[serde(default)]
    pub context: Option<EditorContext>,
}
```

The `#[serde(default)]` ensures existing clients that do not send `context` remain compatible.

### Phase B2: Thread `EditorContext` into prompt assembly in `bridge_events.rs`

In `handle_session_prompt_inner` in `crates/roko-acp/src/bridge_events.rs` (line 1670), after reading `params.prompt`, check `params.context`. If present, serialize the editor context into a human-readable text block and prepend it to the prompt as a `ContentBlock::Text`:

```rust
if let Some(editor_ctx) = &params.context {
    let mut ctx_text = String::from("[Editor Context]\n");
    if let Some(file) = &editor_ctx.active_file {
        ctx_text.push_str(&format!("Active file: {file}\n"));
        // Try to read the file content (bounded to 10K chars)
        if let Ok(content) = std::fs::read_to_string(file) {
            let truncated = &content[..content.len().min(10_000)];
            ctx_text.push_str(&format!("Content:\n{truncated}\n"));
        }
    }
    if let Some(line) = editor_ctx.cursor_line {
        ctx_text.push_str(&format!("Cursor line: {line}\n"));
    }
    for diag in &editor_ctx.diagnostics {
        ctx_text.push_str(&format!("[{}] {}:{} {}\n",
            diag.severity, diag.file, diag.line, diag.message));
    }
    // Insert as first content block
    let mut augmented_prompt = vec![ContentBlock::Text { text: ctx_text }];
    augmented_prompt.extend(params.prompt.iter().cloned());
    params.prompt = augmented_prompt;
}
```

This ensures editor context appears before the user's prompt in the assembled turn.

## Acceptance Criteria

### Phase A

1. `roko run "Fix bug" --context src/lib.rs` loads `src/lib.rs` into the prompt. Currently `roko run` has no `--context` flag and rejects this invocation.
2. `roko do "Add tests" --context https://github.com/org/repo/issues/42` fetches the URL, converts to text, and includes it in the `<context>` block.
3. `roko develop "Add auth" --context docs/AUTH.md` threads context through plan generation.
4. `roko.toml` with `[context] always_include = ["CLAUDE.md"]` causes `CLAUDE.md` content to appear in every `roko do` prompt even without `--context`.
5. URL fetch failures emit a warning line and skip the URL — they do not abort the command.
6. Binary/non-text URL content-types are skipped with a warning.
7. The 50K character budget applies across files and URLs combined.

### Phase B

1. `session/prompt` with an `EditorContext` payload containing `activeFile` produces a prompt that includes the file content and diagnostics block.
2. The ACP server gracefully ignores a missing `context` field (old clients work unchanged).
3. `cargo test -p roko-acp` passes with all existing 180 ACP tests still passing.

## Verification Checklist

- [ ] Add `--context` to `Command::Run` in `main.rs`; run `roko run "hello" --context README.md`; confirm content in prompt
- [ ] Run `roko do "hello" --context https://example.com`; confirm URL content appears in `<context>` block
- [ ] Confirm URL fetch failure prints warning and command continues
- [ ] Add `[context] always_include = ["CLAUDE.md"]` to `roko.toml`; run `roko do "hello"` without `--context`; confirm CLAUDE.md appears
- [ ] Send ACP `session/prompt` with `context: { activeFile: "..." }`; confirm file content in assembled prompt
- [ ] Send ACP `session/prompt` without `context` field; confirm backward compatibility
- [ ] `cargo test --workspace` passes

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/main.rs` | Add `context: Vec<PathBuf>` to `Command::Run` and `Command::Develop`; thread context into `cmd_do` and `cmd_develop` dispatch |
| `crates/roko-cli/src/commands/develop.rs` | Add `context: Vec<PathBuf>` parameter; call `load_context_files` and prepend to plan-generation prompt |
| `crates/roko-cli/src/context_loader.rs` | Add `is_url()` detection; add async URL fetch path; convert HTML to text; apply budget accounting |
| `crates/roko-neuro/src/context.rs` | Add `ContextSource::Url { url, resolved_url }` variant; update all match arms |
| `crates/roko-core/src/config/schema.rs` | Add `ContextConfig` struct and `context: ContextConfig` field to `RokoConfig` |
| `crates/roko-acp/src/types.rs` | Add `EditorContext`, `EditorSelection`, `EditorDiagnostic` structs; add optional `context` field to `SessionPromptParams` |
| `crates/roko-acp/src/bridge_events.rs` | Read `params.context` in `handle_session_prompt_inner`; prepend editor context block to assembled prompt |
