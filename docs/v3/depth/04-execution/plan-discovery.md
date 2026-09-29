# Plan Discovery

> Depth file for [04-EXECUTION.md](../../04-EXECUTION.md) section 3.
> Preserves and updates content from v1 `01-orchestration/01-plan-discovery.md`.

---

## Overview

Plan discovery is the first stage of the execution pipeline. Before any
graph can be constructed or agent spawned, the system must answer: what
plans exist, what do they contain, and in what order should they run?

The discovery function scans a directory for plan files, parses their YAML
frontmatter, validates the results, and returns a ranked list of plan
entries ready for graph conversion.

**Source:** `crates/roko-cli/src/runner/plan_loader.rs`,
`crates/roko-cli/src/plan.rs`

---

## Directory Layout

Two layouts are supported. The directory layout takes precedence when both
exist for the same plan.

### Directory layout (preferred)

```
plans/
  01-workspace-scaffold/
    plan.md          -- plan description with YAML frontmatter
    tasks.toml       -- task definitions
    CONTEXT.md       -- optional context document (skipped by discovery)
  02-core-traits/
    plan.md
    tasks.toml
```

Each plan lives in a numbered directory (`<num>-<slug>/`). The numeric
prefix may include alpha suffixes (`08a-variant`), which sort after pure
numerics: `08` < `08a` < `09`.

### Legacy flat-file layout (fallback)

```
plans/
  01-workspace-scaffold.md
  02-core-traits.md
```

Flat `.md` files at the top level. The base name (minus `.md`) becomes the
plan identifier. Legacy plans are discovered only if no directory exists
with the same base name.

### Conflict resolution

When both `plans/03-foo/plan.md` and `plans/03-foo.md` exist, the
directory layout wins. The legacy flat file is silently skipped. This
ensures smooth migration from flat files to structured plan directories.

---

## YAML Frontmatter

Frontmatter lives between two `---` fences at the top of `plan.md`. All
fields are optional -- a plan without frontmatter discovers successfully
with `frontmatter = None`.

### Schema

```yaml
---
plan: "01-workspace-scaffold"
depends_on: ["00-init"]
parallel_with: ["02-core"]
crates_touched: ["roko-core", "roko-fs"]
estimated_tasks: 8
estimated_parallel_width: 4
estimated_minutes: 45
parallel_safe: true
priority: 10
tags: ["rust"]
milestone: "v0.2"
---
```

### Field semantics

| Field | Type | Default | Purpose |
|---|---|---|---|
| `plan` | `Option<String>` | `None` | Stable identifier for cross-plan references |
| `depends_on` | `Vec<String>` | `[]` | Plans that must complete first |
| `parallel_with` | `Vec<String>` | `[]` | Plans safe for concurrent execution |
| `crates_touched` | `Vec<String>` | `[]` | Crate dirs modified (for conflict inference) |
| `estimated_tasks` | `Option<usize>` | `None` | Advisory task count |
| `estimated_parallel_width` | `Option<usize>` | `None` | Advisory max concurrent agents (must be > 0) |
| `estimated_minutes` | `Option<u32>` | `None` | Advisory duration (must be > 0) |
| `parallel_safe` | `bool` | `true` | Whether tasks can run with other plans |
| `priority` | `Option<u32>` | `None` (0) | Higher values run first |
| `tags` | `Vec<String>` | `[]` | Free-form metadata |
| `milestone` | `Option<String>` | `None` | Milestone association |

### Parsing details

The frontmatter parser is BOM-tolerant (strips `U+FEFF` prefix) and
handles both LF and CRLF line endings. Parsing uses `serde_yaml_ng`.
If the YAML is malformed, discovery fails loudly with
`DiscoveryError::BadFrontmatter` rather than silently dropping the plan.
This is a deliberate design choice to catch errors early.

If a plan file starts with `---` but has no closing `---` fence, it is
treated as having no frontmatter (not an error). This allows writing plans
incrementally.

---

## Validation

After parsing, frontmatter is validated by `validate_frontmatter()`:

