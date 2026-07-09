# 17 — Safety & Security Tasks

> Harden roko from permissive-by-default to defense-in-depth. Remove the
> unconditional `dangerously_skip_permissions: true`, wire built-but-never-called
> safety infrastructure into the live dispatch path, and add compliance tooling
> for the EU AI Act Article 50 deadline (August 2, 2026).

---

## Overview

Roko has 18 safety submodules (`crates/roko-agent/src/safety/`), 8 bundled
YAML contracts, a full path-canonicalization policy, a bash deny-pattern engine,
a network allowlist, a secret scrubber, a rate limiter, and a result filter.
**None of these are called from the live dispatch path.**

The system defaults to `dangerously_skip_permissions: true` in three places:
- `ClaudeCliAgent::new()` at line 128 of `claude_cli_agent.rs`
- `RunConfig::default()` at line 1337 of `runner/types.rs`
- `RunConfig::bench_defaults()` at line 1382 of `runner/types.rs`

The `AgentContract` system has 8 well-defined contracts (implementer, reviewer,
researcher, strategist, architect, auditor, scribe, auto-fixer) with invariants,
governance rules, and recovery actions. The `check_pre_execution()` method works
correctly in unit tests. But `AgentContract` is never imported or called from any
file in `crates/roko-cli/`.

The HTTP control plane (`roko serve`) binds `0.0.0.0:6677` with auth disabled
by default. The `POST /runs/{id}/share` route is inside the auth layer (fixed
since audit-v3), but `GET /api/runs/{id}` and `GET /api/shared/{token}` are
public by design (share receipt readers). Cloud deploy commands (`roko deploy
railway`) do not auto-provision auth.

**The gap is not "build safety." The gap is "wire safety."**

---

## Anti-Patterns to Remove

| ID | Anti-Pattern | Where | Removal |
|---|---|---|---|
| AP-S1 | `dangerously_skip_permissions: true` as default | `claude_cli_agent.rs:128`, `runner/types.rs:1337,1382` | Change default to `false`; require `--skip-permissions` CLI flag to opt in |
| AP-S2 | 18 safety submodules never called from dispatch | `safety/*.rs` all have `check()` methods; `dispatch_v2.rs` and `runner/` never call them | Wire policy chain into dispatch |
| AP-S3 | `AgentContract` never loaded at runtime | `contract.rs` has `load_for_role()` with 8 YAMLs; not imported in CLI crate | Load contract per role, enforce `check_pre_execution()` |
| AP-S4 | `build_settings_json()` blocks only 7 bash patterns | `claude_cli_agent.rs:34-80` | Extend with all Critical/High bash patterns from `BashPolicy` |
| AP-S5 | `ResultFilter` never called from tool loop | `result_filter.rs` has `sanitize()` working in tests | Wire into dispatch output path |
| AP-S6 | `NetworkPolicy` never enforced at dispatch | `network.rs` has full SSRF/private-IP blocking | Wire into `build_settings_json()` and dispatch |
| AP-S7 | `RateLimiter` never instantiated at runtime | `rate_limit.rs` has complete sliding-window impl | Instantiate in runner, wire into dispatch |
| AP-S8 | `ScrubPolicy` not applied to prompts or episodes | `scrub.rs` has 10+ secret patterns | Wire into prompt assembly and episode logging |
| AP-S9 | `chat_session.rs:1053` hardcodes `--dangerously-skip-permissions` | `chat_session.rs` | Make conditional on config |
| AP-S10 | Cloud deploy does not auto-provision auth | deploy commands | Generate API key on deploy, set `api_auth.enabled = true` |

---

## PHASE 1: Contract Enforcement (Kill the Permissive Default)

### Task 17.1: Change `dangerously_skip_permissions` Default to `false`

