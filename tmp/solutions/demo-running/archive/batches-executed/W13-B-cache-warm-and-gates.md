# W13-B: Warm Cargo Cache and Dynamic Gate Channel Buffer

**Wave**: 13 -- Speed & Reliability
**IMPROVEMENTS ref**: 2.2 + 2.4
**Priority**: P1 -- saves 30-120s on first gate
**Effort**: 2-3 hours
**Files to modify**: 4 files
**Dependencies**: None

## Problem

Two related performance issues in the plan run event loop:

1. **Cold cargo cache**: The first compile gate after `scaffold_missing_crates` pays a
   full `cargo check --workspace` cost (30-120s). Subsequent gates are incremental and
   fast (2-5s). A warm-up pass before the main loop would make ALL gates fast.

2. **Hardcoded gate channel buffer**: The gate channel buffer is hardcoded to 16
   (line 261). With 4 concurrent tasks x 7 rungs = 28 possible in-flight gate
   completions, the buffer can overflow and cause backpressure.

## Root Cause

1. No warm-up step exists in the event loop init phase.
2. `mpsc::channel::<GateCompletion>(16)` is a fixed constant unrelated to actual concurrency.

## Exact Code to Change

### File 1: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs`

#### Change 1: Add `warm_cache` field to `RunConfig` struct

**Find this code** (lines 1291-1295):
```rust
    /// When true, print real-time agent and task lifecycle events to
    /// stderr instead of showing a spinner. Enabled in non-quiet,
    /// non-json, non-approval CLI mode.
    pub stream_to_stderr: bool,
}
```

**Replace with:**
```rust
    /// When true, print real-time agent and task lifecycle events to
    /// stderr instead of showing a spinner. Enabled in non-quiet,
    /// non-json, non-approval CLI mode.
    pub stream_to_stderr: bool,
    /// When true, run `cargo check --workspace` before the main event loop
    /// to warm the incremental cache. Makes subsequent compile gates fast.
    /// Default: true.
    pub warm_cache: bool,
}
```

#### Change 2: Set `warm_cache: true` in `from_roko_config` (line 1373)

**Find this code** (lines 1371-1381):
```rust
            stream_to_stderr: false,
            // The runner constructs feedback / projection facades at run
            // start (`event_loop::run`) so they share their lifetime
            // with the run id. `None` here is the safe default for
            // callers that build a `RunConfig` directly without going
            // through the full runner setup (tests, integration shims).
            feedback_facade: None,
            projection: None,
        }
    }
}
```

**Replace with:**
```rust
            stream_to_stderr: false,
            warm_cache: true,
            // The runner constructs feedback / projection facades at run
            // start (`event_loop::run`) so they share their lifetime
            // with the run id. `None` here is the safe default for
            // callers that build a `RunConfig` directly without going
            // through the full runner setup (tests, integration shims).
            feedback_facade: None,
            projection: None,
        }
    }
}
```

#### Change 3: Set `warm_cache: true` in `Default` impl (line 1414)

**Find this code** (lines 1413-1416):
```rust
            stream_to_stderr: false,
        }
    }
}
```

**Replace with:**
```rust
            stream_to_stderr: false,
            warm_cache: true,
        }
    }
}
```

#### Change 3b: Add `warm_cache` to the manual `Debug` impl (line 1450)

**Find this code** (lines 1450-1451):
```rust
            .field("stream_to_stderr", &self.stream_to_stderr)
            .finish()
```

**Replace with:**
```rust
            .field("stream_to_stderr", &self.stream_to_stderr)
            .field("warm_cache", &self.warm_cache)
            .finish()
```

### File 2: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`

#### Change 4: Add `warm_cache: true` to plan.rs RunConfig constructor (line 484)

**Find this code** (lines 483-485):
```rust
                projection: Some(projection),
                stream_to_stderr: !approval && !cli.quiet && !cli.json,
            };
```

**Replace with:**
```rust
                projection: Some(projection),
                stream_to_stderr: !approval && !cli.quiet && !cli.json,
                warm_cache: true,
            };
```

### File 3: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/serve_runtime.rs`

#### Change 5: Add `warm_cache: true` to serve_runtime.rs RunConfig constructor (line 572)

**Find this code** (lines 571-573):
```rust
        projection: None,
        stream_to_stderr: false,
    }
```

**Replace with:**
```rust
        projection: None,
        stream_to_stderr: false,
        warm_cache: true,
    }
```

### File 4: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`

#### Change 6: Dynamic gate channel buffer + cargo cache warm-up

**Find this code** (lines 259-264):
```rust
    // Channels.
    let (agent_tx, mut agent_rx) = mpsc::channel::<AgentEvent>(256);
    let (gate_tx, mut gate_rx) = mpsc::channel::<GateCompletion>(16);

    // Seed playbooks if the store is empty (bootstrap chicken-and-egg).
    seed_playbooks_if_empty(&config.workdir).await;
```

