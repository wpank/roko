# 77 — CLI UX Consistency (8 Command Name / Error Message Issues)

**Priority**: P1 — significant UX friction for new users following documentation
**Size**: S (1-2 days)
**Crates**: `crates/roko-cli/src/main.rs`, `crates/roko-cli/src/explain.rs`,
`crates/roko-cli/src/commands/show.rs`
**Depends on**: None

---

## Background

Testing ~40 CLI commands against the documentation revealed 8 inconsistencies where command
names, accepted subcommand arguments, or error messages don't match what a user would expect
after reading the help text or CLAUDE.md. The core workflow commands (`roko status`, `roko
doctor`, `roko plan run`, etc.) all work correctly. These issues are concentrated in
secondary commands where naming drifted after the Engram-to-Signal rename, or where
subcommand patterns are inconsistent across command families.

Each issue is a specific, isolated fix in a known file and line range. No cross-cutting
refactors are required.

## Current State

1. **`roko learn router` fails.** The `LearnCmd` enum in
   `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` (lines 1170-1214) has a
   variant named `Route` (line 1179), not `Router`. So `roko learn route` works but `roko
   learn router` fails with an unrecognized-subcommand error. However, the help text at
   `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` line 236 says
   `"Learning: learn (router, experiments, efficiency, tune)"`, telling users to type
   `router`. Both spellings should work.

2. **`roko plan show plans/demo-hello` fails.** The `plan show` command passes the raw
   argument directly to `discover_plan_by_id()`, which compares it against
   `stable_plan_id` (the frontmatter `plan:` field) and `plan_info.base` (the bare
   directory name under `plans/`). When the user includes the `plans/` prefix — which is
   natural because they can see `plans/demo-hello/` in the filesystem — neither comparison
   matches. The bare form `roko plan show demo-hello` works. Affected commands: `plan show`,
   `plan run`, `plan validate` (all of which accept a plan ID argument).

3. **`roko explain signals` fails.** The `TOPICS` array in
   `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/explain.rs` (lines 28+) contains
   a topic with `name: "engram"` (line 139) that was not updated during the Engram-to-Signal
   rename. The topic's title is "Signals (Signal Storage)" but its machine name is still
   `"engram"`. `find_topic()` at line 325 does a literal match on `t.name == lower`, so
   `"signal"` and `"signals"` both return `None`, triggering an exit with "unknown topic".
   The topic is accessible only via `roko explain engram`.

4. **`roko show signals` gives a misleading error.** In
   `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/show.rs`, `ShowTarget::parse()`
   at lines 31-46 matches known subjects (`"overview"`, `"cost"/"costs"`, `"agent"/"agents"`,
   `"knowledge"/"know"/"neuro"`, `"plan"/"plans"`, `"learning"/"learn"/"router"/"routing"`,
   `"history"/"events"/"log"`) and falls through to `ShowTarget::WorkId(subject)` for
   anything else (line 44). When the work-item lookup fails, the error at line 3118-ish of
   `main.rs` says `"session not found: signals"` or a work-item lookup error, implying
   `signals` is a work-item ID rather than an unrecognized subject name. The valid subject
   aliases are not mentioned in the error.

5. **`roko market list` fails.** The `MarketCmd` enum at
   `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` lines 1841-1868 has
   `Browse` but no `List` variant. This is inconsistent with `plan list`, `job list`,
   `agent list`, `trigger list`, `feed list`, and `recipe list`, all of which have a `list`
   subcommand. Users who try `roko market list` by analogy get an unrecognized-subcommand
   error.

6. **`roko config show` displays models in Rust Debug format.** The `config show` command
   prints the models HashMap using `{:?}` formatting, producing unreadable output like
   `{"fast": ModelProfile { provider: "cerebras", slug: "llama-4-scout-17b", ... }}`.
   The `providers` field in the same output uses `toml::to_string_pretty` for readable TOML
   output. The `models` field should do the same.

7. **`roko history list` fails with a misleading error.** The `History` command at
   `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` line 893 takes an
   optional positional `id: Option<String>`. When the user types `roko history list`, clap
   parses `"list"` as the session ID. The lookup at line 3099 calls `load_session(&wd,
   "list")`, returns `None`, and line 3118 prints `"session not found: list"`. There is no
   guidance that `roko history` (with no argument) already lists sessions.

