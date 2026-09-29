# depth/14-feeds/03 -- Recipe DAGs

> Validated directed acyclic graphs of deterministic Score operations.
> TOML persistence, topological evaluation, and the six built-in
> Score transformations.

**Parent**: [14-FEEDS-RECIPES.md](../../14-FEEDS-RECIPES.md) -- Section 6

---

## 1. Design Rationale

Recipes exist because many data transformations in an agent system are
pure functions: they take numeric inputs, apply deterministic math, and
produce numeric outputs. These do not need LLM inference, agent
dispatch, or the overhead of a full Graph execution. A Recipe is the
minimal abstraction for composable, versionable, testable scoring
pipelines.

The recipe system deliberately has no plugin or custom operation
extensibility at evaluation time. Unknown `ScoreOp::Custom(name)`
variants are rejected with a clear error. This fail-closed behavior
prevents untested transforms from silently corrupting downstream
values.

---

## 2. Recipe Structure

A Recipe is a four-part structure:

```
Recipe
  +-- identity (id, name, version)
  +-- inputs   (input_feeds: Vec<String>)
  +-- graph    (nodes: Vec<RecipeNode>, edges: Vec<RecipeEdge>)
  +-- output   (output_schema: Option<Value>)
```

### 2.1 Nodes

Each `RecipeNode` names a Score operation and provides its parameters:

```rust
pub struct RecipeNode {
    pub id:        String,              // Unique within the recipe
    pub operation: ScoreOp,             // One of 6 built-ins or Custom
    pub params:    HashMap<String, f64>, // Numeric parameters
}
```

Nodes are pure: they take `Vec<(source_id, f64)>` and return `f64`.

### 2.2 Edges

Each `RecipeEdge` routes a value from an input feed or upstream node
to a downstream node:

```rust
pub struct RecipeEdge {
    pub from:  String,   // Source: an input_feed name or a node ID
    pub to:    String,   // Destination: a node ID
    pub field: String,   // Optional: extract this JSON field from the source
}
```

When `field` is non-empty, the evaluator calls `value.get(field)` on
the source's JSON output and extracts the named field before numeric
conversion.

### 2.3 Input Feeds

`input_feeds: Vec<String>` declares the named external inputs the
recipe requires. At evaluation time, the caller must provide a value
for every declared input feed. Missing inputs cause an immediate
evaluation error.

---

## 3. The Six Built-In Score Operations

### 3.1 WeightedAverage

Computes the weighted mean of all incoming edges. Weights are taken
from the node's `params` map keyed by source ID. Missing weights
default to 1.0.

```
result = sum(value_i * weight_i) / sum(weight_i)
```

**Errors**: No inputs. Zero total weight.

### 3.2 Normalize

Maps a value from the range `[min, max]` to `[0, 1]`:

```
result = (value - min) / (max - min)
```

**Parameters**: `min` (default 0.0), `max` (default 1.0).
**Errors**: `min == max` (zero-width range).

### 3.3 Threshold

Binary classification:

```
result = if value >= threshold then 1.0 else 0.0
```

**Parameters**: `threshold` (default 0.5).

### 3.4 ZScore

Standardization:

```
result = (value - mean) / stddev
```

**Parameters**: `mean` (default 0.0), `stddev` (default 1.0).
**Errors**: `stddev == 0`.

### 3.5 Clamp

Bound a value to `[min, max]`:

```
result = value.clamp(min, max)
```

**Parameters**: `min` (default 0.0), `max` (default 1.0).
**Errors**: `min > max`.

### 3.6 Rescale

Linear mapping between arbitrary ranges:

```
result = to_min + (value - from_min) * (to_max - to_min) / (from_max - from_min)
```

**Parameters**: `from_min`, `from_max`, `to_min`, `to_max` (all
default to 0.0/1.0).
**Errors**: `from_min == from_max` (zero-width source range).

---

## 4. Validation Rules

`Recipe::validate()` returns a `Vec<String>` of errors. An empty vector
means the recipe is valid. The checks, in order:

1. **Non-empty ID**: `recipe.id` must not be empty.
2. **Unique node IDs**: duplicate node identifiers are rejected.
3. **Unique input feeds**: duplicate input feed identifiers are rejected.
4. **Finite parameters**: every `f64` value in every node's `params`
   must be finite (not NaN, not infinity).
5. **Edge destination exists**: every `edge.to` must reference an
   existing node ID.
6. **Edge source exists**: every `edge.from` must reference either an
   existing node ID or an existing input feed ID.
7. **Acyclicity**: the graph must be a DAG. Checked by running Kahn's
   algorithm; if the topological sort cannot consume all nodes, a cycle
   exists.

---

## 5. Topological Evaluation Algorithm

`Recipe::evaluate(inputs)` implements a single-pass forward evaluation:

