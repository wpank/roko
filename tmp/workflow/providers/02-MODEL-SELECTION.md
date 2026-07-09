# 02 — Model Selection & Hardcoded Fallbacks

## The Problem

Mori infers the backend from the model slug, uses per-role defaults, has a fallback model
chain, and health-based routing. Roko has hardcoded model strings scattered across 13+
locations with no consistent resolution.

---

## How Mori Selects Models

### 1. Backend Detection from Model Slug
**File**: `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/roles.rs:19-48`

```
slug.starts_with("claude-")        → Claude CLI
slug.starts_with("composer-")      → Cursor ACP
slug.starts_with("cursor-")        → Cursor ACP
slug == "auto"                     → Cursor ACP
slug.starts_with("sonnet-")        → Cursor ACP  (cursor's naming)
slug.starts_with("opus-")          → Cursor ACP
slug.starts_with("haiku-")         → Cursor ACP
slug.starts_with("gemini-")        → Cursor ACP
slug.starts_with("kimi-")          → Cursor ACP
slug == "gpt-5.2"                  → Cursor ACP
slug.ends_with("-high")            → Cursor ACP
slug.ends_with("-xhigh-fast")      → Cursor ACP
everything else (gpt-*, o3, etc.)  → Codex
```

### 2. Per-Role Default Backend
**File**: `roles.rs:179-213`

| Roles | Default Backend |
|-------|----------------|
| Conductor, Implementer, Strategist, Auditor, Scribe, Critic, Researcher, AutoFixer, QuickReviewer, FullLoopValidator | Claude |
| Architect, Refactorer, PrePlanner, DocVerifier, IntegrationTester, MergeResolver, TerminalValidator, and ~12 more | Codex |

### 3. Model Override Chain
**File**: `mod.rs:67-131`

```
1. Explicit model override (from task/config)
2. → Infer backend from slug
3. → If no override, use role's default backend
4. Spawn with detected backend
5. → On failure, retry with fallback_model
```

### 4. Fallback Model
Always `claude-haiku-4-5` — passed via `--fallback-model` to Claude CLI.
If primary spawn fails, retry once with fallback.

### 5. Default Model
`claude-opus-4-6` (connection.rs:2453)

---

## How Roko Selects Models (Current State)

### run.rs `dispatch_agent()` (lines 465-637)

```
1. Has routing config with providers/models?
   → Use routing_config.agent.default_model
   → Falls back to "claude-sonnet-4-6" (run.rs:530)

2. Claude CLI + ANTHROPIC_API_KEY?
   → Hardcoded "claude-sonnet-4-6-20250514" (dispatch_direct.rs:208)

3. Claude CLI bare?
   → Check routing_config.agent.default_model first
   → Fall back to "claude-sonnet-4-6" (run.rs:530)

4. Ollama?
   → Hardcoded "llama3.1:8b" (run.rs:657)

5. Known protocol?
   → From synthesized config

6. Generic?
   → Command name itself
```

### dispatch_direct.rs `dispatch_prompt()` (chat)

```
AuthMethod::ClaudeCli       → Claude CLI subprocess (model from stream)
AuthMethod::AnthropicApi    → "claude-sonnet-4-6-20250514" (line 208)
AuthMethod::OpenAiCompat    → "gpt-4o" (line 291)
```

### orchestrate.rs `dispatch_agent_with()` (plan execution)

```
1. Explicit override from caller
2. Task-specific from tasks.toml task_def.model
3. Tier-based via task_def.tier → tier_models config
4. CascadeRouter bandit selection (if wired)
5. Fallback: "claude-sonnet-4-6" (with task_def) or "claude-opus-4-6" (generic)
```

### resolved_model() in run.rs (line 1107)

```
1. config.agent.model (explicit)
2. routing_config.agent.default_model (global config merge)
3. "claude-sonnet-4-6" (hardcoded fallback)
```

---

## All Hardcoded Model Strings in Roko

### Runtime-critical (affect actual dispatch)

| File | Line | Value | Context |
|------|------|-------|---------|
| `roko-cli/src/run.rs` | 530 | `"claude-sonnet-4-6"` | CLI fallback |
| `roko-cli/src/run.rs` | 657 | `"llama3.1:8b"` | Ollama fallback |
| `roko-cli/src/dispatch_direct.rs` | 208 | `"claude-sonnet-4-6-20250514"` | Anthropic API default |
| `roko-cli/src/dispatch_direct.rs` | 291 | `"gpt-4o"` | OpenAI-compat default |
| `roko-cli/src/orchestrate.rs` | 13987 | `"claude-sonnet-4-6"` | Task dispatch fallback |
| `roko-cli/src/orchestrate.rs` | 13998 | `"claude-sonnet-4-6"` | Tier fallback |
| `roko-cli/src/orchestrate.rs` | 14020 | `"claude-opus-4-6"` | Generic task fallback |
| `roko-cli/src/auth_detect.rs` | 42 | `"claude-sonnet-4-6"` | Auth model default |

### Test/learning (not runtime-critical)

| File | Value | Context |
|------|-------|---------|
| `roko-primitives/src/tier.rs` | `claude-haiku-4-5`, `claude-opus-4-6`, `claude-sonnet-4` | Tier routing matrix |
| `roko-conductor/src/watchers/` | `claude-sonnet-4-6`, `claude-opus-4-6` | Synthetic test signals |
| `roko-dreams/src/` | `claude-haiku-4-5`, `claude-opus-4-6` | Dream consolidation |
| `roko-neuro/src/distiller.rs` | `claude-haiku-3-5`, `claude-sonnet-4-5` | Distillation |
| `roko-agent/tests/` | Various | Integration tests |

---

## What Needs to Change

### 1. Single model resolution function
All dispatch paths should call one `resolve_model(config, role, task)` function that:
- Checks explicit override
- Checks task-specific model
- Checks `routing_config.agent.default_model`
- Checks `config.agent.model`
- Falls back to a constant, not a string literal

### 2. CascadeRouter consulted at runtime
Currently "built but never used" (`wired-unproven` status).
Orchestrate.rs has the wiring but runner/chat paths skip it entirely.

### 3. Fallback model applied consistently
Only one path (Claude CLI bare at run.rs:540-543) passes `--fallback-model`.
Should be applied to all Claude CLI spawns.

### 4. Backend detection from slug
Roko has no equivalent of mori's `AgentBackend::from_model()`.
Auth detection (`auth_detect.rs`) is env-var based, not model-based.