**Replace with:**
```rust
    // Channels.
    let (agent_tx, mut agent_rx) = mpsc::channel::<AgentEvent>(256);
    // Dynamic gate channel buffer: max_concurrent_tasks * 7 rungs, clamped to [32, 256].
    let gate_buffer = (config.max_concurrent_tasks * 7).max(32).min(256);
    let (gate_tx, mut gate_rx) = mpsc::channel::<GateCompletion>(gate_buffer);

    // -- Warm cargo cache -------------------------------------------------------
    // Run `cargo check --workspace` once before the main loop so that
    // subsequent per-task compile gates are incremental (2-5s vs 30-120s).
    if config.warm_cache {
        if config.stream_to_stderr {
            eprintln!("[plan-run] Warming cargo cache...");
        }
        let warm_start = std::time::Instant::now();
        let warm_result = tokio::process::Command::new("cargo")
            .args(["check", "--workspace", "--message-format=short"])
            .current_dir(&config.workdir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;
        let warm_ms = warm_start.elapsed().as_millis() as u64;
        match warm_result {
            Ok(status) if status.success() => {
                info!(warm_ms, "cargo cache warmed successfully");
                if config.stream_to_stderr {
                    eprintln!("[plan-run] Cargo cache warm: {warm_ms}ms");
                }
            }
            Ok(status) => {
                warn!(
                    warm_ms,
                    exit_code = status.code().unwrap_or(-1),
                    "cargo cache warm failed (non-fatal)"
                );
            }
            Err(e) => {
                warn!(warm_ms, error = %e, "cargo cache warm failed (non-fatal)");
            }
        }
    }

    // Seed playbooks if the store is empty (bootstrap chicken-and-egg).
    seed_playbooks_if_empty(&config.workdir).await;
```

Note: `std::process::Stdio` is used inline (qualified path) so no new import is needed.
`std::time::Instant` is already imported at line 8. `info` and `warn` are imported from
`tracing` at line 23.

## Verification

```bash
# Check that the warm_cache field compiles
cargo check -p roko-cli 2>&1 | head -20

# Integration test: run a plan and observe timing
cargo run -p roko-cli -- plan run plans/ 2>&1 | grep -E "warm|gate"
# Should see "Warming cargo cache..." followed by fast gate times

# Verify channel buffer is dynamic
grep -n "gate_buffer" crates/roko-cli/src/runner/event_loop.rs
```

## Agent Prompt

```
You are implementing W13-B: Warm Cargo Cache and Dynamic Gate Channel Buffer. This saves
30-120s on the first compile gate and prevents channel backpressure.

## Changes to make (4 files)

### 1. types.rs -- add `warm_cache` field to RunConfig

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs`:

a) Add `pub warm_cache: bool` field after `pub stream_to_stderr: bool` in the
   `RunConfig` struct (line 1294).

b) Set `warm_cache: true` in `from_roko_config` -- insert after the
   `stream_to_stderr: false,` line (line 1373, just before the comment block).

c) Set `warm_cache: true` in `impl Default for RunConfig` -- insert after the
   `stream_to_stderr: false,` line (line 1414).

### 2. commands/plan.rs -- add warm_cache to RunConfig constructor

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`, find the
`RunConfig {` block starting at line 448. After the `stream_to_stderr` field (line 484),
add `warm_cache: true,`.

### 3. serve_runtime.rs -- add warm_cache to RunConfig constructor

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/serve_runtime.rs`, find the
`RunConfig {` block starting at line 545. After the `stream_to_stderr: false,` field
(line 572), add `warm_cache: true,`.

### 4. event_loop.rs -- dynamic gate buffer + warm-up

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`, find
lines 259-264 (the channel creation + seed_playbooks block).

Replace `mpsc::channel::<GateCompletion>(16)` with dynamic sizing:
```rust
let gate_buffer = (config.max_concurrent_tasks * 7).max(32).min(256);
let (gate_tx, mut gate_rx) = mpsc::channel::<GateCompletion>(gate_buffer);
```

Then insert the cargo cache warm-up block between the channel creation and
`seed_playbooks_if_empty`. Use `std::process::Stdio::null()` (qualified, no import
needed) for stdout/stderr suppression. Log timing with `tracing::info!`.

Do NOT run cargo build/test/clippy/fmt -- compilation is deferred.
```

## Commit

This batch is committed with all Wave 13 batches together. Do not commit individually.

## Checklist

- [ ] `warm_cache: bool` added to `RunConfig` in types.rs
- [ ] `warm_cache: true` set in `from_roko_config` method
- [ ] `warm_cache: true` set in `Default` impl for RunConfig
- [ ] `warm_cache: true` added to RunConfig in commands/plan.rs
- [ ] `warm_cache: true` added to RunConfig in serve_runtime.rs
- [ ] Gate channel buffer changed from hardcoded 16 to dynamic `(max_concurrent_tasks * 7).max(32).min(256)`
- [ ] Cargo cache warm-up block added before `seed_playbooks_if_empty` in event_loop.rs
- [ ] Warm-up gated on `config.warm_cache`
- [ ] Warm-up logs timing via `tracing::info!` and handles failure non-fatally
- [ ] Pre-commit checks pass

## Audit Status

Audited: 2026-05-05. PASS no changes needed
