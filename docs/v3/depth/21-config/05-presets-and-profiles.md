# 21-config / 05 -- Presets and Profiles

> Named presets produce complete configs for different cost/quality tradeoffs.
> Domain profiles provide inheritable, task-specific overlays for cognitive
> postures like coding, research, and review.

**Parent**: [21-CONFIG](../../21-CONFIG.md)

---

## 1. Named Presets

Three presets are available, each producing a fully-populated `RokoConfig`:

```rust
pub enum Preset {
    Minimal,    // Fastest, cheapest: haiku-class, minimal gates
    Balanced,   // Default: sonnet-class, standard gates
    Thorough,   // Maximum quality: opus-class, all gates, full parallelism
}
```

### 1.1 Preset Comparison

| Dimension | Minimal | Balanced | Thorough |
|-----------|---------|----------|----------|
| **Model tier** | haiku-class (MODEL_FAST) | sonnet-class (MODEL_FOCUSED) | opus-class (MODEL_DEEP) |
| **Effort** | low | medium | high |
| **Context limit** | 100K | 200K | 300K |
| **Clippy** | off | off (default) | on |
| **Tests** | skipped | enabled | enabled |
| **Max iterations** | 1 | 3 | 5 |
| **Max agents** | 2 | 8 | 16 |
| **Parallel plans** | 1 | 2 | 4 |
| **Parallel enabled** | no | no (default) | yes |
| **Express mode** | yes | no (default) | no |
| **Plan budget** | $5 | unlimited ($0) | $100 |
| **Turn budget** | $1 | unlimited ($0) | $5 |
| **Task budget** | $1 | unlimited ($0) | $10 |
| **Prompt tokens** | 4,000 | 8,000 | 20,000 |
| **Playbook refresh** | off | off (default) | on |
| **File intel** | off | off (default) | on |
| **Warning patterns** | off | off (default) | on |
| **Wave context** | off | off (default) | on |
| **Error patterns** | off | off (default) | on |
| **Replan on failure** | off | off (default) | on |
| **Lookahead router** | off | off (default) | on |
| **Fallback model** | none | none | MODEL_FOCUSED |

The `balanced` preset is exactly `RokoConfig::default()`.

### 1.2 Parsing

Preset names are parsed case-insensitively with aliases:

| Input | Preset |
|-------|--------|
| `minimal`, `min`, `fast` | Minimal |
| `balanced`, `default`, `normal` | Balanced |
| `thorough`, `max`, `full` | Thorough |

### 1.3 CLI Usage

Presets apply to specific config sections, not the entire config:

```bash
# Apply thorough gates (clippy on, tests on, 5 iterations)
roko config preset gates --preset thorough

# Preview what routing changes would look like with minimal
roko config preset routing --preset minimal --dry-run

# Apply balanced budget with auto-confirmation
roko config preset budget --preset balanced --yes

# Apply thorough model settings
roko config preset model --preset thorough
```

Available preset targets: `gates`, `routing`, `budget`, `model`.

---

## 2. Domain Profiles

Domain profiles provide inheritable config overlays for named cognitive postures.
Each profile optionally overrides model, effort, context window, gate iterations,
tool profile, and gate configuration.

```rust
pub struct DomainProfile {
    pub name: String,
    pub base: Option<String>,           // Parent profile name
    pub model: Option<String>,
    pub effort: Option<String>,
    pub context_limit_k: Option<u32>,
    pub max_iterations: Option<u32>,
    pub tool_profile: Option<String>,
    pub gate_config: Option<GateProfileConfig>,
    pub pack: Option<String>,           // A [gates.packs.<name>] (9125)
    pub role_identity: Option<String>,  // One line that leads the prompt (9125)
    pub outbound: Option<OutboundPolicy>, // allow, stage or deny (9131)
    pub extra: HashMap<String, toml::Value>,
}
```

Domain packs as data (9125): a plan task whose work domain label is `L` follows
`[profiles.L]` when the workspace declares it, resolved through `base` (which may
end at a built-in profile). Its `pack` names the `[gates.packs.<name>]` that
verifies the task, in place of `[gates.packs.L]`; its `tool_profile` names the
built-in tool set (`coding`, `chain`, `research` or `general`, from
`roko_std::roles`) the task's agent gets; and its `role_identity` leads the task's
prompt. Plan runs ignore `model`, `effort` and `max_iterations`, and log a warning
when a profile sets them: the tier ladder picks each task's model.