```
1. validate()  -- fail if any errors
2. for each input_feed:
     assert inputs.contains_key(feed)  -- fail if missing
3. order = topological_order()  -- Kahn's algorithm
4. values = inputs.clone()
5. for node_id in order:
     incoming = edges where edge.to == node_id
     for each incoming edge:
       value = values[edge.from]
       if edge.field is non-empty:
         value = value[edge.field]
       convert to f64  -- fail if non-numeric
     output = evaluate_node(node, incoming)  -- fail if non-finite
     values[node_id] = output
6. sinks = nodes with no outgoing edges
7. if one sink: return values[sink.id]
   if multiple sinks: return { sink_id: values[sink_id], ... }
   if no sinks: error
```

### 5.1 Kahn's Algorithm for Topological Order

```
1. Compute in-degree for each node (count edges where edge.to == node)
2. Initialize queue with all zero-in-degree nodes
3. While queue is non-empty:
     Pop node from queue, append to order
     For each outgoing edge from node:
       Decrement in-degree of destination
       If destination in-degree reaches 0, enqueue it
4. If order.len() != nodes.len(): cycle detected
```

### 5.2 Finite Output Guarantee

After each node evaluation, the result is checked with `is_finite()`.
Non-finite results (NaN, infinity) cause an immediate error. This
prevents silent corruption: a NaN at node 2 in a 10-node chain would
propagate through every downstream node without this check.

---

## 6. TOML Persistence

### 6.1 Storage Layout

```
.roko/recipes/
  {recipe_id}.toml
```

Recipe IDs must match: `[a-zA-Z0-9_-]{1,128}`. This restriction
prevents path traversal (`../secret`) and overly long filenames.

### 6.2 Atomic Writes

`RecipeStore::save()` uses `atomic_write_str()` (write to temporary
file, then `rename(2)`) to prevent partial writes from corrupting
recipe files.

### 6.3 Version Management

Saving a recipe follows these version rules:

- If the recipe is new (no existing file), the version is set to at
  least 1.
- If the recipe already exists, the version is incremented beyond both
  the existing version and the submitted version. This prevents
  version collisions when multiple writers save concurrently.

### 6.4 Example TOML File

```toml
id = "quality-score"
name = "Code Quality Score"
version = 3
input_feeds = ["lint_warnings", "test_coverage", "complexity"]

[[nodes]]
id = "normalize_lint"
operation = "Normalize"

[nodes.params]
min = 0.0
max = 100.0

[[nodes]]
id = "normalize_coverage"
operation = "Normalize"

[nodes.params]
min = 0.0
max = 100.0

[[nodes]]
id = "normalize_complexity"
operation = "Rescale"

[nodes.params]
from_min = 1.0
from_max = 50.0
to_min = 1.0
to_max = 0.0

[[nodes]]
id = "blend"
operation = "WeightedAverage"

[nodes.params]
normalize_lint = 0.3
normalize_coverage = 0.4
normalize_complexity = 0.3

[[edges]]
from = "lint_warnings"
to = "normalize_lint"

[[edges]]
from = "test_coverage"
to = "normalize_coverage"

[[edges]]
from = "complexity"
to = "normalize_complexity"

[[edges]]
from = "normalize_lint"
to = "blend"

[[edges]]
from = "normalize_coverage"
to = "blend"

[[edges]]
from = "normalize_complexity"
to = "blend"
```

Evaluation with `lint_warnings=20, test_coverage=85, complexity=12`:

```
normalize_lint       = (20 - 0) / (100 - 0)                = 0.20
normalize_coverage   = (85 - 0) / (100 - 0)                = 0.85
normalize_complexity = 1.0 + (12 - 1.0) * (0.0 - 1.0) / (50 - 1) = 0.776
blend = (0.20*0.3 + 0.85*0.4 + 0.776*0.3) / (0.3+0.4+0.3) = 0.633
```

---

## 7. REST and CLI Integration

### 7.1 REST Evaluation

```
POST /api/recipes/quality-score/evaluate
Content-Type: application/json

{
  "lint_warnings": 20,
  "test_coverage": 85,
  "complexity": 12
}

-> 200 OK
0.6328
```

### 7.2 CLI Evaluation

```bash
roko recipe run quality-score \
  --input lint_warnings=20 \
  --input test_coverage=85 \
  --input complexity=12

# Output: 0.6328
```

---

## 8. Source References

| File | What it contains |
|---|---|
| `crates/roko-core/src/recipe.rs` | `Recipe`, `RecipeNode`, `RecipeEdge`, `ScoreOp`, `validate()`, `evaluate()`, topological sort |
| `crates/roko-core/src/recipe_store.rs` | `RecipeStore`, TOML load/save/list/delete, path safety |
| `crates/roko-serve/src/routes/recipes.rs` | REST routes: list, get, save, delete, evaluate |
| `crates/roko-cli/src/commands/recipe.rs` | CLI commands: list, show, validate, run |