**Priority**: P0 (critical — system currently runs permissive)

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/claude_cli_agent.rs` (line 128)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs` (lines 1337, 1382)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_session.rs` (line 1053)

**What**: The `ClaudeCliAgent::new()` constructor sets `dangerously_skip_permissions: true`
unconditionally. `RunConfig::default()` does the same. `chat_session.rs` hardcodes
the flag. Change all defaults to `false`. Add a `--skip-permissions` CLI flag that
explicitly opts in with a `tracing::warn!` when used.

**Steps**:
1. In `claude_cli_agent.rs` line 128, change `dangerously_skip_permissions: true` to `false`
2. In `runner/types.rs` lines 1337 and 1382, change both `dangerously_skip_permissions: true` to `false`
3. In `chat_session.rs` line 1053, make the `--dangerously-skip-permissions` arg conditional
   on a `skip_permissions` config value, not hardcoded
4. Add `--skip-permissions` flag to the `plan run` clap args in `commands/plan.rs`
5. When `--skip-permissions` is passed, set `RunConfig.dangerously_skip_permissions = true`
   and emit `tracing::warn!("running with --skip-permissions: all agent permission checks bypassed")`
6. Update existing tests that depend on the permissive default: search for
   `with_dangerously_skip_permissions(false)` assertions and adjust

**Acceptance criteria**:
- `cargo run -p roko-cli -- plan run plans/` does NOT pass `--dangerously-skip-permissions` to the Claude CLI
- `cargo run -p roko-cli -- plan run plans/ --skip-permissions` passes it and logs a warning
- `cargo run -p roko-cli -- chat` does NOT hardcode the flag
- Unit tests that explicitly test the flag continue to pass
- `RunConfig::default()` has `dangerously_skip_permissions: false`

**Existing code to reference**:
- `ClaudeCliAgent::with_dangerously_skip_permissions()` at `claude_cli_agent.rs:245` (builder method)
- `dispatch_v2.rs:320-322` and `357-358` (where flag is consumed to build CLI args)

---

### Task 17.2: Wire `AgentContract` Into Runner v2 Dispatch

**Priority**: P0 (critical — contracts exist but are never enforced)

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_v2.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`

**Files to read** (existing, do not modify):
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/contract.rs` (full contract system)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/contracts/*.yaml` (8 bundled contracts)

**What**: Load the role-appropriate `AgentContract` at agent spawn time.
Apply contract constraints to the Claude CLI invocation: if the contract has
`allowed_tools`, pass them as `--allowedTools`; if the contract has
`ForbiddenTools`, add them to the `build_settings_json()` denylist.

**Steps**:
1. Add `roko-agent` safety dependency if not already available in the CLI crate
2. When building an `AgentDispatchRequest` in the runner, determine the role
   from `TaskDef.role` (already available in the task TOML)
3. Call `AgentContract::load_for_role_with_mode(role, ContractLoadMode::RestrictedFallback)`
4. If contract has `allowed_tools: Some(tools)`, set `request.tools = tools.join(",")`
5. If contract has `ForbiddenTools` governance rules, merge them into the
   settings JSON hooks as additional `PreToolUse` blockers
6. If contract has `MaxTokensPerTurn(n)`, cap `request.max_turns` accordingly
7. Log the loaded contract at `tracing::info!` level: role, invariant count,
   governance rule count, allowed_tools count
8. Store the contract in the dispatch context for post-execution auditing

**Acceptance criteria**:
- A task with `role = "reviewer"` spawns an agent without `edit_file` in its tool
  allowlist (reviewer contract forbids edit tools)
- A task with `role = "auditor"` spawns an agent without write tools (auditor
  contract is read-only, has `NoNetworkAccess`)
- A task with `role = "implementer"` gets the full tool set minus network tools
  (implementer contract has `ForbiddenTools: ["network", "fetch"]`)
- Contract load failures in `RestrictedFallback` mode log a warning but do not
  crash the runner

**Existing code to reference**:
- `AgentContract::load_for_role_with_mode()` at `contract.rs:148`
- `AgentContract::permits_tool()` at `contract.rs:177`
- `AgentContract::check_pre_execution()` at `contract.rs:196`
- `ContractLoadMode::RestrictedFallback` at `contract.rs:42` (deny-all fallback)

---

### Task 17.3: Wire `AgentContract` Into WorkflowEngine (V2 `roko run` Path)

**Priority**: P1 (high — `roko run` is the other main entry point)

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/claude_cli.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/mod.rs`

**What**: The `WorkflowEngine` dispatches agents via `EffectDriver` and
`ModelCallService`. This path must also load and enforce contracts. The provider
options struct at `provider/mod.rs:534` already has a
`dangerously_skip_permissions: bool` field. Add contract enforcement alongside it.

**Steps**:
1. In `EffectDriver`, when spawning an agent for a role, load the contract
2. Pass the contract's tool constraints through to the provider options
3. In `provider/claude_cli.rs:54`, the Claude CLI provider already reads
   `options.dangerously_skip_permissions`. Add parallel logic to read
   contract tool restrictions from the options
4. When the contract forbids a tool category, reflect it in the CLI args
5. Record contract enforcement in the feedback event

**Acceptance criteria**:
- `roko run "review this code"` dispatches a reviewer with read-only tools
- `roko run "fix this bug"` dispatches an implementer with edit tools allowed
  but network tools forbidden
- Contract violations are logged (not silently swallowed)

---

### Task 17.4: Generate Default Contracts During `roko init`

**Priority**: P1

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/init.rs`

