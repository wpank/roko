# A — Core Abstractions (Docs 00, 03, 04)

Parity analysis of `docs/02-agents/00-agent-trait.md`, `03-chat-types.md`, `04-agent-roles.md` vs actual codebase.

---

## A.01 — Agent Trait (3 methods) (Doc 00)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
The `Agent` trait lives at `crates/roko-agent/src/agent.rs` with 3 methods:
1. `async fn run(&self, input: &Signal, ctx: &Context) -> AgentResult`
2. `fn name(&self) -> &str`
3. `fn supports_streaming(&self) -> bool` (default `false`)

Requires `Send + Sync`. Uses `&Signal` input and `&Context` context.

### What exists
All 3 methods present at `crates/roko-agent/src/agent.rs:119-137`. Signatures match with one terminology delta:

| Aspect | Doc | Code | Match |
|--------|-----|------|-------|
| Trait bounds | `Send + Sync` | `Send + Sync` | MATCH |
| `run` input type | `&Signal` | `&Engram` | NAME ONLY (Engram = Signal) |
| `run` context type | `&Context` | `&Context` | MATCH |
| `run` return type | `AgentResult` | `AgentResult` | MATCH |
| `name` signature | `fn name(&self) -> &str` | `fn name(&self) -> &str` | MATCH |
| `supports_streaming` | default `false` | default `false` | MATCH |
| `#[async_trait]` | Yes | Yes (line 119) | MATCH |

