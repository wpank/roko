# PAD_02: Audit and migrate remaining direct Claude CLI calls to ModelCallService

## Task
Identify all `Command::new("claude")` call sites that bypass `ModelCallService` and migrate them or document why they're exempt.

## Runner Context
Runner PAD (Stream Parser Consolidation), batch 2 of 3. Depends on PAD_01.

## Problem
DP-2 anti-pattern: "13+ invocation paths, 3 response parsers." `ModelCallService` (model_call_service.rs:56-105) was created to unify all inference calls with fallback chains, cost tracking, and feedback integration. But several call sites still bypass it.

## Current State (VERIFIED)

**Using ModelCallService** (correct):
- `dispatch_v2.rs:54-112`
- `unified.rs:95-101`
- `ServiceFactory::build()` at ~L152

**Bypassing ModelCallService** (needs migration or exemption):
- `run.rs:1842-1843` — has `// TODO(gateway): migrate to ModelCallService`
- `serve/gateway.rs:971` — creates own MCS but not from shared state
- ACP `runner.rs:1752-1785` — raw `Command::new("claude")`
- Episode distillation — reads `ANTHROPIC_API_KEY` from env directly (covered by PK_06)

## Exact Changes

### Step 1: Audit all Command::new("claude") sites

```bash
grep -rn 'Command::new.*claude' crates/ --include='*.rs' | grep -v target/ | grep -v test
```

For each site, classify as:
- **MIGRATE**: Should use ModelCallService
- **EXEMPT**: Has valid reason to bypass (e.g., CLI-specific features not in MCS)
- **COVERED**: Already handled by another prompt (e.g., PK_06, PL_02)

### Step 2: Migrate run.rs TODO

At `run.rs:1842-1843`, replace the direct CLI call:

```rust
// BEFORE:
// TODO(gateway): migrate to ModelCallService
let output = Command::new("claude").arg("--print").arg(prompt)...

// AFTER:
let response = model_call_service.call(ModelCallRequest {
    prompt: prompt.to_string(),
    model: config.model.clone(),
    ..Default::default()
}).await?;
```

### Step 3: Consolidate serve/gateway.rs MCS creation

At `serve/gateway.rs`, use the shared `ModelCallService` from `AppState` instead of creating a new one:

```rust
// BEFORE:
let mcs = ModelCallService::new(...);  // local construction

// AFTER:
let mcs = &app_state.model_call_service;  // shared from AppState
```

### Step 4: Document exemptions

For any site that MUST bypass MCS (e.g., specific CLI flags not supported by MCS), add a comment:

```rust
// EXEMPT from ModelCallService: requires --output-format stream-json with PTY
// which MCS doesn't support. See PAD_02 exemption list.
```

## Write Scope
- `crates/roko-cli/src/run.rs` (migrate TODO)
- `crates/roko-serve/src/routes/gateway.rs` (use shared MCS)
- Any other non-exempt bypass sites found in Step 1

## Read-Only Context
- `crates/roko-agent/src/model_call_service.rs` (ModelCallService API)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- All `Command::new("claude")` sites either migrated to MCS or documented as exempt
- `run.rs` TODO resolved
- Shared MCS used from AppState (not locally constructed)
- Each exemption has a code comment explaining why

## Do NOT
- Force-migrate sites that have valid technical reasons to bypass MCS
- Change the ModelCallService API
- Migrate ACP sites (covered by PL_02)
