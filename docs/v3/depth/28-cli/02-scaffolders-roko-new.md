# 28.02 -- Scaffolders (`roko new`)

> Depth file for [28-CLI.md](../../28-CLI.md) -- v1/12/02.

---

## Overview

`roko new <type> <name>` generates compilable Rust boilerplate for the 12
kernel traits. Each scaffold produces one or more `.rs` files with a working
implementation, tests, and doc comments.

## Supported Scaffold Types

Nine scaffold types are currently supported:

```rust
pub const SCAFFOLD_TYPES: &[&str] = &[
    "gate",          // Verify trait implementation
    "scorer",        // Score trait implementation
    "router",        // Route trait implementation
    "policy",        // React trait implementation
    "substrate",     // Store trait implementation
    "composer",      // Compose trait implementation
    "domain",        // Full domain profile with config, gates, templates
    "template",      // Prompt template module
    "event-source",  // EventSource trait implementation
];
```

## Usage

```bash
# Generate a gate scaffold
roko new gate custom-lint
# -> Creates custom_lint_gate.rs

# Generate a full domain profile
roko new domain payment-processing
# -> Creates payment_processing/ directory with multiple files

# Generate a scorer
roko new scorer relevance
# -> Creates relevance_scorer.rs
```

## Name Conversion

Scaffold names support kebab-case and snake_case input. Two converters produce
Rust-appropriate names:

```rust
fn to_pascal_case(name: &str) -> String {
    // "custom-lint" -> "CustomLint"
    name.split(|c: char| c == '-' || c == '_')
        .map(|word| capitalize_first(word))
        .collect()
}

fn to_snake_case(name: &str) -> String {
    // "custom-lint" -> "custom_lint"
    name.replace('-', "_").to_lowercase()
}
```

## Gate Scaffold

The gate scaffold generates a `Verify` trait implementation:

```rust
pub struct CustomLintVerify {
    pub threshold: f32,
}

impl Cell for CustomLintVerify {
    fn cell_id(&self) -> &str { "custom_lint_gate" }
    fn cell_name(&self) -> &str { "CustomLintVerify" }
}

#[async_trait]
impl Verify for CustomLintVerify {
    async fn verify(&self, signal: &Signal, _ctx: &Context) -> Verdict {
        if signal.score >= self.threshold {
            Verdict::pass("custom_lint_gate")
        } else {
            Verdict::fail("custom_lint_gate", "score below threshold")
        }
    }
}
```

Includes two tests: `passes_above_threshold` and `rejects_below_threshold`.

## Scorer Scaffold

Generates a `Score` trait implementation with confidence, novelty, and utility
fields:

```rust
impl ScoreTrait for RelevanceScorer {
    fn score(&self, signal: &Signal, _ctx: &Context) -> ScoreValue {
        ScoreValue {
            confidence: signal.score,
            novelty: 0.5,
            utility: 0.5,
        }
    }
}
```

## Router Scaffold

Generates a `Route` trait implementation:

```rust
impl Route for CustomRouter {
    fn route(&self, signal: &Signal, _ctx: &Context) -> RouteDecision {
        RouteDecision::default()
    }
}
```

## Policy Scaffold

Generates a `React` trait implementation for behavioral policies.

## Substrate Scaffold

Generates a `Store` trait implementation for custom storage backends.

## Composer Scaffold

Generates a `Compose` trait implementation for prompt assembly.

## Domain Scaffold

The domain scaffold generates a complete directory structure:

```
payment_processing/
  mod.rs           -- Module root with re-exports
  config.rs        -- Domain-specific configuration
  gates.rs         -- Custom gate definitions
  templates/       -- Prompt template files
    mod.rs
    default.txt
```

## Template Scaffold

Generates a prompt template module with a default template string and
a builder function.

## Event Source Scaffold

Generates an `EventSource` trait implementation for custom event generation.

## File Output

All scaffolds use `write_scaffold_file()` which creates parent directories
as needed:

```rust
fn write_scaffold_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}
```

Each scaffold function returns `Vec<PathBuf>` listing all created files.

## Remaining Work

Six additional scaffold types are planned but not yet implemented. The
SCAFFOLD_TYPES constant will be extended as they are added. The domain
scaffold is the most comprehensive, producing 4+ files for a complete
domain profile.

## Source

- `crates/roko-cli/src/scaffold.rs` -- All scaffold implementations