Doc comment at lines 96-109 matches the documented rationale (4 reasons agents don't fit core traits).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.01.1 | Doc says `Signal`, code says `Engram` (cosmetic name mismatch documented in doc 01 glossary) | doc 00 line 81 vs agent.rs:128 | LOW |

### Verify
```bash
grep -n 'async fn run\|fn name\|fn supports_streaming' crates/roko-agent/src/agent.rs
```

---

## A.02 — AgentResult Struct (Doc 00)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`AgentResult` at `crates/roko-agent/src/agent.rs` with 4 fields:
- `output: Signal`
- `trace: Vec<Signal>`
- `usage: Usage`
- `success: bool`

Plus constructors `ok()`, `fail()`, `with_trace()`, `with_usage()`, `all_signals()`.

### What exists
All 4 fields present at `crates/roko-agent/src/agent.rs:9-23`:

| Field | Doc | Code (line) | Match |
|-------|-----|-------------|-------|
| `output` | `Signal` | `Engram` (line 12) | NAME ONLY |
| `trace` | `Vec<Signal>` | `Vec<Engram>` (line 16) | NAME ONLY |
| `usage` | `Usage` | `Usage` (line 19) | MATCH |
| `success` | `bool` | `bool` (line 22) | MATCH |

All 5 methods present:
- `ok()` at line 28 (`const fn`, `#[must_use]`)
- `fail()` at line 39 (`const fn`, `#[must_use]`)
- `with_trace()` at line 50 (`#[must_use]`)
- `with_usage()` at line 57 (`const fn`, `#[must_use]`)
- `all_signals()` at line 64 (`#[must_use]`)

Additionally, the code has 2 helper functions not in the doc:
- `derived_output()` at line 76 -- builds lineage-preserving output signals
- `full_lineage()` at line 88 -- returns full upstream lineage iterator

### Gaps
None. Implementation matches spec (plus extras).

### Verify
```bash
grep -n 'pub.*fn ok\|pub.*fn fail\|pub.*fn with_trace\|pub.*fn with_usage\|pub.*fn all_signals' crates/roko-agent/src/agent.rs
```

---

## A.03 — Usage Tracking (Doc 00)

- **Status**: PARTIAL
- **Priority**: P2
- **Estimated LOC**: 5
- **Dependencies**: None
- **Files to modify**: `crates/roko-agent/src/usage.rs`

### What the doc says
`Usage` struct (from `crates/roko-agent/src/usage.rs`) with 7 fields:
1. `input_tokens`
2. `output_tokens`
3. `cache_read_tokens`
4. `cache_write_tokens`
5. `cost_usd`
6. `duration_ms`
7. `model` -- which model was used

### What exists
6 of 7 fields present at `crates/roko-agent/src/usage.rs:11-24`:

| Field | Doc Name | Code Name | Type | Match |
|-------|----------|-----------|------|-------|
| 1 | `input_tokens` | `input_tokens` | `u32` | MATCH |
| 2 | `output_tokens` | `output_tokens` | `u32` | MATCH |
| 3 | `cache_read_tokens` | `cache_read_tokens` | `u32` | MATCH |
| 4 | `cache_write_tokens` | `cache_create_tokens` | `u32` | RENAMED |
| 5 | `cost_usd` | `cost_usd` | `f32` | MATCH |
| 6 | `duration_ms` | `wall_ms` | `u64` | RENAMED |
| 7 | `model` | -- | -- | MISSING |

Helper methods: `zero()` at line 29, `total_tokens()` at line 42, `add()` at line 47.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.03.1 | `model` field missing from `Usage` (doc says it carries model for cost attribution) | usage.rs | LOW |
| A.03.2 | Field `cache_write_tokens` is `cache_create_tokens` in code | usage.rs:19 | LOW (cosmetic) |
| A.03.3 | Field `duration_ms` is `wall_ms` in code | usage.rs:23 | LOW (cosmetic) |

### Verify
```bash
grep -n 'pub ' crates/roko-agent/src/usage.rs | head -15
```

---

## A.04 — Concrete Agent Implementations (Doc 00)

- **Status**: DONE (exceeds spec)
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
6 agent implementations + CursorAgent = 7 total:

| Name | Module | Backend |
|------|--------|---------|
| `ClaudeCliAgent` | `claude_cli_agent.rs` | Claude CLI subprocess |
| `ClaudeAgent` | `claude_agent.rs` | Anthropic Messages API |
| `OpenAiAgent` | `openai_agent.rs` | OpenAI Chat Completions |
| `OllamaAgent` | `ollama_agent.rs` | Ollama HTTP |
| `ExecAgent` | `exec.rs` | Any CLI binary |
| `MockAgent` | `mock.rs` | In-memory test double |
| `CursorAgent` | `cursor_agent.rs` | Cursor ACP |

### What exists
19 types implement `Agent` (searched via `impl Agent for`):

| # | Implementation | File | In Doc? |
|---|---------------|------|---------|
| 1 | `ClaudeCliAgent` | `claude_cli_agent.rs:399` | YES |
| 2 | `ClaudeAgent` | `claude_agent.rs:342` | YES |
| 3 | `OpenAiAgent` | `openai_agent.rs:138` | YES |
| 4 | `OllamaAgent` | `ollama_agent.rs:198` | YES |
| 5 | `ExecAgent` | `exec.rs:123` | YES |
| 6 | `MockAgent` | `mock.rs:57` | YES |
| 7 | `CursorAgent` | `cursor_agent.rs:267` | YES |
| 8 | `CodexAgent` | `codex_agent.rs:298` | NO |
| 9 | `CompositeAgent` | `composition.rs:451` | NO |
| 10 | `MorphableAgent` | `metamorphosis.rs:162` | NO |
| 11 | `ToolLoopAgent` | `tool_loop/agent_wrapper.rs:90` | NO |
| 12 | `PerplexityChatAgent` | `perplexity/chat.rs:115` | NO |
| 13 | `PerplexityDeepResearchAgent` | `perplexity/deep_research.rs:229` | NO |
| 14 | `PerplexityEmbedAgent` | `perplexity/adapter.rs:54` | NO |
| 15 | `PerplexityToolLoopAgent` | `perplexity/tool_loop.rs:228` | NO |
| 16 | `GeminiCompatAgent` | `gemini/compat.rs:72` | NO |
| 17 | `GeminiNativeAgent` | `gemini/native.rs:366` | NO |
| 18 | `GeminiEmbedAgent` | `gemini/embed.rs:198` | NO |
| 19 | `SequenceAgent` | `task_runner.rs:638` (test-only) | NO |

12 implementations beyond the 7 documented. Code exceeds spec significantly.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.04.1 | Doc lists 6+1=7 implementations; code has 19 (12 undocumented) | doc 00 line 178 | LOW (doc outdated, code is fine) |

### Verify
```bash
grep -rn 'impl Agent for' crates/roko-agent/src/ | wc -l
```

---

## A.05 — ChatResponse Canonical Type (Doc 03)

- **Status**: DONE (with duplication caveat)
- **Priority**: P2
- **Estimated LOC**: 30
- **Dependencies**: None
- **Files to modify**: `crates/roko-agent/src/chat_types.rs`, `crates/roko-agent/src/translate/mod.rs`

### What the doc says
`ChatResponse` defined at `crates/roko-agent/src/translate/mod.rs:55` with 6 fields:
- `content: String`
- `reasoning: Option<String>`
- `tool_calls: Vec<ToolCall>`
- `usage: Usage`
- `finish_reason: FinishReason`
- `metadata: ResponseMetadata`

### What exists
**Two** `ChatResponse` structs exist in the codebase:

**Copy 1** -- `crates/roko-agent/src/translate/mod.rs:58-66`:
```rust
pub struct ChatResponse {
    pub content: String,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
    pub finish_reason: crate::chat_types::FinishReason,
    pub metadata: ResponseMetadata,
}
```
6 fields. Matches doc exactly.

**Copy 2** -- `crates/roko-agent/src/chat_types.rs:91-101`:
```rust
pub struct ChatResponse {
    pub content: String,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
    pub finish_reason: FinishReason,
    pub metadata: ResponseMetadata,
    pub raw_assistant_message: Option<ChatMessage>,
    pub session: SessionState,
}
```
8 fields. Has 2 extra fields vs doc: `raw_assistant_message` and `session`.
Also has helper methods: `as_assistant_message()` at line 135 and `to_signal()` at line 166.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.05.1 | Two competing `ChatResponse` structs (translate/mod.rs vs chat_types.rs) | translate/mod.rs:58, chat_types.rs:91 | MEDIUM |
| A.05.2 | `chat_types.rs` version has extra `raw_assistant_message` and `session` fields not in doc | chat_types.rs:99-100 | LOW (undocumented enhancement) |
| A.05.3 | Doc says location is `translate/mod.rs:55`; canonical version appears to be `chat_types.rs` based on lib.rs re-exports | lib.rs:63 | LOW |

### Verify
```bash
grep -rn 'pub struct ChatResponse' crates/roko-agent/src/
```

---

## A.06 — FinishReason Enum (Doc 03)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
5-variant enum at `crates/roko-agent/src/translate/mod.rs`:
- `Stop` (default)
- `Length`
- `ToolCalls`
- `ContentFilter`
- `Error(String)`

Plus `normalize_finish_reason()` function mapping raw strings to canonical variants.

### What exists
**Two** copies (same pattern as ChatResponse):

**Copy 1** -- `crates/roko-agent/src/translate/mod.rs` (re-exported from chat_types): uses `pub use crate::chat_types::FinishReason` at line 37.

**Copy 2** -- `crates/roko-agent/src/chat_types.rs:122-130`:
```rust
pub enum FinishReason {
    #[default]
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    Error(String),
}
```
All 5 variants match. `#[default]` on `Stop` matches. Derives: `Debug, Clone, PartialEq, Eq, Default`.

`normalize_finish_reason()` at `translate/mod.rs:82-91` matches doc exactly:

| Raw string | Canonical | Match |
|-----------|-----------|-------|
| `"stop"` / `"end_turn"` | `Stop` | MATCH |
| `"length"` / `"max_tokens"` | `Length` | MATCH |
| `"tool_calls"` / `"tool_use"` | `ToolCalls` | MATCH |
| `"content_filter"` / `"sensitive"` | `ContentFilter` | MATCH |
| `"network_error"` | `Error("network_error")` | MATCH |
| `"model_context_window_exceeded"` | `Error("context_overflow")` | MATCH |
| anything else | `Error(other)` | MATCH |

### Gaps
None. Exact match including normalization table.

### Verify
```bash
grep -n 'pub enum FinishReason' crates/roko-agent/src/chat_types.rs
```

---

## A.07 — ResponseMetadata Struct (Doc 03)

- **Status**: PARTIAL
- **Priority**: P3
- **Estimated LOC**: 0
- **Dependencies**: A.05
- **Files to modify**: None (cosmetic)

### What the doc says
`ResponseMetadata` with 7 fields:
1. `response_id: Option<String>`
2. `model_used: Option<String>`
3. `cached_tokens: Option<u64>`
4. `content_filter: Option<serde_json::Value>`
5. `web_search: Option<serde_json::Value>`
6. `provider_latency_ms: Option<u64>`
7. `raw_finish_reason: Option<String>`

### What exists
**Two** copies (same duplication as ChatResponse/FinishReason):

**Copy 1** -- `crates/roko-agent/src/translate/mod.rs:68-78`:
```rust
pub struct ResponseMetadata {
    pub response_id: Option<String>,
    pub model_used: Option<String>,
    pub cached_tokens: Option<u64>,
    pub content_filter: Option<serde_json::Value>,
    pub web_search: Option<serde_json::Value>,
    pub extra: Option<serde_json::Value>,         // NOT IN DOC
    pub provider_latency_ms: Option<u64>,
    pub raw_finish_reason: Option<String>,
}
```
8 fields (1 extra: `extra`).

**Copy 2** -- `crates/roko-agent/src/chat_types.rs:111-120`:
```rust
pub struct ResponseMetadata {
    pub response_id: Option<String>,
    pub model_used: Option<String>,
    pub cached_tokens: Option<u64>,
    pub content_filter: Option<serde_json::Value>,
    pub web_search: Option<serde_json::Value>,
    pub provider_latency_ms: Option<u64>,
    pub raw_finish_reason: Option<String>,
}
```
7 fields. Matches doc exactly (no `extra` field).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.07.1 | translate/mod.rs copy has extra `extra: Option<serde_json::Value>` field not in doc or chat_types.rs | translate/mod.rs:75 | LOW |
| A.07.2 | Two copies of ResponseMetadata with divergent field counts | translate/mod.rs:68 vs chat_types.rs:111 | MEDIUM (same root cause as A.05.1) |

### Verify
```bash
grep -rn 'pub struct ResponseMetadata' crates/roko-agent/src/
```

---

## A.08 — ChatResponse Crate Location (Doc 03)

- **Status**: NOT DONE
- **Priority**: P1
- **Estimated LOC**: 80
- **Dependencies**: None
- **Files to modify**: `crates/roko-core/src/`, `crates/roko-agent/src/chat_types.rs`, `crates/roko-agent/src/translate/mod.rs`, `crates/roko-compose/src/`

### What the doc says
`ChatResponse`, `FinishReason`, `ResponseMetadata`, and `normalize_finish_reason` **must eventually live in roko-core** so both `roko-compose` and `roko-agent` can depend on them. Currently in `roko-agent::translate` which creates a circular dependency problem: `roko-compose` cannot depend on `roko-agent`.

The workaround: `roko-compose` uses raw `Signal` metadata tags and JSON values instead of typed `ChatResponse` structs.

### What exists
Types still live in `roko-agent`:
- `ChatResponse` in `crates/roko-agent/src/chat_types.rs:91` and `crates/roko-agent/src/translate/mod.rs:58`
- `FinishReason` in `crates/roko-agent/src/chat_types.rs:122` (re-exported by translate/mod.rs)
- `ResponseMetadata` in both locations
- `normalize_finish_reason` in `crates/roko-agent/src/translate/mod.rs:82`

No types have been migrated to `roko-core`. The `roko-compose` crate still cannot import `ChatResponse`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.08.1 | `ChatResponse`, `FinishReason`, `ResponseMetadata` not in roko-core | roko-core/src/ | HIGH |
| A.08.2 | `roko-compose` workaround (raw Signal metadata) still in effect | roko-compose/src/ | MEDIUM |
| A.08.3 | Two competing copies of chat types within roko-agent itself (translate/mod.rs vs chat_types.rs) need reconciliation before or during migration | roko-agent/src/ | MEDIUM |

### Verify
```bash
grep -rn 'ChatResponse\|FinishReason\|ResponseMetadata' crates/roko-core/src/ --include='*.rs' | head -5
# Expected: no results (types not migrated yet)
```

---

## A.09 — BackendResponse Enum (Doc 03)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
3-variant enum below `ChatResponse`:
- `Json(serde_json::Value)` -- single JSON object
- `StreamJson(Vec<serde_json::Value>)` -- stream-json events
- `Text(String)` -- plain text

With methods: `extract_text()`, `extract_reasoning()`.

### What exists
`BackendResponse` at `crates/roko-agent/src/translate/mod.rs:159-168`:

| Variant | Doc | Code | Match |
|---------|-----|------|-------|
| `Json(serde_json::Value)` | Yes | line 163 | MATCH |
| `StreamJson(Vec<serde_json::Value>)` | Yes | line 165 | MATCH |
| `Text(String)` | Yes | line 167 | MATCH |

Methods:
- `extract_text()` at line 177 -- matches doc, enhanced with Gemini support (`extract_gemini_text`)
- `extract_reasoning()` at line 208 -- matches doc, handles 4+ wire formats
- `extract_usage()` at line 230 -- NOT in doc (additional utility)

`extract_text()` handles the 3 JSON shapes documented plus Gemini's `candidates/0/content/parts` format.

`extract_reasoning()` handles all 4 wire formats documented in the spec:
1. OpenAI-style `reasoning_content` (line 240)
2. Anthropic-style `content` blocks with `type: "thinking"` (line 265)
3. Stream-JSON `delta.reasoning_content` (line 287)
4. Stream-JSON `delta.thinking` (line 294)
Plus additional: `content_block.reasoning_content` (line 300) and `content_block.type == "thinking"` (line 307).

### Gaps
None. Implementation exceeds spec.

### Verify
```bash
grep -n 'pub enum BackendResponse' crates/roko-agent/src/translate/mod.rs
```

---

## A.10 — AgentRole Enum (Doc 04)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
28 roles defined by `AgentRole` enum in `crates/roko-core/src/agent.rs`, organized into 7 groups:
- Planning (3): Architect, Planner, Researcher
- Implementation (4): Implementer, Debugger, Optimizer, Migrator
- Review (2): Reviewer, Auditor
- Validation (3): Tester, Validator, GateKeeper
- Orchestration (3): Conductor, Coordinator, Monitor
- Specialized (4): DocWriter, Translator, Analyst, Explorer
- Operations (2): Deployer, Operator
Plus "additional roles complete the taxonomy to 28 total."

### What exists
28 variants at `crates/roko-core/src/agent.rs:582-639`. Confirmed by test at line 1057: `assert_eq!(all_with_conductor.len(), 28)`. `ALL_AGENTS` constant at line 644 contains 27 (excludes Conductor).

However, the actual role names diverge significantly from the doc:

| Doc Role | Code Role | Match |
|----------|-----------|-------|
| Architect | `Architect` | MATCH |
| Planner | `Strategist` | RENAMED |
| Researcher | `Researcher` | MATCH |
| Implementer | `Implementer` | MATCH |
| Debugger | `ErrorDiagnoser` | RENAMED |
| Optimizer | -- | MISSING (closest: `Refactorer`) |
| Migrator | -- | MISSING |
| Reviewer | `QuickReviewer` | RENAMED |
| Auditor | `Auditor` | MATCH |
| Tester | `IntegrationTester` | RENAMED |
| Validator | `TerminalValidator` | RENAMED |
| GateKeeper | -- | MISSING |
| Conductor | `Conductor` | MATCH |
| Coordinator | `PlanLifecycleManager` | RENAMED |
| Monitor | `PerformanceSentinel` | RENAMED |
| DocWriter | `Scribe` | RENAMED |
| Translator | -- | MISSING |
| Analyst | -- | MISSING |
| Explorer | -- | MISSING |
| Deployer | -- | MISSING |
| Operator | -- | MISSING |

Code roles NOT in doc:
`Critic`, `AutoFixer`, `Refactorer`, `PrePlanner`, `DocVerifier`, `MergeResolver`, `GolemLifecycleTester`, `SpecDriftDetector`, `RegressionDetector`, `CoverageTracker`, `CrossSystemTester`, `DependencyValidator`, `PatternExtractor`, `SnapshotComparator`, `FullLoopValidator`

The doc's role names are essentially a pre-implementation sketch. The actual enum reflects the refined taxonomy that emerged during development. The count (28) is correct; the names diverged.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.10.1 | Doc role names (Planner, Debugger, Reviewer, etc.) do not match code (Strategist, ErrorDiagnoser, QuickReviewer, etc.) | doc 04 vs agent.rs:582 | LOW (doc outdated, code is authoritative) |
| A.10.2 | Doc lists generic roles (Optimizer, Migrator, GateKeeper, Translator, Analyst, Explorer, Deployer, Operator) that don't exist in code -- replaced by specialized variants | doc 04 vs agent.rs | LOW (doc outdated) |

### Verify
```bash
grep -c '^ *Self::' crates/roko-core/src/agent.rs | head -1
# Count enum variants in label() match
```

---

## A.11 — Per-Role Defaults (Tier, Budget, Permissions) (Doc 04)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: A.10
- **Files to modify**: None

### What the doc says
Each role carries 4 defaults via associated methods:
1. `fn backend(&self) -> AgentBackend`
2. `fn model_tier(&self) -> ModelTier` with 3 tiers: Fast, Standard, Premium
3. Per-turn `TurnBudget { base_usd, multiplier }`
4. `ToolPermissions { read, write, exec, git, network }`

### What exists
All 4 methods present on `AgentRole` at `crates/roko-core/src/agent.rs`:

| Method | Line | Returns | Match |
|--------|------|---------|-------|
| `backend()` | 749 | `AgentBackend` | MATCH |
| `model_tier()` | 790 | `ModelTier` | MATCH |
| `turn_budget()` | 832 | `TurnBudget` | MATCH |
| `tool_permissions()` | 867 | `ToolPermissions` | MATCH |

**ModelTier** enum at line 442-452: 3 variants `Fast`, `Standard`, `Premium` -- matches doc.

**TurnBudget** struct at line 462-467: `base_usd: f32`, `multiplier: f32` -- matches doc. Methods: `new()` at 472, `effective_usd()` at 481, `with_multiplier()` at 487.

**ToolPermissions** struct at line 502-513: 5 bool fields `read, write, exec, git, network` -- matches doc exactly. Convenience constructors: `full()` at 518, `read_only()` at 529, `read_exec()` at 541, `networked()` at 554.

Budget comparison (sample, doc vs code):

| Role | Doc Budget | Code Budget (line 833) | Match |
|------|-----------|------------------------|-------|
| Conductor | $0.10 | $0.10 | MATCH |
| Implementer | $1.50 | $1.50 (line 853) | MATCH |
| Architect | $3.00 | $3.00 (line 860) | MATCH |
| Researcher | $1.50 | $1.00 (line 852) | DIFFERS |

Permission comparison (sample):

| Role | Doc Permissions | Code Permissions | Match |
|------|----------------|------------------|-------|
| Implementer | Read+Write+Exec | `full()` (R+W+E+Git) (line 871) | ENHANCED |
| Auditor | Read | `read_only()` (line 897) | MATCH |
| Researcher | Read+Network | `networked()` (R+E+Net) (line 894) | ENHANCED |

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.11.1 | Some budget values differ between doc and code (e.g., Researcher: doc $1.50, code $1.00) | agent.rs:852 vs doc 04 | LOW (code is authoritative) |
| A.11.2 | Some permission sets are enhanced vs doc (Implementer gets git=true for MergeResolver only) | agent.rs:871-878 | LOW (code is finer-grained) |

### Verify
```bash
grep -n 'pub const fn backend\|pub const fn model_tier\|pub const fn turn_budget\|pub const fn tool_permissions' crates/roko-core/src/agent.rs
```

---

## A.12 — Role Permission Enforcement in ToolDispatcher (Doc 04)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
The `ToolDispatcher` at `crates/roko-agent/src/dispatcher/mod.rs:198` checks `def.permission.satisfied_by(&role_perms)` before allowing any tool call to proceed. A `Reviewer` role cannot write files even if the model requests `write_file` -- the dispatcher blocks with `ToolError::PermissionDenied`.

### What exists
Permission enforcement at `crates/roko-agent/src/dispatcher/mod.rs:194-224`:

```rust
// Line 198-204: Build role permissions from context
let role_perms = ToolPermissions {
    read: ctx.capabilities.read,
    write: ctx.capabilities.write,
    exec: ctx.capabilities.exec,
    git: ctx.capabilities.git,
    network: ctx.capabilities.network,
};
// Line 205: Check
if !def.permission.satisfied_by(&role_perms) {
```

The 6-step pipeline documented in the module doc comment at lines 1-31:
1. **Validate** args against registry's JSON schema (line 140)
2. **Resolve** the `ToolDef` for canonical name (line 156)
3. **Authorize** via `def.permission.satisfied_by(&role_perms)` (line 205)
4. **Safety checks** via `SafetyLayer.check_pre_execution()` (line 238)
5. **Race** handler.execute against timeout + cancellation (line 283)
6. **Truncate** oversized results (line 290)
Plus step 7: **Scrub** secrets from output (line 292)

The pipeline also includes a task-level tool filter step (line 172) that checks `allowed_tools` and `denied_tools` lists -- not documented in doc 04 but adds defense-in-depth.

Tests confirm enforcement:
- `missing_permission_returns_permission_denied` at line 676
- `allowlist_blocks_unlisted_tool_with_clear_error` at line 707
- `denylist_blocks_listed_tool_with_clear_error` at line 733
- `permission_denial_emits_failure_audit_signals` at line 879

### Gaps
None. Implementation matches and exceeds spec.

### Verify
```bash
grep -n 'satisfied_by\|PermissionDenied' crates/roko-agent/src/dispatcher/mod.rs | head -10
```

---

## A.13 — Agent Composition (Doc 00)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Algebraic composition via `AgentComposition` enum with 4 variants:
- `Pipeline(Vec<Box<dyn Agent>>)`
- `Parallel { agents, merge: MergeStrategy }`
- `Conditional { router, branches }`
- `MixtureOfAgents { layers, aggregator }`

`MergeStrategy` enum: `Concatenate`, `Aggregate(Box<dyn Agent>)`, `MajorityVote`, `BestOfN { n }`.

`CompositeAgent` wraps an inner agent plus a skill library. `SkillSelector` uses HDC embeddings and transition graphs.

### What exists
`AgentComposition` at `crates/roko-agent/src/composition.rs:142-161`:

| Variant | Doc | Code | Match |
|---------|-----|------|-------|
| `Pipeline` | `Vec<Box<dyn Agent>>` | `Vec<Box<dyn Agent>>` (line 144) | MATCH |
| `Parallel` | `{ agents, merge }` | `(Vec<Box<dyn Agent>>, MergeStrategy)` (line 146) | MATCH (tuple vs struct) |
| `Conditional` | `{ router: Box<dyn Fn>, branches }` | `{ condition: Box<dyn Fn(&Task) -> usize>, branches }` (line 148) | ENHANCED (Task-typed routing) |
| `MixtureOfAgents` | `{ layers, aggregator }` | `{ agents, aggregator }` (line 155) | MATCH (renamed `layers`->`agents`) |

`MergeStrategy` at `crates/roko-agent/src/composition.rs:19-29`:

| Variant | Doc | Code | Match |
|---------|-----|------|-------|
| `Concatenate` | Yes | line 22 | MATCH |
| `Aggregate(Box<dyn Agent>)` | Agent-backed | `Aggregate` (enum variant, no inner agent) | SIMPLIFIED |
| `MajorityVote` | Yes | `Vote` (line 26) | RENAMED |
| `BestOfN { n }` | Yes | `BestOfN` (line 28, no `n` field) | SIMPLIFIED |

`CompositeAgent` at line 214: wraps `name: String` + `composition: AgentComposition`. Different from doc's `inner + skills + selector + max_skills_per_prompt` design -- doc describes a skill-compilation approach while code implements a composition-operator approach.

`SkillSelector` at line 44: uses `TaskCategory`, `TaskComplexityBand`, `TaskReasoningLevel`, `TaskSpeedPriority`, `TaskQualityProfile` -- NOT HDC embeddings + transition graphs as doc describes.

All 4 composition patterns are fully implemented with working `Agent` trait impl at line 451. Tests at lines 504-571 verify pipeline, parallel, and conditional execution.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.13.1 | `CompositeAgent` is a composition wrapper, not the skill-compilation model described in doc (no `AgentSkill`, no HDC embeddings) | composition.rs:214 | LOW (design evolved) |
| A.13.2 | `SkillSelector` uses task metadata routing instead of HDC embeddings + transition graphs | composition.rs:44 | LOW (practical simplification) |
| A.13.3 | `MergeStrategy::Aggregate` has no inner agent -- simplified from doc's `Aggregate(Box<dyn Agent>)` | composition.rs:24 | LOW |

### Verify
```bash
grep -n 'pub enum AgentComposition\|pub enum MergeStrategy\|pub struct CompositeAgent\|pub struct SkillSelector' crates/roko-agent/src/composition.rs
```

---

## A.14 — Agent Introspection (Doc 00)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`AgentIntrospection` struct with 5 fields: identity, recent_episodes, resource_usage, confidence, available_tools. `AgentIdentity` with role, model_tier, temperament, capabilities. `MetacognitiveMonitor` with configurable thresholds. `Intervention` enum: EscalateModel, HumanHandoff, Abort, InjectReflection.

### What exists
At `crates/roko-agent/src/introspection.rs`:

**`AgentIdentity`** at line 12: 4 fields:
- `role: AgentRole` (match)
- `model_tier: roko_core::ModelTier` (match)
- `temperament: String` (match, doc had `Temperament` type)
- `capabilities: ToolPermissions` (match, doc had `Vec<String>`)

**`Turn`** at line 38: per-turn observation (not in doc). Fields: `index`, `assistant_text`, `reasoning`, `tool_calls`, `confidence`.

**`Intervention`** at line 77: 4 variants:
- `EscalateModel` (match, doc had `EscalateModel(ModelTier)`)
- `HumanHandoff` (match, doc had `HumanHandoff(String)`)
- `Abort` (match, doc had `Abort(String)`)
- `InjectReflection(String)` (match)

**`MetacognitiveMonitor`** at line 90: 4 thresholds:
- `repeat_threshold: usize` (match)
- `contradiction_window: usize` (additional)
- `confidence_threshold: f32` (match)
- `human_handoff_threshold: f32` (additional)

`check()` method at line 115 -- fully implemented (not `todo!()` as doc showed). Checks: repeated tool calls, contradictions, confidence thresholds.

**Not implemented**: `AgentIntrospection` (the top-level struct), `EpisodeSummary`, `ResourceUsage`. These remain conceptual; the `MetacognitiveMonitor` and `AgentIdentity` are the practical extractions.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.14.1 | `AgentIntrospection` struct not implemented (doc's umbrella type that collects identity + episodes + resources + tools) | introspection.rs | LOW (individual components exist) |
| A.14.2 | `EpisodeSummary` and `ResourceUsage` structs not implemented | introspection.rs | LOW (data available elsewhere) |
| A.14.3 | `Intervention` variants simplified (no inner data for EscalateModel, HumanHandoff, Abort) | introspection.rs:77 | LOW |

### Verify
```bash
grep -n 'pub struct AgentIdentity\|pub enum Intervention\|pub struct MetacognitiveMonitor' crates/roko-agent/src/introspection.rs
```

---

## A.15 — Agent Metamorphosis (Doc 00)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`MorphableAgent` wrapping an inner agent with dynamic role switching. `RoleProfile` with clarity/differentiation/alignment scores. `should_morph()` and `morph()` methods. `allowed_transitions` safety constraint.

### What exists
At `crates/roko-agent/src/metamorphosis.rs`:

**`RoleProfile`** at line 13: 4 fields: `role`, `clarity: f32`, `differentiation: f32`, `alignment: f32` -- matches doc (doc used `f64`, code uses `f32`).

**`MorphError`** at line 38: `TransitionDenied { from, to }` -- matches doc concept.

**`MorphableAgent`** at line 46: 6 fields: `inner`, `identity`, `profile`, `allowed_transitions`, `system_prompt`, `name`.

Methods:
- `morph()` at line 117 -- fully implemented (not `todo!()`). Checks `allowed_transitions`, updates identity, profile, system prompt, and name.
- `should_morph()` -- NOT implemented (doc showed this as a task-alignment heuristic). Only `morph()` exists; callers decide when to morph.
- `with_transitions()` at line 84 -- transition override.
- Implements `Agent` trait at line 162 with input augmentation.

`default_transition_matrix()` at line 195 defines safe transitions:
```
Implementer -> [QuickReviewer, Auditor, Refactorer]
QuickReviewer -> [Auditor]
Auditor -> [Implementer, Critic]
Strategist -> [Implementer, Architect, Researcher]
Researcher -> [Strategist, Implementer, Auditor]
Conductor -> [Strategist, Implementer, Auditor]
Refactorer -> [Auditor, Implementer]
```

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.15.1 | `should_morph()` method not implemented (doc showed task-alignment heuristic) | metamorphosis.rs | LOW (callers decide externally) |
| A.15.2 | `RoleProfile` uses `f32` instead of doc's `f64` | metamorphosis.rs:17-19 | LOW (precision sufficient) |

### Verify
```bash
grep -n 'pub fn morph\|pub struct MorphableAgent\|pub struct RoleProfile' crates/roko-agent/src/metamorphosis.rs
```

---

## A.16 — Capability-Based Security / AgentWarrant (Doc 00)

- **Status**: DONE
- **Priority**: --
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
OCaps model with `AgentWarrant` (unforgeable, attenuating capability token), `Capability` enum (Tool, ReadPath, WritePath, Exec, Network), `WarrantConstraint` (Subpath, Ttl, MaxInvocations, Cel). Warrants have cryptographic chain and expiration.

### What exists
At `crates/roko-agent/src/safety/capabilities.rs`:

**`Capability`** at line 12: 5 variants:
- `Tool(String)` -- match
- `ReadPath(PathBuf)` -- match
- `WritePath(PathBuf)` -- match
- `Exec(String)` -- match
- `Network { host: String, port: u16 }` -- match (structured vs doc's `Network(String)`)

**`AgentWarrant`** at line 27: 5 fields:
- `id: [u8; 32]` -- simplified from doc's cryptographic chain
- `capabilities: Vec<Capability>` -- match
- `issuer: String` -- match
- `expires_at: Option<u64>` -- match (unix seconds vs doc's `SystemTime`)
- `delegate_depth: u8` -- additional (controls delegation chain depth)

Functions:
- `check_capability()` at line 78 -- matches doc concept
- `delegate()` at line 86 -- implemented with depth checking and subset validation
- `network_capability_from_url()` at line 132 -- additional utility
- `exec_capability_from_command()` at line 146 -- additional utility

Re-exported from `crates/roko-agent/src/lib.rs:88`: `AgentWarrant, Capability, CapabilityError, check_capability, delegate`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| A.16.1 | `WarrantConstraint` enum not implemented (Subpath, Ttl, MaxInvocations, Cel) | capabilities.rs | LOW (constraints handled via delegate_depth + expires_at) |
| A.16.2 | No cryptographic delegation chain (`DelegationHop` with signatures) | capabilities.rs | LOW (simplified to depth counter) |

### Verify
```bash
grep -n 'pub struct AgentWarrant\|pub enum Capability\|pub fn check_capability\|pub fn delegate' crates/roko-agent/src/safety/capabilities.rs
```

---

## Section Summary

| Item | Doc | Status | Parity |
|------|-----|--------|--------|
| A.01 | Agent Trait (3 methods) | DONE | 100% -- all 3 methods, correct signatures |
| A.02 | AgentResult Struct | DONE | 100% -- all fields + constructors + extras |
| A.03 | Usage Tracking | PARTIAL | 85% -- 6/7 fields, `model` field missing |
| A.04 | Concrete Implementations | DONE | 270% -- 19 implementations vs 7 documented |
| A.05 | ChatResponse Type | DONE | 95% -- present but duplicated across 2 modules |
| A.06 | FinishReason Enum | DONE | 100% -- exact match including normalization |
| A.07 | ResponseMetadata | PARTIAL | 90% -- matched but duplicated with divergent fields |
| A.08 | ChatResponse in roko-core | NOT DONE | 0% -- types still in roko-agent, not migrated |
| A.09 | BackendResponse | DONE | 110% -- exceeds spec with Gemini support |
| A.10 | AgentRole Enum (28 variants) | DONE | 100% count -- names evolved from doc sketch |
| A.11 | Per-Role Defaults | DONE | 95% -- all 4 methods, minor value diffs |
| A.12 | ToolDispatcher Permission Enforcement | DONE | 110% -- full pipeline + task-level filters |
| A.13 | Agent Composition | DONE | 85% -- all patterns, simplified SkillSelector |
| A.14 | Agent Introspection | DONE | 70% -- core types exist, umbrella struct missing |
| A.15 | Agent Metamorphosis | DONE | 90% -- morph works, should_morph not implemented |
| A.16 | AgentWarrant / OCaps | DONE | 75% -- warrants work, constraints simplified |

### Priority actions
1. **P1** (A.08): Migrate `ChatResponse`, `FinishReason`, `ResponseMetadata` to `roko-core` so `roko-compose` can use them
2. **P2** (A.05/A.07): Reconcile the two copies of `ChatResponse` and `ResponseMetadata` in roko-agent (chat_types.rs vs translate/mod.rs)
3. **P2** (A.03): Add `model` field to `Usage` struct for cost attribution

---

## Agent Execution Notes

### A.03 — Usage Tracking

Do not treat the missing `model` field as an isolated one-line fix unless ownership is already clear.

Recommended sequence:

1. decide whether `Usage` stays agent-owned or becomes part of a shared core response surface,
2. add model attribution in the owning type,
3. add one test proving attribution survives the relevant construction path.

Acceptance criteria:

- model attribution exists in the shared runtime contract,
- the field is actually populated on at least one live path or the missing producer is documented,
- the patch does not create another near-duplicate usage type.

### A.05 / A.07 — Response Type Duplication

Treat this as a prerequisite for later crate extraction.

Recommended slice:

1. collapse to one canonical definition inside `roko-agent`,
2. preserve compatibility with re-exports if needed,
3. only then move the shared surface into `roko-core`.

Acceptance criteria:

- one real `ChatResponse` definition,
- one real `ResponseMetadata` definition,
- translator and tool-loop code compile against the same owner.

### A.08 — Shared Response Ownership

This is high leverage but cross-crate. Keep it bounded:

- move only the minimum shared response surface,
- leave compatibility re-exports where they reduce churn,
- avoid widening into provider-specific payload redesign.