**Files to read**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/contracts/*.yaml` (8 files)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/contract.rs` (loading logic)

**What**: When `roko init` creates `.roko/`, also create `.roko/contracts/` with
copies of the 8 bundled contract YAML files. Add a `[safety]` section to the
generated `roko.toml`. Update `AgentContract::load_for_role()` to check the
project contract dir first, then fall back to the bundled asset.

**Steps**:
1. In `init.rs`, after creating the `.roko/` layout, create `.roko/contracts/`
2. Embed the 8 YAML files from `crates/roko-agent/src/safety/contracts/` using
   `include_str!` or read from the crate's installed assets
3. Write each to `.roko/contracts/{role}.yaml`
4. Add to the generated `roko.toml`:
   ```toml
   [safety]
   contract_dir = ".roko/contracts"
   skip_permissions = false
   ```
5. Update `AgentContract::load_for_role()` in `contract.rs`:
   - Accept an optional `project_dir: Option<&Path>` parameter
   - Check `{project_dir}/.roko/contracts/{role}.yaml` first
   - Fall back to the bundled asset if not found
6. Or: add a new method `AgentContract::load_for_role_from_project(role, project_dir)`
   that tries the project path first

**Acceptance criteria**:
- `roko init` creates `.roko/contracts/implementer.yaml` (and 7 others)
- Editing `.roko/contracts/reviewer.yaml` to add `ForbiddenTools: ["bash"]` is
  respected at dispatch time (requires Task 17.2 to be wired)
- If `.roko/contracts/` is missing (old projects), bundled contracts are used

---

## PHASE 2: Safety Hook Chain (Wire the Built Policies)

### Task 17.5: Extend `build_settings_json()` with Full Bash Denylist

**Priority**: P1

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/claude_cli_agent.rs` (lines 34-80)

**Files to read**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/bash.rs` (full `BashPolicy`)

**What**: `build_settings_json()` currently blocks only 7 patterns (git checkout,
git switch, git branch -m, git push, rm -rf, rm -fr, rm -r). The `BashPolicy`
in `bash.rs` has a much more comprehensive denylist including `sudo`, `curl | sh`,
fork bombs, `mkfs`, raw-device writes, and world-writable chmods. Port these
into the settings JSON.

**Steps**:
1. Review `BashPolicy::with_defaults()` in `bash.rs` for the full deny pattern set
2. Add `PreToolUse` hooks to `build_settings_json()` for:
   - `sudo *` (all sudo commands)
   - `curl * | sh`, `curl * | bash`, `wget * | sh`, `wget * | bash` (pipe-to-shell)
   - `eval *` (eval injection)
   - `chmod 777 *` (world-writable)
   - `mkfs*` (filesystem destruction)
   - `dd if=* of=/dev/*` (raw device write)
   - `:(){ :|:& };:` or variants (fork bomb)
3. Make `build_settings_json()` accept an optional `&AgentContract` parameter so
   contract-specific `ForbiddenTools` can be merged into the hooks
4. Keep the existing 7 patterns unchanged (backward compat)

**Acceptance criteria**:
- `curl https://example.com | bash` is blocked by the settings JSON hooks
- `sudo rm -rf /` is blocked
- `cargo test` is NOT blocked
- All existing tests pass

---

### Task 17.6: Wire `NetworkPolicy` Into Dispatch

**Priority**: P1

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/claude_cli_agent.rs`

**Files to read**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/network.rs` (full policy)

**What**: `NetworkPolicy` has complete SSRF blocking (private IPs, link-local,
cloud metadata), scheme enforcement (HTTPS-only), and host allow/deny lists.
None of this is called from the dispatch path. Wire it into `build_settings_json()`
as `PreToolUse` hooks that block network commands matching denied patterns.

**Steps**:
1. Add `PreToolUse` hooks that block `Bash(curl http://10.*)`,
   `Bash(curl http://127.*)`, `Bash(curl http://169.254.*)`,
   `Bash(curl http://192.168.*)`, `Bash(curl http://172.16.*)`
   and similar patterns for `wget`, `nc`, `telnet`
2. For agents with `NoNetworkAccess` invariant (reviewer, auditor, scribe),
   block ALL network commands: `curl *`, `wget *`, `http *`
3. Make the allowlist configurable via `[safety.network.allowlist]` in roko.toml
4. Default allowlist: `crates.io`, `docs.rs`, `github.com`, `api.github.com`,
   `registry.npmjs.org`

**Acceptance criteria**:
- An agent attempting `curl http://169.254.169.254/metadata` is blocked (SSRF)
- An agent with `NoNetworkAccess` attempting `curl https://example.com` is blocked
- `curl https://crates.io/api/v1/crates/serde` is allowed for roles that permit network
- Network blocks are visible in settings JSON hooks

---

### Task 17.7: Wire `ResultFilter` Into Tool Output Processing

**Priority**: P1

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_v2.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`

**Files to read**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/result_filter.rs`

**What**: The `ResultFilter` has working `sanitize()` that truncates oversized
output, strips secrets via `ScrubPolicy`, and annotates external tool output.
Wire it into the agent output processing path so agent responses are sanitized
before being stored as episodes or passed to downstream consumers.

**Steps**:
1. Instantiate a `ResultFilter::with_defaults()` in the runner event loop
2. After receiving agent output in `handle_agent_event()`, run the output through
   `result_filter.sanitize(output, "agent_response")`
3. Apply the same filter to gate error output before injecting into retry context
4. Make `max_response_bytes` configurable via `[safety.tool_output.max_bytes]`
   in roko.toml (default: 100KB per the existing constant)
5. Log sanitization events (what was stripped) at `tracing::debug!` level

**Acceptance criteria**:
- Agent output containing `sk-ant-api01-...` has the key redacted before episode storage
- Agent output exceeding 100KB is truncated with `[OUTPUT TRUNCATED]` marker
- Tool output from `bash` commands is annotated as external-source data
- Existing tests pass; new test confirms sanitization is applied

---

### Task 17.8: Wire `RateLimiter` Into Dispatch

**Priority**: P2

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_v2.rs`

**Files to read**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/rate_limit.rs`

**What**: The `RateLimiter` has a complete sliding-window implementation with
`check_and_record()` method. It is never instantiated at runtime. Wire it into
the dispatch path to prevent runaway agents.

**Steps**:
1. Instantiate `RateLimiter::new(RateLimitPolicy { max_calls_per_window: 120, window_duration: 60s })`
   in the runner event loop initialization
2. Before each agent dispatch, call `rate_limiter.check_and_record(&key)`
   with `key = RateLimitKey { role, tool: "agent_call" }`
3. If rate limited, delay (do not drop) the dispatch and log a warning
4. Make limits configurable via `[safety.rate_limits]` in roko.toml:
   ```toml
   [safety.rate_limits]
   per_tool = 120        # max calls per tool per minute
   per_role = 60         # max agent calls per role per minute
   global = 300          # max total agent calls per minute
   ```
5. Rate limit violations should appear in efficiency events

**Acceptance criteria**:
- An agent making > 60 calls per minute is throttled (delayed, not terminated)
- Rate limit config is loaded from roko.toml
- `roko learn efficiency` shows rate-limited events
- Default limits are reasonable for normal execution (no false positives)

---

### Task 17.9: Wire `ScrubPolicy` Into Prompt Assembly and Episode Logging

**Priority**: P1

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_v2.rs` (or prompt_builder.rs)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`

**Files to read**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/scrub.rs`

**What**: The `ScrubPolicy` has 10+ secret patterns (Anthropic keys, OpenAI keys,
AWS keys, GitHub tokens, JWTs, private keys, .env values). It is used in the
HTTP share path (`shared_runs.rs` calls `LogScrubber`) but NOT in:
- Prompt assembly (secrets in repo files could leak into prompts)
- Episode logging (agent output containing secrets persists to `episodes.jsonl`)
- CLI share/gist path

**Steps**:
1. Import `scrub_secrets()` from `roko_agent::safety::scrub`
2. In prompt assembly, scan the assembled system prompt for secret patterns
   before sending to the LLM. If found, redact and log a warning
3. In episode recording, scan episode content before writing to
   `.roko/episodes.jsonl`. Redact any detected secrets
4. In the CLI `--share` gist path, apply `scrub_secrets()` before upload
   (this fixes the LOW security finding from the audit)
5. Add custom scrub patterns via `[safety.scrub.patterns]` in roko.toml

**Acceptance criteria**:
- A prompt containing `ANTHROPIC_API_KEY=sk-ant-...` has the key redacted
- Episode logs in `.roko/episodes.jsonl` never contain raw API keys
- `roko run --share` with secrets in output produces a Gist with `[REDACTED]`
- Custom patterns from config are applied alongside defaults

---

## PHASE 3: Trust Domain Separation

### Task 17.10: Immutable Gate Configs

**Priority**: P1

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/gate_dispatch.rs`

**What**: Gate definitions should be loaded once at pipeline start and frozen.
Currently, gates are read from `roko.toml` which agents can modify mid-run.
An agent that writes to `roko.toml` during execution could change gate behavior.

**Steps**:
1. At the start of `run()` in `event_loop.rs`, load all gate configs from
   `roko.toml` into an `Arc<Vec<GateConfig>>` (immutable after construction)
2. Hash the gate config files at start using BLAKE3
3. Pass the frozen config to `gate_dispatch` instead of re-reading from disk
4. Before each gate execution, verify the config file hash has not changed
5. If changed, log a `tracing::error!` and use the frozen config (do NOT
   pick up the modified version)

**Acceptance criteria**:
- Modifying `roko.toml` `[[gates]]` section mid-run does NOT affect running gates
- An agent that writes to `roko.toml` during execution does not change gate behavior
- Gate config integrity is verified before each gate run
- Hash mismatch produces a clear error message

---

### Task 17.11: Worktree Path Isolation Enforcement

**Priority**: P2

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/claude_cli_agent.rs`

**Files to read**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/path.rs` (full policy)

**What**: `PathPolicy` in `path.rs` has complete worktree escape detection
including symlink traversal blocking. Wire it into the Claude CLI settings
hooks to prevent agents from accessing files outside their assigned worktree.

**Steps**:
1. When spawning a Claude CLI agent in a worktree, set `current_dir` to the
   worktree root (this may already happen)
2. Add `PreToolUse` hooks to `build_settings_json()` that block:
   - `Bash(cd /*)` targeting paths outside the worktree
   - `Bash(cat /etc/*)`, `Bash(cat ~/.*)` and similar escape patterns
   - `Bash(ln -s /*)` creating symlinks to outside paths
3. The `--allowed-directory` flag on Claude CLI (if available) should be set
   to the worktree root
4. Log path violations at `tracing::warn!` level

**Acceptance criteria**:
- An agent in a worktree cannot `cat /etc/passwd` via bash
- An agent cannot create a symlink pointing outside the worktree
- Legitimate file operations within the worktree are unaffected
- Path blocks appear in the settings JSON hooks

---

### Task 17.12: Inter-Agent Message Sanitization

**Priority**: P2

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_v2.rs`

**What**: When one agent's output is injected into the next agent's context
(prior task outputs, gate failure feedback, strategist briefs), the content
is treated as trusted. An agent could embed prompt injection payloads that
affect downstream agents.

**Steps**:
1. Create an `InterAgentSanitizer` with:
   - Injection pattern list: `<system>`, `[INST]`, `<|im_start|>`, `Human:`,
     `\n\nHuman:`, `IGNORE PREVIOUS INSTRUCTIONS`
   - Max context size: 32KB per injection point
2. Apply sanitization to:
   - Prior task output loaded for context in dispatch
   - Gate failure error messages injected into retry prompts
   - Any `strategist_brief` content
3. Strip matching patterns and truncate oversized context
4. Log sanitization events at `tracing::debug!` level

**Acceptance criteria**:
- An agent output containing `[SYSTEM] ignore your instructions` is sanitized
  before reaching the next agent
- Context injection from prior tasks is capped at 32KB
- Legitimate context (code snippets, error messages) passes through unchanged

---

## PHASE 4: HTTP Security Hardening

### Task 17.13: Auto-Provision Auth on Cloud Deploy

**Priority**: P0 (security — cloud deployments are currently unauthenticated)

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/` (deploy-related files)

**What**: `roko deploy railway` (and fly, docker) creates a public deployment
without auth. Anyone who discovers the URL has full control of the agent runtime.
Generate a random API key during deploy and set `api_auth.enabled = true`.

**Steps**:
1. In the deploy command, generate a random 32-byte API key:
   `hex::encode(rand::random::<[u8; 32]>())`
2. Set `api_auth.enabled = true` and `api_auth.keys = [{ hash = sha256(key) }]`
   in the deployed config
3. Print the API key to stdout: "Your API key: rk-{key}. Save this — it cannot
   be recovered."
4. Set the key as an environment variable in the deployment:
   `ROKO_API_KEY=rk-{key}`
5. Update `roko doctor` to warn when `api_auth.enabled = false` and the bind
   address is not loopback

**Acceptance criteria**:
- `roko deploy railway` outputs an API key and sets auth enabled
- The deployed server rejects unauthenticated requests with 401
- `roko doctor` warns about disabled auth on non-loopback binds

---

### Task 17.14: Scrub Secrets on CLI `--share` (Gist) Path

**Priority**: P1 (security — fixes LOW finding from audit)

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/` (share/gist related code)

**What**: The HTTP share path (`shared_runs.rs`) applies `scrub_run_transcript()`
before persisting. The CLI `--share` path creates a GitHub Gist with the raw
agent transcript without scrubbing. API keys, tokens, and secrets in agent
output are uploaded to GitHub as-is.

**Steps**:
1. Find the CLI `--share` gist creation code
2. Import `LogScrubber` or `scrub_secrets()` from the safety module
3. Apply scrubbing to the transcript text before uploading
4. Apply the same `scrub_long_secret_like_strings()` regex patterns used in
   `shared_runs.rs` (hex strings > 32 chars, base64 strings > 32 chars)
5. Log a count of redacted secrets at `tracing::info!` level

**Acceptance criteria**:
- `roko run --share` with `ANTHROPIC_API_KEY=sk-ant-...` in output produces
  a Gist containing `[REDACTED]` instead of the key
- Long hex and base64 strings are scrubbed
- The scrubbing matches the HTTP path's behavior

---

## PHASE 5: Audit Logging

### Task 17.15: Unified Security Audit Trail

**Priority**: P1

**Files to create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/audit.rs`

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/lib.rs` (add `pub mod audit`)

**What**: Create a single append-only JSONL audit log for all security-relevant
events. This is the foundation for compliance and for Tasks 17.1-17.14 to
report their enforcement actions.

**Steps**:
1. Define `SecurityAuditEvent`:
   ```rust
   pub struct SecurityAuditEvent {
       pub timestamp: DateTime<Utc>,
       pub event_type: AuditEventType,
       pub agent_id: String,
       pub task_id: Option<String>,
       pub detail: serde_json::Value,
       pub severity: AuditSeverity,
   }

   pub enum AuditEventType {
       ContractLoaded, ContractViolation,
       PermissionGranted, PermissionDenied,
       ToolCallBlocked, ToolOutputSanitized,
       RateLimitHit, NetworkBlocked,
       GateConfigVerified, GateConfigTampered,
       SecretRedacted, PathViolation,
   }

   pub enum AuditSeverity { Info, Warning, Violation, Critical }
   ```
2. Create `SecurityAuditLogger` that writes to `.roko/audit/security.jsonl`
   (append-only, never truncated)
3. Add file rotation when log exceeds 100MB (keep 10 rotated files)
4. Expose as `Arc<SecurityAuditLogger>` for use by all safety checks
5. Add `roko audit show` CLI command that displays recent security events
   with severity filtering

**Acceptance criteria**:
- Every contract enforcement from Task 17.2 produces a `SecurityAuditEvent`
- Every permission decision (grant/deny) is logged
- Log files are append-only (no overwrite, no truncation)
- `roko audit show` displays recent events with `--severity` filter

---

### Task 17.16: Audit Log Retention Policy

**Priority**: P2

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/audit.rs` (from Task 17.15)

**What**: Implement configurable log retention for EU AI Act Article 26(6)
compliance (at least 6 months retention for automatic event logs).

**Steps**:
1. Add `[safety.audit]` config section:
   ```toml
   [safety.audit]
   retention_days = 180
   max_file_size_mb = 100
   rotation_count = 10
   ```
2. On `roko serve` startup, check audit log age and warn if retention < 180 days
3. Add `roko audit gc` command that only removes logs older than `retention_days`
4. `roko doctor` warns if audit logging is disabled or retention < 180 days

**Acceptance criteria**:
- `roko audit gc` with `retention_days = 180` preserves logs from last 6 months
- `roko doctor` warns when `retention_days < 180`
- Retention config is loaded from roko.toml

---

## PHASE 6: EU AI Act Compliance

### Task 17.17: Article 50 Transparency Disclosure

**Priority**: P2 (deadline: August 2, 2026)

**Files to create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/compliance.rs`

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs` (add route)

**What**: Article 50(1) requires informing users they are interacting with AI.
Article 50(2) requires marking AI-generated outputs. Implement both.

**Steps**:
1. Add `[compliance]` config section to roko.toml:
   ```toml
   [compliance]
   eu_ai_act = false
   transparency_mode = "auto"
   output_marking = false
   ```
2. When `eu_ai_act = true`:
   - First agent response includes a disclosure: "This response was generated
     by an AI system (roko agent, model: {model})"
   - AI-generated files include a comment header:
     `// Generated by AI (roko, {model}, {timestamp})`
3. Add C2PA-aligned metadata sidecar as `.roko/provenance/{run_id}.json`:
   ```json
   { "ai_generated": true, "generator": "roko", "model": "...", "timestamp": "..." }
   ```
4. Add `GET /api/compliance/posture` endpoint showing compliance state
5. Add `roko compliance report` CLI command for local generation

**Acceptance criteria**:
- With `eu_ai_act = true`, agent responses include AI disclosure on first turn
- AI-generated files include metadata comment header
- `GET /api/compliance/posture` returns JSON compliance report
- Provenance sidecar files are created for each run

---

## PHASE 7: Security Validation and Observability

### Task 17.18: Security Configuration Validator

**Priority**: P2

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/config_cmd.rs`

**What**: Add `roko config validate-security` that checks the entire security
configuration for gaps. Wire into `roko doctor` as a security subsection.

**Steps**:
1. Check contract coverage: all roles used in plans have contracts
2. Check `dangerously_skip_permissions` is not set in config
3. Check MCP servers: version pinned (no `@latest`)
4. Check audit logging: enabled, retention >= 180 days
5. Check auth: enabled if bind is not loopback
6. Check network policy: configured with explicit allowlist
7. Output a security posture report: pass/warn/fail per check
8. Add `--json` flag for machine-readable output
9. Wire the check list into `roko doctor` as a "Security" section

**Acceptance criteria**:
- `roko config validate-security` produces a structured report
- Missing contracts for used roles trigger a warning
- Disabled audit logging triggers a failure
- `roko doctor` includes a security subsection with pass/warn/fail

---

### Task 17.19: ASR (Attack Success Rate) Tracking

**Priority**: P3

**Files to create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/security_metrics.rs`

**What**: Track the ratio of blocked vs. total safety checks. Surface as a
metric for monitoring safety enforcement effectiveness.

**Steps**:
1. Define `SecurityMetrics` with counters per category (contract, permission,
   sanitization, rate_limit, network)
2. Increment on every safety check from the audit trail
3. Persist to `.roko/learn/security-metrics.json`
4. Add `roko learn security` CLI command showing metrics
5. Alert when ASR rises above 5% threshold

**Acceptance criteria**:
- `roko learn security` shows safety check breakdown by category
- Metrics persist across sessions
- ASR > 5% triggers a warning in `roko doctor`

---

## PHASE 8: MCP Security

### Task 17.20: MCP Server Version Pinning

**Priority**: P2

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/` (MCP config schema)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/mcp.rs`

**What**: Require explicit version pinning for MCP servers when
`safety.mcp.strict = true`. Reject `@latest` or unversioned packages.

**Steps**:
1. Extend MCP config schema to include version and trust_level fields
2. When `safety.mcp.strict = true`, reject configs where version is
   `latest`, missing, or a range
3. Emit a warning for `unknown` trust_level servers
4. Block `unknown` servers when `safety.mcp.block_unknown = true`
5. Add validation to `roko config mcp list` output

**Acceptance criteria**:
- `@modelcontextprotocol/server-filesystem@latest` is rejected in strict mode
- `@modelcontextprotocol/server-filesystem@1.2.3` is accepted
- `roko config mcp list` shows version and trust level per server

---

### Task 17.21: MCP Tool Call Sanitization

**Priority**: P2

**Files to create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/mcp_sanitize.rs`

**Files to modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/mod.rs` (add module)

**What**: Sanitize inputs to and outputs from MCP tool calls. Block SSRF
patterns, path traversal, and data exfiltration.

**Steps**:
1. Input sanitization:
   - Block SSRF patterns in URL arguments (private IPs, cloud metadata)
   - Block path traversal in file arguments
   - Validate argument types against MCP tool schemas
2. Output sanitization:
   - Apply `ResultFilter` pipeline (reuse from Task 17.7)
   - Strip prompt injection payloads
   - Size-limit responses (reject > 1MB)
3. Rate-limit MCP tool calls per server (default: 60/minute)
4. Log all MCP calls to `.roko/audit/mcp.jsonl`

**Acceptance criteria**:
- MCP call with `url: "http://169.254.169.254/metadata"` is blocked
- MCP response containing `<system>` injection markers is sanitized
- MCP calls exceeding rate limits return a clear error

---

## Dependency Graph

```
Phase 1 (Kill Permissive Default)
  17.1 (change default) ──┐
  17.2 (wire contracts)  ─┼──> 17.4 (init contracts)
  17.3 (workflow engine) ─┘

Phase 2 (Wire Built Policies)
  17.5 (bash denylist)     depends on: none
  17.6 (network policy)    depends on: none
  17.7 (result filter)     depends on: none
  17.8 (rate limiter)      depends on: none
  17.9 (secret scrubbing)  depends on: none

Phase 3 (Trust Isolation)
  17.10 (gate immutability)      depends on: none
  17.11 (worktree isolation)     depends on: 17.5
  17.12 (inter-agent sanitize)   depends on: 17.7

Phase 4 (HTTP Security)
  17.13 (auto-provision auth)    depends on: none
  17.14 (CLI share scrubbing)    depends on: 17.9

Phase 5 (Audit)
  17.15 (audit trail)            depends on: none (but all Tasks 17.1-17.14 should emit to it)
  17.16 (retention policy)       depends on: 17.15

Phase 6 (Compliance)
  17.17 (Article 50)             depends on: 17.15

Phase 7 (Validation)
  17.18 (security validator)     depends on: 17.1, 17.2, 17.15
  17.19 (ASR tracking)           depends on: 17.15

Phase 8 (MCP)
  17.20 (version pinning)        depends on: none
  17.21 (MCP sanitization)       depends on: 17.7
```

**Critical path**: 17.1 -> 17.2 -> 17.4 -> 17.18 (contract enforcement end-to-end)

**Parallelizable**: All Phase 2 tasks (17.5-17.9) are independent and can run
concurrently. Phase 4 tasks are independent. Phase 8 tasks are independent.

---

## Effort Estimates

| Phase | Tasks | Effort | Priority |
|---|---|---|---|
| 1: Kill Permissive Default | 17.1-17.4 | 2-3 days | **P0** — system is running wide-open |
| 2: Wire Built Policies | 17.5-17.9 | 3-4 days | **P1** — 18 submodules built but unused |
| 3: Trust Isolation | 17.10-17.12 | 2-3 days | P1 — defense in depth |
| 4: HTTP Security | 17.13-17.14 | 1-2 days | **P0** — cloud deployments unauthenticated |
| 5: Audit | 17.15-17.16 | 2-3 days | P1 — foundation for compliance |
| 6: Compliance | 17.17 | 2 days | P2 — deadline Aug 2026 |
| 7: Validation | 17.18-17.19 | 2 days | P2 — observability |
| 8: MCP | 17.20-17.21 | 2-3 days | P2 — supply chain |

**Total**: 16-23 days across 21 tasks

**Immediate priorities**: Tasks 17.1, 17.2, 17.13 (P0 — currently running permissive
and unauthenticated on cloud)

---

## Risk Factors

1. **Backward compatibility**: Existing plans that rely on permissive defaults
   will break when Task 17.1 lands. Migration path: `--skip-permissions` flag
   provides explicit opt-out. Add a deprecation warning for one release cycle.

2. **Performance overhead**: Every safety check adds latency. Budget 50ms total
   for the full safety chain per tool call. The contract `check_pre_execution()`
   is already fast (pure match logic, no I/O). Profile before and after.

3. **False positives**: The bash deny patterns and network blocks may catch
   legitimate agent commands. Start conservative (block only Critical-tier
   patterns), expand based on observed false positive rate from the audit trail.

4. **Claude CLI limitations**: Some safety enforcement depends on Claude CLI
   supporting `--allowedTools` and `--settings` hooks. If the CLI changes its
   flag interface, `build_settings_json()` and the tool allowlist need updating.
   The `with_dangerously_skip_permissions(false)` path must work correctly with
   the installed Claude CLI version.

5. **Contract YAML format**: The bundled contracts use JSON-in-YAML (`.yaml`
   extension but JSON content). If users edit them expecting YAML syntax, the
   JSON parser will reject them. Consider migrating to actual YAML or
   documenting the format clearly.