1. **Plan ID must not be empty.** `plan: ""` or `plan: "   "` is rejected
   with `ValidationError::MissingPlanId`. `plan` absent (`None`) is fine
   -- only an explicitly empty ID is an error.

2. **Estimated minutes must be > 0.** `estimated_minutes: 0` is rejected
   with `ValidationError::InvalidMinutes`.

3. **Estimated parallel width must be > 0.** `estimated_parallel_width: 0`
   is rejected with `ValidationError::InvalidParallelWidth`.

Validation is intentionally lax -- only load-bearing invariants trigger
errors. Missing optional fields are fine. This allows plans to be written
incrementally: start with just the prose, add frontmatter later as the
plan matures.

---

## Plan Ranking

After discovery, plans are sorted by `rank_plans()`:

1. **Primary sort:** priority (descending). Higher values run first.
2. **Secondary sort:** numeric prefix (ascending, lexicographic). Lower
   numbers run first among equal-priority plans.

```
priority: 10, num: "12" -- runs first
priority: 10, num: "13" -- runs second (same priority, lower num wins)
priority:  1, num: "11" -- runs third
priority:  0, num: "01" -- runs fourth (default priority)
```

The ranking determines the initial execution queue order. The queue can be
dynamically reordered during execution via conductor decisions or operator
intervention.

---

## PlanInfo Structure

```rust
pub struct PlanInfo {
    pub base: String,                     // "01-workspace-scaffold"
    pub num: String,                      // "01"
    pub path: PathBuf,                    // full path to plan.md
    pub frontmatter: Option<PlanFrontmatter>,
}
```

The `base` field is the plan's stable identifier throughout the system.
It appears in:

- Graph metadata (`graph_name`)
- Worktree branch names (`roko/plan/<base>`)
- Snapshot paths (`.roko/state/graph/<base>/`)
- Episode log entries (`plan_id` field)
- Cost tracking tables
- Event log payloads
- GitHub PR titles and branch names

---

## Integration with Execution

After discovery, the ranked `Vec<PlanInfo>` flows into the execution
pipeline:

```
discover_plans()
    -> Vec<PlanInfo>
    -> parse tasks.toml for each plan
    -> plan_to_graph() or ProductionPlanTopology::build()
    -> GraphEngine::new(graph, registry)
    -> drive_controller()
```

The plan's `depends_on` frontmatter feeds the `CrossPlanDag` for cross-plan
wave computation. The `crates_touched` field enables crate-overlap warnings
for plans in the same wave. The `parallel_safe` flag determines whether a
plan's tasks can be scheduled concurrently with tasks from other plans.

---

## Error Handling

| Error | Cause | Action |
|---|---|---|
| `DirMissing(path)` | Plans directory does not exist | Create it or fix path |
| `ReadFailed { path, source }` | I/O error reading plan file | Check permissions, disk space |
| `BadFrontmatter { path, reason }` | YAML parse error | Fix the YAML syntax |
| `Invalid { path, source }` | Validation failure | Fix the field value |

All errors include the offending file path for easy diagnosis.

---

## Test Coverage

The plan discovery module has comprehensive tests covering:

- Missing directory detection
- Empty directory returns empty vector
- Directory and flat-file discovery
- Directory layout wins on conflict
- Plans without frontmatter (discovers with `None`)
- Malformed YAML fails loudly (not silently dropped)
- Alpha-suffix prefix preservation (`08a`)
- Alpha-suffix sorting (`08` < `08a` < `09`)
- BOM prefix stripping (`U+FEFF`)
- Priority-based ordering with tie-breaking
- Directories without `plan.md` are skipped
- `CONTEXT.md` files are skipped
- Array fields parse correctly
- CRLF line endings are handled
- `parallel_safe` defaults to `true`
- Validation rejects zero minutes, zero width, empty plan ID
- Multiple plans sort deterministically

---

## Verification Commands

```bash
# List discovered plans with ranking
cargo run -p roko-cli -- plan list

# List with cross-plan DAG waves
cargo run -p roko-cli -- plan list --waves

# Validate a plan's tasks.toml without executing
cargo run -p roko-cli -- plan validate plans/<dir>

# Inspect the deterministic plans index
cargo run -p roko-cli -- plan index
```