Outbound effects (9131): a tool call that sends, posts, pays or changes a remote
system (`roko_agent::safety::effects::is_outbound_effect`: an MCP tool that is
destructive, or open-world and not read-only, with an omitted MCP hint taking the
spec's default; a plugin tool with network and write access) follows the task's
outbound policy. `allow` runs it, `deny` refuses it, and `stage` holds it for a
person's approval in `.roko/state/effect-holds/<run>/<effect_id>.json` (mode 0600)
and tells the agent it has not run. The policy is the plan's `[meta] outbound`,
which a chat host's `roko run` sets to `stage`; else the profile's `outbound`;
else decision 9107's default, `stage` in the `ops` domain and `allow` elsewhere.
Only in-process tool loops are covered: CLI agents such as Claude Code run their
own tools.

`roko effects list | show <id> | approve <id> | reject <id>` decides a held effect
(9132). An approval replays the call once through a fresh dispatcher over the
workspace's `.mcp.json` servers, behind an `applying` marker (a crash after the
marker leaves the effect `ambiguous`, never retried), then runs the `receipt` rungs
of the task's pack with the effect's JSON in the file `ROKO_EFFECT_FILE` names.
Each decision appends a record (outcome `applied`, `failed`, `ambiguous` or
`rejected`, the result's scrubbed tail, the receipt verdicts, no arguments) to
`.roko/state/effects.jsonl`, and the hold is removed.

```toml
[gates.packs.deep-research]
rungs = [{ name = "sources", kind = "citations", artefacts = ["report.md"] }]

[profiles.research]
name = "research"
pack = "deep-research"
tool_profile = "research"
role_identity = "You are a careful research analyst who cites every source."
```

### 2.1 Built-in Profiles

Three built-in cognitive postures ship as inheritance bases:

```rust
pub fn builtin_profiles() -> HashMap<String, DomainProfile> {
    // "coding": effort=high, max_iterations=3, tool_profile="full"
    // "research": effort=medium, context_limit_k=200, skip_tests=true
    // "review": effort=low, max_iterations=1
}
```

| Profile | Purpose | Key overrides |
|---------|---------|--------------|
| `coding` | Implementation tasks | High effort, 3 gate iterations, full tool profile |
| `research` | Information gathering | Medium effort, 200K context, tests skipped |
| `review` | Code review / inspection | Low effort, 1 iteration (no retries) |

### 2.2 Inheritance

Profiles support single-parent inheritance with `base`:

```toml
[profiles.my-coding]
base = "coding"               # Inherits from built-in "coding"
model = "claude-opus-4-6"     # Override model
context_limit_k = 300         # Override context

[profiles.deep-research]
base = "research"
model = "claude-opus-4-6"
context_limit_k = 500
```

Inheritance rules:

1. **Depth limit**: Maximum 5 levels. Exceeding this returns an error.
2. **Cycle detection**: The resolver tracks visited profile names and rejects
   cycles (e.g., A -> B -> A).
3. **Resolution semantics**: Child fields that are `Some` win; `None` falls
   through to the parent via `Option::or()`.
4. **Gate config overlay**: `GateProfileConfig` fields overlay independently
   within their own struct.
5. **Extension fields**: The `extra` map merges additively (child keys
   overwrite parent keys with the same name; parent-only keys are retained).

```rust
pub fn resolve_profile(
    name: &str,
    profiles: &HashMap<String, DomainProfile>,
) -> Result<DomainProfile, String> {
    // Recursive resolution with cycle detection and depth limit
    fn resolve(
        name: &str,
        profiles: &HashMap<String, DomainProfile>,
        stack: &mut Vec<String>,
        depth: usize,
    ) -> Result<DomainProfile, String> {
        if stack.iter().any(|seen| seen == name) {
            return Err(format!("domain profile inheritance cycle: {}", stack.join(" -> ")));
        }
        if depth > 5 {
            return Err(format!("domain profile inheritance exceeds maximum depth 5 at '{name}'"));
        }
        // ... resolve parent, overlay child
    }
}
```

### 2.3 Profile Usage in Config

```toml
# Define custom profiles
[profiles.my-impl]
base = "coding"
model = "claude-opus-4-6"

[profiles.my-triage]
base = "review"
effort = "medium"

# Reference a profile as the default domain
[agent]
domain = "my-impl"
```

Profiles are resolved at dispatch time: the runner looks up the task's domain
profile, resolves inheritance, and applies the resulting overrides to the
agent's configuration for that task.