8. **The help text for `learn` is inconsistent with the actual subcommand.** The help text
   at line 236 says `learn (router, experiments, efficiency, tune)`. The actual clap variant
   is `Route` (not `Router`). After fix 1 adds the `router` alias, this will be consistent.
   If fix 1 is not applied, line 236 must be changed from `router` to `route`.

## Implementation Plan

### Fix 1: Add `router` alias to `LearnCmd::Route`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`, find the `Route` variant
in `LearnCmd` at lines 1178-1183:

```rust
/// Show cascade router state.
Route {
    /// Working directory (default: cwd).
    #[arg(long)]
    workdir: Option<PathBuf>,
},
```

Add the clap alias:

```rust
/// Show cascade router state.
#[command(alias = "router")]
Route {
    /// Working directory (default: cwd).
    #[arg(long)]
    workdir: Option<PathBuf>,
},
```

This makes both `roko learn route` and `roko learn router` work. The help text at line 236
already says `router`, so no change needed there once the alias exists.

### Fix 2: Strip `plans/` prefix before plan lookup

Find where the plan ID is passed to `discover_plan_by_id()` in the `show`, `run`, and
`validate` arms of the plan command dispatch. The most likely location is
`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`.

Before each call to `discover_plan_by_id(plan_id)`, add prefix stripping:

```rust
let plan_id = plan_id
    .strip_prefix("plans/")
    .or_else(|| plan_id.strip_prefix("plans\\"))
    .unwrap_or(&plan_id)
    .to_string();
```

Search for all call sites with:
```
rg 'discover_plan_by_id' crates/roko-cli/src/
```
Apply the stripping at each call site that takes a user-provided plan ID argument.

### Fix 3: Add `signal` and `signals` aliases to the `"engram"` topic

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/explain.rs`, the `find_topic()`
function at line 325 does a literal string comparison. Change it to resolve topic aliases
before searching:

```rust
fn resolve_topic_alias(name: &str) -> &str {
    match name {
        "signal" | "signals" | "engrams" => "engram",
        _ => name,
    }
}

pub fn find_topic(name: &str) -> Option<&'static TopicEntry> {
    let lower = name.to_ascii_lowercase();
    let resolved = resolve_topic_alias(&lower);
    let topic = TOPICS.iter().find(|t| t.name == resolved);
    if topic.is_none() {
        request_unknown_topic_exit_if_cli_explain();
    }
    topic
}
```

Alternatively (simpler), just rename the `engram` topic's `name` field to `"signal"` and
add `"engram"` as an alias. Since `TOPICS` is a static slice of `TopicEntry` structs and
`TopicEntry` currently has no `aliases` field, adding an alias to the resolver function is
the lower-change path.

### Fix 4: Improve `roko show` error for unrecognized subjects

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/show.rs`, when
`ShowTarget::parse()` falls through to `ShowTarget::WorkId(subject)` and the subsequent
work-item lookup fails, check whether the input looks like a mistyped subject name and
include the valid subjects in the error.

Find the error return path (the code that handles `ShowTarget::WorkId` and fails to find a
work item). Replace the current error message with:

```rust
anyhow::bail!(
    "unknown subject `{subject}`.\n\
     Valid subjects: overview, costs, agents, knowledge, plans, learning, history.\n\
     To look up a work item by ID, pass its exact ID (e.g. a UUID or plan slug)."
)
```

Or add a heuristic: if the input has no slashes, no UUIDs, and no digits, treat it as a
subject attempt and list valid subjects. Check the `ShowTarget::parse()` match arms to
confirm the complete list of valid subjects (currently: overview/summary, cost/costs,
agent/agents, knowledge/know/neuro, plan/plans, learning/learn/router/routing,
history/events/log).

### Fix 5: Add `list` alias to `MarketCmd::Browse`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`, find the `Browse`
variant in `MarketCmd` at lines 1842-1848:

```rust
/// Browse marketplace artifacts.
Browse {
    #[arg(long)]
    query: Option<String>,
    ...
},
```

Add the alias:

```rust
/// Browse marketplace artifacts.
#[command(alias = "list")]
Browse {
    #[arg(long)]
    query: Option<String>,
    ...
},
```

### Fix 6: Fix `config show` models display format

Find the `models` display in the `config show` handler. Search for:
```
rg 'models.*:?' crates/roko-cli/src/config_cmd.rs
```

Replace the `{:?}` formatting with `toml::to_string_pretty`. If the `models` HashMap does
not derive `Serialize`, add it or use `serde_json::to_string_pretty` as a fallback. The
fix should mirror how `providers` is formatted in the same function (likely using
`toml::to_string_pretty`).

### Fix 7: Improve `roko history list` error message

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`, at the `None =>` arm
after `load_session(&wd, &id)` returns `None` (around line 3117-3119):

```rust
None => {
    eprintln!("session not found: {id}");
    return Ok(EXIT_FAILURE);
}
```

Change to:

```rust
None => {
    if matches!(id.as_str(), "list" | "ls" | "all" | "recent" | "sessions") {
        eprintln!(
            "hint: `roko history` (no argument) lists the 20 most recent sessions.\n\
             See also: `roko show history`"
        );
    } else {
        eprintln!("session not found: {id}");
        eprintln!(
            "hint: run `roko history` (no argument) to see available sessions."
        );
    }
    return Ok(EXIT_FAILURE);
}
```

### Fix 8: Help text consistency (resolved by Fix 1)

After Fix 1 adds the `router` alias, the help text at line 236 (`learn (router,
experiments, efficiency, tune)`) is consistent with the new alias. No separate change needed.
If Fix 1 is not implemented, change line 236 from `router` to `route`.

## Acceptance Criteria

1. `roko learn router` produces the same output as `roko learn route`.
2. `roko plan show plans/demo-hello` shows the demo-hello plan (same output as
   `roko plan show demo-hello`).
3. `roko explain signals` explains signals (same content as `roko explain engram`).
4. `roko show signals` prints a helpful error listing valid subjects instead of a
   work-item-not-found message.
5. `roko market list` produces the same output as `roko market browse`.
6. `roko config show` displays models in readable TOML-like format, not Rust Debug format.
7. `roko history list` prints a hint directing the user to `roko history` (no argument)
   or `roko show history`.
8. `cargo test -p roko-cli` passes.
9. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

## Verification Checklist

- [ ] `roko learn router` exits 0 and prints cascade router state
- [ ] `roko plan show plans/demo-hello` exits 0 and shows the plan (assuming the plan exists)
- [ ] `roko explain signals` exits 0 and prints the Signals topic content
- [ ] `roko explain signal` also exits 0 (same content)
- [ ] `roko show signals` exits nonzero with a message listing valid subjects (not "session
  not found")
- [ ] `roko market list` exits 0 (same as `roko market browse`)
- [ ] `roko config show` output for models section does not contain `ModelProfile {` Rust
  Debug syntax
- [ ] `roko history list` exits nonzero with a hint about `roko history` (no argument)
- [ ] `cargo test -p roko-cli` passes

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` | Add `#[command(alias = "router")]` to `LearnCmd::Route`; add `#[command(alias = "list")]` to `MarketCmd::Browse`; improve `history` error message at the `None =>` arm |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/explain.rs` | Add `resolve_topic_alias()` function; update `find_topic()` to call it before searching `TOPICS` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/show.rs` | Improve error message when `ShowTarget::WorkId` lookup fails with no matching work item |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs` | Strip `plans/` prefix before passing plan ID to `discover_plan_by_id()` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/config_cmd.rs` | Replace `{:?}` formatting for models HashMap with `toml::to_string_pretty` |

## Not in Scope

- Broader CLI verb consolidation (tracked in backlog 65)
- CLI output formatting overhaul (CliReporter, spinners, JSON output) (tracked in backlog 35)
- Adding new explain topics beyond the signal alias
- Changing the `History` command to a subcommand-based enum (architectural change)
- Fixing unrecognized commands in other command families
