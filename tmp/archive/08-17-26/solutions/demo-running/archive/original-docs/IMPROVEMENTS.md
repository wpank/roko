# Demo Pipeline Improvement Recommendations

**Date**: 2026-05-04 (updated — real pipeline run analysis added)
**Source**: Full audit of `tmp/solutions/demo-running/` + deep code analysis + real end-to-end pipeline run (gpt54-mini, BTC Funding Alert CLI, 4/4 tasks, 161s)
**Scope**: Speed, reliability, prompt quality, structural patterns, generalization, code health, runner architecture, compose/template system, serve/SSE/WS, learning subsystem, config system, pipeline run findings
**Total items**: 103 improvements across 14 categories
**Total anti-patterns found**: ~8,000 instances across 18 crates

---

## 1. Critical Bugs (Fix First)

### 1.1 Gate channel send failure causes task hang

**File**: `crates/roko-cli/src/runner/event_loop.rs:2550-2552`

**Bug**: Auto-pass gate for read-only roles sends completion via `tokio::spawn`. If `gate_tx.send()` fails (buffer full, receiver dropped), the error is logged but the task hangs forever — the event loop never receives the gate completion.

**Steps**:
1. Open `crates/roko-cli/src/runner/event_loop.rs`
2. Find the `tokio::spawn(async move { if let Err(e) = gate_tx.send(completion).await` block (~line 2550)
3. Add a fallback: if send fails, send a `Fatal` event via the executor channel instead:
   ```rust
   tokio::spawn(async move {
       if let Err(e) = gate_tx.send(completion).await {
           error!(plan_id = %plan_id, err = %e,
               "CRITICAL: failed to send auto-pass gate — task will hang");
           // Send fatal via executor to prevent infinite wait
           let _ = fatal_tx.send(ExecutorEvent::Fatal(
               format!("gate channel closed: {e}")
           )).await;
       }
   });
   ```
4. Thread a `fatal_tx` (clone of the executor event sender) into the spawn closure

### 1.2 Chain client unwrap panics without type guard

**File**: `crates/roko-cli/src/orchestrate.rs:16348`

**Bug**: `Arc::clone(self.chain_client.as_ref().unwrap())` — guarded by `if self.chain_client.is_some()` at line 16346, but not bound by the type system. Desync between check and use = panic.

**Steps**:
1. Open `crates/roko-cli/src/orchestrate.rs`
2. Find line 16346-16348
3. Replace with pattern match:
   ```rust
   if let Some(client) = self.chain_client.as_ref() {
       let client = Arc::clone(client);
       // ... use client ...
   }
   ```

### 1.3 Lock poisoning causes unrecoverable panic

**File**: `crates/roko-cli/src/orchestrate.rs:1552, 1556, 17875`

**Bug**: Three `.lock().expect()` calls on `std::sync::Mutex`. If any panic occurs while holding these locks, all subsequent calls panic permanently.

**Steps**:
1. Find all three locations in orchestrate.rs
2. Replace with `parking_lot::Mutex` (already a dependency — used at line 868):
   ```rust
   // Before
   self.stats.lock().expect("enrichment stats lock").clone()
   // After
   self.stats.lock().clone()
   ```
   `parking_lot::Mutex` is not poisonable — it auto-recovers after panics.

### 1.4 Config `from_toml` skips reference validation

**File**: `crates/roko-core/src/config/schema.rs:164-178`

**Bug**: `RokoConfig::from_toml()` deserializes and warns about schema version but never calls `validate_references()`. A config with a model referencing a non-existent provider loads successfully and fails at dispatch time (minutes later).

**Steps**:
1. Open `crates/roko-core/src/config/schema.rs`
2. Find `pub fn from_toml(s: &str)` (~line 164)
3. Add validation after deserialization:
   ```rust
   pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
       let config: Self = toml::from_str(s)?;
       // Validate that all model.provider references exist
       let warnings = config.validate_references();
       for w in &warnings {
           tracing::warn!("config validation: {}", w);
       }
       // ... existing version check ...
       Ok(config)
   }
   ```

### 1.5 Synthesized model profiles reference non-existent providers

**File**: `crates/roko-core/src/config/schema.rs:236-283`

**Bug**: `synthesized_model_profile()` falls back to `expected_kind.label()` when no provider of the expected kind exists. This label (e.g., `"claude"`) may not be a key in `self.providers`, causing silent failures at dispatch.

**Steps**:
1. Find `fn synthesized_model_profile` (~line 236)
2. Change the fallback from silent label to explicit error:
   ```rust
   let provider = self.providers.iter()
       .find(|(_, p)| p.kind == expected_kind)
       .map(|(name, _)| name.as_str());
   let provider = match provider {
       Some(p) => p.to_owned(),
       None => {
           tracing::warn!(slug = %slug, kind = ?expected_kind,
               "no provider configured for synthesized model; using label as fallback");
           expected_kind.label().to_owned()
       }
   };
   ```

### 1.6 Shell injection in demo `roko()` command builder

**File**: `demo/demo-app/src/lib/terminal-session.ts:139-145`

**Bug**: `ctx.activeModel` is injected into shell commands without quoting:
```typescript
return `${bin} --model ${ctx.activeModel} ${subcommand}`;
```
If `activeModel` contains shell metacharacters, commands are injectable.

**Steps**:
1. Open `demo/demo-app/src/lib/terminal-session.ts`
2. Find the `roko()` function
3. Use `shellQuote()` (already exists at line 104) for the model value:
   ```typescript
   if (ctx.activeModel) {
       return `${bin} --model ${shellQuote(ctx.activeModel)} ${subcommand}`;
   }
   ```

---

## 2. Speed Improvements

### 2.1 Deterministic TOML repair pipeline

**Current**: Plan generation → parse failure → retry with stricter prompt (up to 2 retries × full LLM call each). Each retry costs 15-60s.

**Files to modify**:
- `crates/roko-cli/src/task_parser.rs` — add `repair_toml()` function
- `crates/roko-cli/src/commands/prd.rs` — call repair before retry

**Steps**:
1. In `task_parser.rs`, add a new function after `validate()`:
   ```rust
   pub fn repair_toml(raw: &str) -> String {
       let mut s = raw.to_string();
       // Step 1: Strip markdown fences (already exists in parse_agent_output)
       s = strip_markdown_fences(&s);
       // Step 2: Strip trailing prose after last ]]
       if let Some(pos) = s.rfind("]]") {
           s.truncate(pos + 2);
       }
       // Step 3: Fix known typos (already exists — 19 patterns)
       s = fix_known_typos(&s);
       // Step 4: Detect merged fields — split at known field boundaries
       s = split_merged_fields(&s);
       // Step 5: Fix unclosed strings
       s = close_unclosed_strings(&s);
       s
   }
   ```
2. Add `split_merged_fields()`:
   ```rust
   fn split_merged_fields(s: &str) -> String {
       // Known field names that get merged by LLMs
       let field_boundaries = ["max_loc", "timeout_secs", "max_retries",
           "model_hint", "allowed_tools", "denied_tools"];
       let mut result = s.to_string();
       for field in &field_boundaries {
           // Pattern: `value_of_prev_field{field} = ` → split into two lines
           let pattern = format!("{} = ", field);
           // Find occurrences where the field name is preceded by non-whitespace
           // (indicating it was merged with the previous field's value)
           result = result.replace(
               &format!("\"{}",  pattern),  // e.g., `"claude-sonnet-4-amax_loc = `
               &format!("\"\n{}", pattern),  // split into two lines
           );
       }
       result
   }
   ```
3. Add `close_unclosed_strings()`:
   ```rust
   fn close_unclosed_strings(s: &str) -> String {
       s.lines().map(|line| {
           let quote_count = line.chars().filter(|&c| c == '"').count();
           if quote_count % 2 != 0 {
               format!("{}\"", line)
           } else {
               line.to_string()
           }
       }).collect::<Vec<_>>().join("\n")
   }
   ```
4. In `commands/prd.rs`, call `repair_toml()` before `toml::from_str()`:
   ```rust
   let repaired = task_parser::repair_toml(&raw_output);
   match toml::from_str::<TasksFile>(&repaired) {
       Ok(tasks) => tasks,
       Err(e) => {
           // Only now retry with LLM
           warn!("TOML repair failed, retrying with LLM: {}", e);
           // ... existing retry logic ...
       }
   }
   ```

**Expected impact**: Eliminates ~80% of LLM retries. Saves 30-120s per plan generation.

### 2.2 Warm cargo cache before plan execution

**File**: `crates/roko-cli/src/runner/event_loop.rs` — init phase (before main loop)

**Steps**:
1. After `scaffold_missing_crates()` returns (currently around line 250), add:
   ```rust
   // Warm cargo cache — makes subsequent per-task compile gates incremental
   if config.stream_to_stderr {
       eprintln!("[plan-run] Warming cargo cache...");
   }
   let warm_result = tokio::process::Command::new("cargo")
       .args(["check", "--workspace"])
       .current_dir(&config.workdir)
       .stdout(Stdio::null())
       .stderr(Stdio::null())
       .status()
       .await;
   if let Err(e) = warm_result {
       warn!("cargo cache warm failed (non-fatal): {}", e);
   }
   ```
2. Gate the warm-up on a config flag: `config.warm_cache` (default true)
3. Add `warm_cache: bool` to `RunConfig` in `runner/types.rs`

**Expected impact**: First compile gate drops from 30-120s to 2-5s.

### 2.3 Batch gate execution (compile + clippy in parallel)

**File**: `crates/roko-cli/src/runner/event_loop.rs` — gate dispatch logic

**Steps**:
1. In the gate completion handler (where `rung < max_gate_rung` triggers next rung), add logic:
   ```rust
   // If compile passed and clippy is next, run clippy AND test in parallel
   // (clippy and test are independent of each other)
   if rung == Rung::Compile && next_rung == Rung::Lint {
       // Don't wait for clippy before starting test
       // Schedule both clippy and test simultaneously
       schedule_gate(plan_id, task_id, Rung::Lint, ctx);
       // If test is also enabled, schedule it too
       if max_rung >= Rung::Test {
           schedule_gate(plan_id, task_id, Rung::Test, ctx);
       }
       // Mark that we're waiting for both
       state.pending_parallel_gates.insert((plan_id, task_id), 2);
       return; // Don't schedule test sequentially
   }
   ```
2. In the gate completion handler, decrement the parallel gate counter:
   ```rust
   if let Some(remaining) = state.pending_parallel_gates.get_mut(&(plan_id, task_id)) {
       *remaining -= 1;
       if *remaining > 0 {
           return; // Still waiting for other parallel gates
       }
       state.pending_parallel_gates.remove(&(plan_id, task_id));
   }
   ```
3. Add `pending_parallel_gates: HashMap<(PlanId, TaskId), usize>` to `RunState`

**Expected impact**: 15-30s savings per task (clippy runs while compile finishes).

### 2.4 Gate channel buffer sizing

**File**: `crates/roko-cli/src/runner/event_loop.rs:260-261`

**Bug**: Gate channel buffer is hardcoded to 16. With 4 concurrent tasks × 7 rungs = 28 possible in-flight gate completions, the buffer can overflow.

**Steps**:
1. Find line 261: `let (gate_tx, mut gate_rx) = mpsc::channel::<GateCompletion>(16);`
2. Replace with dynamic sizing:
   ```rust
   let gate_buffer = (config.max_concurrent_tasks * 7).max(32).min(256);
   let (gate_tx, mut gate_rx) = mpsc::channel::<GateCompletion>(gate_buffer);
   ```

### 2.5 Connection pooling for LLM API calls

**File**: `crates/roko-agent/src/provider/openai_compat.rs` — client construction

**Steps**:
1. In `SharedAgentFactory` (event_loop.rs), add a shared HTTP client:
   ```rust
   pub struct SharedAgentFactory {
       // ... existing fields ...
       http_client: reqwest::Client, // shared, connection-pooled
   }
   ```
2. Pass this client into each `OpenAiCompatLlmBackend` construction instead of creating new clients per dispatch
3. Configure the client with connection pool limits:
   ```rust
   let http_client = reqwest::Client::builder()
       .pool_max_idle_per_host(4)
       .pool_idle_timeout(Duration::from_secs(30))
       .timeout(Duration::from_secs(config.timeout_secs))
       .build()?;
   ```

---

## 3. Reliability Improvements

### 3.1 Atomic state writes

**File**: `crates/roko-cli/src/runner/event_loop.rs:1620-1635`

**Steps**:
1. Add `atomic_write` helper to `runner/persist.rs`:
   ```rust
   pub fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
       let tmp = path.with_extension("tmp");
       fs::write(&tmp, data)?;
       fs::rename(&tmp, path)?;
       Ok(())
   }
   ```
2. Replace all `fs::write(path, data)` calls in `save_snapshot()` with `atomic_write(path, data)`
3. Add a checkpoint file that records the expected state files + their SHA256:
   ```rust
   fn write_checkpoint(state_dir: &Path, files: &[(&str, &[u8])]) -> io::Result<()> {
       use sha2::{Sha256, Digest};
       let entries: Vec<_> = files.iter().map(|(name, data)| {
           let hash = hex::encode(Sha256::digest(data));
           format!("{}:{}", name, hash)
       }).collect();
       atomic_write(
           &state_dir.join("checkpoint.txt"),
           entries.join("\n").as_bytes(),
       )
   }
   ```
4. On resume, verify checkpoint before accepting state:
   ```rust
   fn verify_checkpoint(state_dir: &Path) -> Result<bool> {
       let checkpoint = fs::read_to_string(state_dir.join("checkpoint.txt"))?;
       for line in checkpoint.lines() {
           let (name, expected_hash) = line.split_once(':').context("bad checkpoint")?;
           let data = fs::read(state_dir.join(name))?;
           let actual_hash = hex::encode(Sha256::digest(&data));
           if actual_hash != expected_hash {
               return Ok(false); // Corrupt — start fresh
           }
       }
       Ok(true)
   }
   ```

### 3.2 Typed error taxonomy (replace string matching)

**Files**:
- `crates/roko-agent/src/task_runner.rs:596` (string matching for error classification)
- `crates/roko-agent/src/error.rs` (new file)
- `crates/roko-cli/src/runner/event_loop.rs` (update retry logic)

**Steps**:
1. Create `crates/roko-agent/src/error.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum AgentFailureKind {
       RateLimited { retry_after_secs: Option<u64> },
       AuthExpired { provider: String },
       ModelUnavailable { model: String, provider: String },
       ContextExceeded { tokens_used: usize, limit: usize },
       ToolCallFailed { tool: String, reason: String },
       OutputMalformed { expected: String, got_preview: String },
       Timeout { elapsed_secs: u64, limit_secs: u64 },
       ProcessCrashed { exit_code: i32, stderr_preview: String },
       NetworkError { url: String, status: Option<u16> },
       Unknown { message: String },
   }

   impl AgentFailureKind {
       pub fn is_retryable(&self) -> bool {
           matches!(self,
               Self::RateLimited { .. } |
               Self::Timeout { .. } |
               Self::NetworkError { .. }
           )
       }

       pub fn suggested_cooldown(&self) -> Duration {
           match self {
               Self::RateLimited { retry_after_secs: Some(s) } => Duration::from_secs(*s),
               Self::RateLimited { .. } => Duration::from_secs(30),
               Self::Timeout { .. } => Duration::from_secs(5),
               Self::NetworkError { .. } => Duration::from_secs(10),
               _ => Duration::from_secs(0),
           }
       }
   }
   ```
2. In `task_runner.rs`, replace the string-based classification:
   ```rust
   // Before:
   } else if text.contains("io error") || text.contains("filesystem") { ... }
   // After:
   fn classify_error(status: Option<u16>, message: &str) -> AgentFailureKind {
       match status {
           Some(429) => AgentFailureKind::RateLimited {
               retry_after_secs: extract_retry_after(message),
           },
           Some(401) | Some(403) => AgentFailureKind::AuthExpired {
               provider: extract_provider(message),
           },
           Some(404) => AgentFailureKind::ModelUnavailable { ... },
           _ => AgentFailureKind::Unknown { message: message.to_string() },
       }
   }
   ```
3. Update the retry logic in event_loop.rs to use `kind.is_retryable()` and `kind.suggested_cooldown()` instead of string matching

### 3.3 Prevent cross-plan state leakage with `--fresh`

**File**: `crates/roko-cli/src/commands/plan.rs:243-268`

**Steps**:
1. Find the `--fresh` handler (~line 246-268)
2. Expand to remove ALL state files:
   ```rust
   if args.fresh {
       let state_dir = roko_dir.join("state");
       let state_files = [
           "executor.json",
           "orchestrator.json",
           "run-state.json",
           "events.json",
       ];
       for file in &state_files {
           let path = state_dir.join(file);
           if path.exists() {
               let backup = path.with_extension(format!("json.bak.{}",
                   std::time::SystemTime::now()
                       .duration_since(std::time::UNIX_EPOCH)
                       .unwrap_or_default()
                       .as_millis()));
               if let Err(e) = fs::rename(&path, &backup) {
                   warn!("failed to archive {}: {}", file, e);
               }
           }
       }
   }
   ```
3. Update `--help` text to document what `--fresh` removes

### 3.4 Schema-driven TOML validation

**File**: `crates/roko-cli/src/task_parser.rs` — add `TaskFieldSchema`

**Steps**:
1. Define the schema struct after `TaskDef`:
   ```rust
   struct TaskFieldSchema {
       required_by_role: HashMap<&'static str, Vec<&'static str>>,
       valid_roles: Vec<&'static str>,
       valid_tiers: Vec<&'static str>,
       valid_statuses: Vec<&'static str>,
       valid_replan_strategies: Vec<&'static str>,
   }

   impl TaskFieldSchema {
       fn default_schema() -> Self {
           Self {
               required_by_role: HashMap::from([
                   ("implementer", vec!["verify", "files", "context"]),
                   ("researcher", vec!["context"]),
                   ("strategist", vec![]),
                   ("quick-reviewer", vec!["context"]),
               ]),
               valid_roles: vec!["implementer", "researcher", "strategist",
                   "architect", "reviewer", "quick-reviewer", "scribe"],
               valid_tiers: vec!["mechanical", "focused", "integrative", "architectural"],
               valid_statuses: vec!["pending", "active", "done", "blocked", "skipped"],
               valid_replan_strategies: vec!["retry", "decompose", "escalate", "skip"],
           }
       }
   }
   ```
2. Add a `validate_against_schema()` method to `TasksFile`:
   ```rust
   pub fn validate_against_schema(&self) -> Vec<String> {
       let schema = TaskFieldSchema::default_schema();
       let mut issues = Vec::new();
       for task in &self.tasks {
           // Check role is valid
           if !schema.valid_roles.contains(&task.role.as_str()) {
               issues.push(format!("task {}: unknown role '{}'", task.id, task.role));
           }
           // Check required fields for this role
           if let Some(required) = schema.required_by_role.get(task.role.as_str()) {
               for field in required {
                   match *field {
                       "verify" if task.verify.is_empty() =>
                           issues.push(format!("task {}: missing verify steps (required for {})",
                               task.id, task.role)),
                       "files" if task.files.is_empty() =>
                           issues.push(format!("task {}: missing files list", task.id)),
                       "context" if task.context.is_none() =>
                           issues.push(format!("task {}: missing context section", task.id)),
                       _ => {}
                   }
               }
           }
           // Check tier is valid
           if let Some(ref tier) = task.tier {
               if !schema.valid_tiers.contains(&tier.as_str()) {
                   issues.push(format!("task {}: unknown tier '{}'", task.id, tier));
               }
           }
           // Check numeric bounds
           if task.timeout_secs == 0 {
               issues.push(format!("task {}: timeout_secs must be > 0", task.id));
           }
           if task.max_loc.map_or(false, |m| m == 0) {
               issues.push(format!("task {}: max_loc must be > 0", task.id));
           }
       }
       issues
   }
   ```
3. Call `validate_against_schema()` from `validate_before_run()` in `commands/plan.rs`

### 3.5 Scaffold Cargo.toml insertion via TOML parser

**File**: `crates/roko-cli/src/runner/plan_loader.rs:201-207`

**Bug**: Workspace Cargo.toml member insertion uses string search for `]`, which breaks on comments inside the members array.

**Steps**:
1. Replace the string-based insertion with `toml_edit`:
   ```rust
   use toml_edit::{Document, value, Array};

   fn add_workspace_member(cargo_toml_path: &Path, member: &str) -> Result<()> {
       let content = fs::read_to_string(cargo_toml_path)?;
       let mut doc = content.parse::<Document>()?;

       let members = doc["workspace"]["members"]
           .as_array_mut()
           .context("workspace.members is not an array")?;

       // Check if already present
       if members.iter().any(|m| m.as_str() == Some(member)) {
           return Ok(());
       }

       members.push(member);
       fs::write(cargo_toml_path, doc.to_string())?;
       Ok(())
   }
   ```
2. Add `toml_edit = "0.22"` to `crates/roko-cli/Cargo.toml` (if not already present)

### 3.6 Validate crate names in scaffold

**File**: `crates/roko-cli/src/runner/plan_loader.rs:122-144`

**Bug**: No validation against directory traversal (`..`) or invalid Rust crate names.

**Steps**:
1. Add validation function:
   ```rust
   fn is_valid_crate_name(name: &str) -> bool {
       !name.is_empty()
           && !name.starts_with('-')
           && !name.starts_with('.')
           && !name.contains("..")
           && !name.contains('/')
           && !name.contains('\\')
           && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
   }
   ```
2. Use it in the scaffold loop:
   ```rust
   if !is_valid_crate_name(&crate_name) {
       info!(crate_name = %crate_name, "scaffold: skipping invalid crate name");
       continue;
   }
   ```

---

## 4. Prompt Quality Improvements

### 4.1 Add workspace context to task prompts (CRITICAL)

**File**: `crates/roko-cli/src/orchestrate.rs` — `dispatch_agent_with()` (~line 14882)

**Steps**:
1. Before building the system prompt in `dispatch_agent_with()`, generate a workspace listing:
   ```rust
   fn workspace_context(workdir: &Path) -> String {
       let mut ctx = String::from("## Current Workspace State\n\n");
       // List crate directories
       if let Ok(entries) = fs::read_dir(workdir.join("crates")) {
           ctx.push_str("### Crates in workspace:\n");
           for entry in entries.flatten() {
               if entry.path().is_dir() {
                   let name = entry.file_name().to_string_lossy().to_string();
                   let has_lib = entry.path().join("src/lib.rs").exists();
                   let has_main = entry.path().join("src/main.rs").exists();
                   let kind = if has_main { "binary" } else if has_lib { "library" } else { "empty" };
                   ctx.push_str(&format!("- `crates/{}/` ({})\n", name, kind));
               }
           }
       }
       // List workspace Cargo.toml members
       if let Ok(content) = fs::read_to_string(workdir.join("Cargo.toml")) {
           if let Ok(doc) = content.parse::<toml::Value>() {
               if let Some(members) = doc.get("workspace")
                   .and_then(|w| w.get("members"))
                   .and_then(|m| m.as_array()) {
                   ctx.push_str("\n### Workspace members (from Cargo.toml):\n");
                   for m in members {
                       if let Some(s) = m.as_str() {
                           ctx.push_str(&format!("- `{}`\n", s));
                       }
                   }
               }
           }
       }
       ctx
   }
   ```
2. Inject this context into the system prompt before the task-specific content
3. Cap the context at 2000 tokens to avoid bloating the prompt

### 4.2 Fix model_hint contradiction in plan generator

**File**: `crates/roko-cli/src/plan_generate.rs:128-134`

**Steps**:
1. Delete the `model_hint()` method from `TaskTier`:
   ```rust
   // REMOVE this entire method:
   // pub const fn model_hint(&self) -> &'static str { ... }
   ```
2. Update any callers (search `model_hint()` in plan_generate.rs) — there are ~3 test assertions and 0 production callers
3. Update tests to verify the method no longer exists:
   ```rust
   #[test]
   fn tier_does_not_expose_model_hints() {
       // TaskTier should only expose complexity metadata, not model names
       assert_eq!(TaskTier::Mechanical.max_loc(), 20);
       assert_eq!(TaskTier::Mechanical.label(), "mechanical");
       // model_hint() should not exist — model selection is config-driven
   }
   ```

### 4.3 Add failure recovery guidance to implementer template

**File**: `crates/roko-compose/src/templates/implementer.rs` — `ROLE_IDENTITY` constant

**Steps**:
1. Find the `ROLE_IDENTITY` static string (near top of file)
2. Append the following after the existing rules:
   ```rust
   const ROLE_IDENTITY: &str = r#"
   ...existing content...

   ## When Things Go Wrong

   If `cargo check` fails:
   1. Read the FIRST compiler error only — fix it, then recheck. Later errors often cascade from the first.
   2. Common causes: missing `use` import, wrong type, missing trait implementation.
   3. Do NOT add `#[allow(...)]` to suppress real warnings. Only suppress unused-import warnings for imports you intentionally changed.

   If tests fail after your change:
   1. Run the specific failing test: `cargo test -p <crate> -- <test_name> --nocapture`
   2. If the test was asserting OLD behavior that your change intentionally updated, update the test's expected values.
   3. If the test failure reveals a bug in YOUR change, fix your code — not the test.
   4. Never delete or skip a test to make your change pass.

   If your change breaks a file NOT in your task's `files` list:
   1. If the fix is a simple import or type annotation change (1-3 lines), fix it.
   2. If the fix requires significant changes, note it in your output as a dependency issue and continue with your assigned files.
   "#;
   ```

### 4.4 Add few-shot TOML example to plan generator prompt

**File**: `crates/roko-cli/src/plan_generate.rs` — `PLAN_GENERATOR_SYSTEM_PROMPT`

**Steps**:
1. Find the end of the `PLAN_GENERATOR_SYSTEM_PROMPT` constant (~line 350)
2. Add a complete example before the closing `"`:
   ```
   ## Complete Example

   For a PRD titled "Add health check endpoint to roko-serve":

   ```toml
   [meta]
   plan = "health-check-endpoint"
   total = 3
   status = "pending"
   max_parallel = 1

   [[task]]
   id = "T1"
   title = "Research existing health patterns"
   description = "Read the current serve routes to understand the pattern for adding new endpoints."
   role = "researcher"
   tier = "mechanical"
   status = "pending"
   files = []
   depends_on = []
   [task.context]
   read_files = ["crates/roko-serve/src/routes/mod.rs", "crates/roko-serve/src/routes/status/mod.rs"]

   [[task]]
   id = "T2"
   title = "Implement /health endpoint"
   description = "Add a GET /health endpoint that returns 200 OK with JSON body containing version and uptime."
   role = "implementer"
   tier = "focused"
   status = "pending"
   timeout_secs = 600
   max_retries = 2
   max_loc = 50
   files = ["crates/roko-serve/src/routes/status/health.rs", "crates/roko-serve/src/routes/status/mod.rs"]
   depends_on = ["T1"]
   [task.context]
   read_files = ["crates/roko-serve/src/routes/mod.rs"]
   anti_patterns = ["Do NOT modify existing routes", "Do NOT add external dependencies"]
   [[task.verify]]
   command = "cargo check -p roko-serve"
   description = "Code compiles"
   [[task.verify]]
   command = "cargo test -p roko-serve -- health"
   description = "Health endpoint tests pass"

   [[task]]
   id = "T3"
   title = "Add integration test"
   description = "Write a test that starts the server and hits GET /health."
   role = "implementer"
   tier = "focused"
   status = "pending"
   timeout_secs = 600
   max_loc = 40
   files = ["crates/roko-serve/tests/health_test.rs"]
   depends_on = ["T2"]
   [task.context]
   read_files = ["crates/roko-serve/src/routes/status/health.rs"]
   [[task.verify]]
   command = "cargo test -p roko-serve -- health_test"
   description = "Integration test passes"
   ```
   ```

### 4.5 Add role-tool mapping to plan generator prompt

**File**: `crates/roko-cli/src/plan_generate.rs` — `PLAN_GENERATOR_SYSTEM_PROMPT`

**Steps**:
1. Find the roles section in the prompt (~line 277-289)
2. Add tool constraints after the role definitions:
   ```
   ## Role-Tool Constraints

   Each role has specific tool access. The runtime enforces these — do not assign tasks
   that require tools the role doesn't have:

   | Role | Can Read | Can Write | Can Execute | Notes |
   |------|----------|-----------|-------------|-------|
   | researcher | Yes | No | No | Gathers information only |
   | strategist | Yes | No | No | Plans and analyzes only |
   | implementer | Yes | Yes | Yes | Full toolkit |
   | architect | Yes | Yes | No | Designs, may write specs |
   | quick-reviewer | Yes | No | No | Reviews code, no changes |
   | scribe | Yes | Yes | No | Documentation only |

   A researcher task that says "update the file" will FAIL because researchers
   cannot write files. Use an implementer for any task that modifies files.
   ```

### 4.6 Consolidate file path rules in plan generator

**File**: `crates/roko-cli/src/plan_generate.rs` — `PLAN_GENERATOR_SYSTEM_PROMPT`

**Steps**:
1. Find the first occurrence of file path rules (~line 204-205)
2. Remove the duplicate occurrence (~line 337-345)
3. Replace both with a single, definitive section placed BEFORE the task structure example:
   ```
   ## File Path Rules (ALL fields that reference files)

   1. CONCRETE paths only: `crates/roko-foo/src/lib.rs` — never `crates/*/src/*.rs` or `crates/`
   2. Paths are relative to workspace root (no leading `/`)
   3. For new crates: `crates/{slug}/src/lib.rs` (library) or `crates/{slug}/src/main.rs` (binary)
   4. `files` = the COMPLETE list of files this task will CREATE or MODIFY
   5. `context.read_files` = files the agent should READ for context (may overlap with `files`)
   6. Do NOT include directory paths — always specify the exact file
   7. If a task creates a new crate, include BOTH `Cargo.toml` and the source file in `files`
   ```

---

## 5. Structural & Design Pattern Improvements

### 5.1 Extract dispatch-and-record helper from orchestrate.rs

**File**: `crates/roko-cli/src/orchestrate.rs` — 11 duplicated dispatch patterns

**Problem**: The same dispatch → match Ok/Err → record episode → emit signals flow is repeated 11 times at lines 9318, 9785, 12136, 12203, 12313, 12900, 14085, 14289, 14453, 21494, 21573.

**Steps**:
1. Define a helper struct for dispatch outcomes:
   ```rust
   struct DispatchOutcome {
       exit_code: i32,
       output: String,
       cost_usd: f64,
       tokens_in: u64,
       tokens_out: u64,
       elapsed: Duration,
   }
   ```
2. Extract the common pattern into a method:
   ```rust
   async fn dispatch_and_record(
       &mut self,
       plan_id: &str,
       task_id: &str,
       role: &str,
       prompt: &str,
       model: Option<&str>,
       exec_dir: &Path,
   ) -> Result<DispatchOutcome> {
       let result = self.dispatch_agent_with(plan_id, task_id, role, prompt, model, exec_dir).await;
       match result {
           Ok(outcome) => {
               self.record_episode(plan_id, task_id, &outcome);
               self.emit_efficiency_event(plan_id, task_id, &outcome);
               self.daimon.appraise(AffectEvent::TaskOutcome {
                   success: outcome.exit_code == 0,
                   cost: outcome.cost_usd,
               });
               Ok(outcome)
           }
           Err(e) => {
               self.record_failure_episode(plan_id, task_id, &e);
               Err(e)
           }
       }
   }
   ```
3. Replace all 11 call sites with `self.dispatch_and_record(...)`:
   - Line 9318: search for the first `dispatch_agent_with` call, replace the surrounding match block
   - Repeat for each of the 11 locations

### 5.2 Log all daimon/conductor/substrate errors

**File**: `crates/roko-cli/src/orchestrate.rs` — 30 `let _ =` instances

**Problem**: 12 `let _ = self.daimon.appraise(...)` calls silently drop errors from the affect engine. 2 `let _ = substrate.put(sig).await` calls silently drop signal persistence failures.

**Steps**:
1. Search for `let _ = self.daimon.appraise` (12 instances)
2. Replace each with:
   ```rust
   if let Err(e) = self.daimon.appraise(AffectEvent::...) {
       warn!(error = %e, "daimon appraisal failed (non-fatal)");
   }
   ```
3. Search for `let _ = self.conductor.decide` (1 instance at line 15118)
4. Replace with:
   ```rust
   if let Err(e) = self.conductor.decide(&signals, ...) {
       warn!(error = %e, "conductor decision failed (non-fatal)");
   }
   ```
5. Search for `let _ = substrate.put` (2 instances at lines 17232, 17250)
6. Replace with:
   ```rust
   if let Err(e) = substrate.put(sig).await {
       error!(error = %e, "signal persistence failed — audit trail may be incomplete");
   }
   ```

### 5.3 SafetyLayer should be required, not optional

**File**: `crates/roko-agent/src/dispatcher/mod.rs:89, 113`

**Steps**:
1. Change the field from `Option<SafetyLayer>` to `SafetyLayer`:
   ```rust
   pub struct ToolDispatcher {
       // ...
       safety: SafetyLayer,  // was Option<SafetyLayer>
   }
   ```
2. Make `SafetyLayer` have a `Permissive` variant for when no safety is needed:
   ```rust
   impl SafetyLayer {
       pub fn permissive() -> Self {
           Self { /* all checks pass, no restrictions */ }
       }
   }
   ```
3. Update the constructor to require safety:
   ```rust
   pub fn new(registry: Arc<dyn ToolRegistry>,
              resolver: Arc<dyn HandlerResolver>,
              safety: SafetyLayer) -> Self {
   ```
4. Update all callers — replace `.with_safety(layer)` with passing `layer` directly; replace omission of `.with_safety()` with `SafetyLayer::permissive()`

### 5.4 Extract streaming output as a pluggable sink

**File**: `crates/roko-cli/src/runner/agent_events.rs` — mixed in with event handling

**Steps**:
1. Create `crates/roko-cli/src/runner/output_sink.rs`:
   ```rust
   pub trait RunOutputSink: Send + Sync {
       fn task_started(&self, task_id: &str, title: &str, role: &str,
                       index: usize, total: usize);
       fn agent_line(&self, task_id: &str, line: &str);
       fn tool_call(&self, task_id: &str, tool_name: &str);
       fn gate_result(&self, task_id: &str, rung: &str, passed: bool, detail: &str);
       fn task_completed(&self, task_id: &str, elapsed: Duration, cost: f64);
       fn task_failed(&self, task_id: &str, reason: &str);
       fn plan_summary(&self, completed: usize, failed: usize, total_cost: f64,
                       elapsed: Duration);
   }
   ```
2. Implement `StderrSink`:
   ```rust
   pub struct StderrSink {
       pub colorize: bool,
       pub verbose: bool, // show agent content lines
   }

   impl RunOutputSink for StderrSink {
       fn task_started(&self, task_id: &str, title: &str, role: &str,
                       index: usize, total: usize) {
           eprintln!("[plan-run] Starting task {}/{}: \"{}\" ({})",
               index + 1, total, title, role);
       }
       fn gate_result(&self, task_id: &str, rung: &str, passed: bool, detail: &str) {
           let icon = if passed { "✓" } else { "✗" };
           eprintln!("[plan-run]   {} {} {}", icon, rung,
               if detail.is_empty() { "" } else { detail });
       }
       // ... other methods ...
   }
   ```
3. Implement `NoopSink`:
   ```rust
   pub struct NoopSink;
   impl RunOutputSink for NoopSink {
       fn task_started(&self, ..) {}
       fn agent_line(&self, ..) {}
       // ... all no-ops ...
   }
   ```
4. Add `sink: Arc<dyn RunOutputSink>` to `RunContext`
5. Replace all `if config.stream_to_stderr { eprintln!(...) }` in `agent_events.rs` and `event_loop.rs` with `sink.task_started(...)` etc.

### 5.5 Environment variable parse warnings

**File**: `crates/roko-core/src/config/schema.rs:309-322`

**Steps**:
1. Find the `apply_env` method
2. Replace each silent `if let Ok(n) = v.parse::<T>()` with a warning on failure:
   ```rust
   if let Some(v) = env_fn("ROKO_CONTEXT_LIMIT_K") {
       match v.parse::<u32>() {
           Ok(n) => self.agent.context_limit_k = n,
           Err(e) => {
               tracing::warn!(
                   env_var = "ROKO_CONTEXT_LIMIT_K", value = %v, error = %e,
                   "invalid env var value; using default"
               );
           }
       }
   }
   ```
3. Repeat for all ~8 env var parse sites in the method

---

## 6. Code Health (Codebase-Wide)

### 6.1 Top 10 unwrap() replacements

**Total across codebase**: 3,534 unwrap/expect calls in non-test code.

**Priority files** (handle these first — highest production risk):

| # | File | Count | Fix Pattern |
|---|------|-------|-------------|
| 1 | `orchestrate.rs` | 60 | Replace with `?` or `if let` — these are in the main execution loop |
| 2 | `worktree.rs` | 56 | Replace with `?` — git operations should never panic |
| 3 | `main.rs` | 123 | Many are in CLI parsing (acceptable); audit the ~20 in runtime paths |
| 4 | `skill_library.rs` | 100 | Replace with `.unwrap_or_default()` or `?` |
| 5 | `dag.rs` | 57 | Replace with `.ok_or_else(|| anyhow!(...))` — DAG operations must not panic |
| 6 | `runtime_feedback.rs` | 85 | Replace with `?` — feedback system should degrade gracefully |
| 7 | `file_substrate.rs` | 81 | Replace with `?` — filesystem failures should return errors |
| 8 | `config.rs` | 73 | Replace with proper error propagation |
| 9 | `prd.rs` | 58 | Replace with `?` and `context()` |
| 10 | `index.rs` | 48 | Replace with `?` |

**Mechanical approach per file**:
1. Run `grep -n '\.unwrap()' <file> | grep -v '#\[test\]' | grep -v '#\[cfg(test)\]'`
2. For each hit, determine if it's in a function returning `Result` → replace with `?`
3. If the function returns a non-Result type, wrap in `if let Some/Ok`:
   - `.unwrap()` on `Option` → `.unwrap_or_default()` or `if let Some(x) = ...`
   - `.unwrap()` on `Result` → `.map_err(|e| warn!("...: {e}")).ok()?`

### 6.2 Hardcoded model name extraction

**Total**: 28 hardcoded model references.

**Steps**:
1. Create `crates/roko-core/src/defaults.rs` (if not exists):
   ```rust
   /// Default model slugs — referenced from config, not hardcoded in logic
   pub mod model_defaults {
       pub const DEFAULT_IMPLEMENTER_MODEL: &str = "sonnet";  // tier-based, not slug
       pub const DEFAULT_RESEARCHER_MODEL: &str = "haiku";
       pub const DEFAULT_REVIEWER_MODEL: &str = "opus";
   }
   ```
2. Search for all 28 instances: `grep -rn '"claude-' crates/ --include='*.rs' | grep -v test | grep -v '#\[cfg'`
3. Replace each with a reference to config or the defaults module:
   - `orchestrator/repair.rs:610` — use `config.repair.current_model`
   - `learn/episode_logger.rs:792` — use `config.cost.model_multipliers`
   - `gate/forensic.rs:290+` — use `config.gate.model`

### 6.3 Timeout centralization

**Total**: 2,371 hardcoded Duration values.

**Steps**:
1. Create `crates/roko-core/src/config/timeouts.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct TimeoutConfig {
       pub agent_dispatch_secs: u64,      // default 600
       pub gate_compile_secs: u64,        // default 120
       pub gate_test_secs: u64,           // default 300
       pub gate_clippy_secs: u64,         // default 60
       pub llm_call_secs: u64,            // default 120
       pub http_request_secs: u64,        // default 30
       pub workspace_lock_secs: u64,      // default 5
       pub health_check_secs: u64,        // default 3
       pub plan_total_secs: u64,          // default 3600
   }

   impl Default for TimeoutConfig { ... }
   ```
2. Add `[timeouts]` section to `RokoConfig` schema
3. Progressively replace hardcoded `Duration::from_secs(N)` with `config.timeouts.X`

---

## 7. Demo App Improvements

### 7.1 Timeout configuration via ScenarioContext

**File**: `demo/demo-app/src/lib/terminal-session.ts`

**Steps**:
1. Define a `TimeoutConfig` interface:
   ```typescript
   export interface TimeoutConfig {
       binaryDetection: number;   // default 4000
       execCheck: number;         // default 3000
       websocketOpen: number;     // default 8000
       shellPrompt: number;       // default 6000
       workspaceCd: number;       // default 5000
   }

   export const DEFAULT_TIMEOUTS: TimeoutConfig = {
       binaryDetection: 4000,
       execCheck: 3000,
       websocketOpen: 8000,
       shellPrompt: 6000,
       workspaceCd: 5000,
   };
   ```
2. Thread `timeouts?: Partial<TimeoutConfig>` through `resolveRoko()`, `enterWorkspace()`, and `showCmd()`
3. Use `{ ...DEFAULT_TIMEOUTS, ...timeouts }` to merge with defaults
4. Add a timeout multiplier to `ScenarioContext`:
   ```typescript
   interface ScenarioContext {
       timeoutMultiplier: number; // default 1.0, configurable via UI
       // ... existing fields ...
   }
   ```

### 7.2 Structured command result errors

**File**: `demo/demo-app/src/lib/terminal-session.ts`

**Steps**:
1. Add error codes to `CommandResult`:
   ```typescript
   export type CommandFailureReason =
       | 'timeout'
       | 'ws_closed'
       | 'command_error'
       | 'aborted'
       | 'unknown';

   export interface CommandResult {
       ok: boolean;
       elapsed: number;
       exitCode?: number;
       gates: GateResult[];
       cost: string | null;
       tokens: string | null;
       error?: string;
       failureReason?: CommandFailureReason;
   }
   ```
2. Populate `failureReason` in each error path of `showCmd()`:
   - Timeout path → `failureReason: 'timeout'`
   - WebSocket close → `failureReason: 'ws_closed'`
   - Non-zero exit → `failureReason: 'command_error'`

### 7.3 Single-source command definitions

**File**: `demo/demo-app/src/lib/scenario-runners/prd-pipeline.ts`

**Steps**:
1. Define a single command template list:
   ```typescript
   interface CommandTemplate {
       id: string;
       subcommand: string;  // roko subcommand (without binary path or flags)
       display: string;     // clean display text for sidebar
       description: string;
       timeout: number;
       needsModel: boolean; // whether --model flag should be injected
   }

   const PRD_PIPELINE_TEMPLATES: CommandTemplate[] = [
       { id: 'init', subcommand: 'init', display: 'roko init',
         description: 'Create workspace and config', timeout: 10000, needsModel: false },
       { id: 'idea', subcommand: `prd idea "${PRD_IDEA}"`,
         display: `roko prd idea "${PRD_IDEA}"`,
         description: 'Capture work item', timeout: 10000, needsModel: false },
       // ... etc
   ];
   ```
2. Generate both static display list and runtime commands from this single source:
   ```typescript
   export function getDisplayCommands(): CommandDef[] {
       return PRD_PIPELINE_TEMPLATES.map(t => ({
           id: t.id,
           command: t.display,
           description: t.description,
           timeout: t.timeout,
       }));
   }

   export function getRuntimeCommand(ctx: ScenarioContext, template: CommandTemplate): string {
       const bin = getRoko();
       const parts = [bin];
       if (template.needsModel && ctx.activeModel) {
           parts.push('--model', shellQuote(ctx.activeModel));
       }
       parts.push(template.subcommand);
       return parts.join(' ');
   }
   ```
3. Remove the duplicate `prdCommands(ctx)` function

### 7.4 Metrics tracking with AbortController cleanup

**File**: `demo/demo-app/src/lib/terminal-session.ts:443-474`

**Steps**:
1. Add `signal` parameter to `trackMetrics()`:
   ```typescript
   export function trackMetrics(
       handle: TerminalHandle,
       opts: {
           onCost?: (cost: string) => void;
           onTokens?: (tokens: string) => void;
           onGate?: (gate: GateResult) => void;
           signal?: AbortSignal;
       },
       intervalMs = 500,
   ): ReturnType<typeof setInterval> {
       const interval = setInterval(() => {
           // ... existing logic ...
       }, intervalMs);

       // Auto-cleanup on abort
       opts.signal?.addEventListener('abort', () => clearInterval(interval));

       return interval;
   }
   ```
2. In `showCmd()`, create an AbortController per command and pass `signal` to `trackMetrics()`

---

## 8. Generalization & Abstraction

### 8.1 Data-driven gate rungs

**Files**:
- `crates/roko-core/src/config/schema.rs` — add gate rung config
- `crates/roko-gate/src/` — read config instead of hardcoded rungs
- `crates/roko-cli/src/runner/event_loop.rs` — use config-driven rungs

**Steps**:
1. Add to config schema:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct GateRungConfig {
       pub name: String,
       pub command: String,
       pub timeout_secs: u64,
       pub required: bool,          // if false, failure is warning-only
       pub parallel_with: Vec<String>, // rungs that can run in parallel with this one
   }

   // In RokoConfig:
   pub struct GateConfig {
       pub rungs: Vec<GateRungConfig>,
       // ... existing fields ...
   }
   ```
2. Provide sensible defaults:
   ```rust
   impl Default for GateConfig {
       fn default() -> Self {
           Self {
               rungs: vec![
                   GateRungConfig { name: "compile".into(), command: "cargo check --workspace".into(),
                       timeout_secs: 120, required: true, parallel_with: vec![] },
                   GateRungConfig { name: "lint".into(), command: "cargo clippy --workspace --no-deps -- -D warnings".into(),
                       timeout_secs: 60, required: true, parallel_with: vec!["compile".into()] },
                   GateRungConfig { name: "test".into(), command: "cargo test --workspace".into(),
                       timeout_secs: 300, required: true, parallel_with: vec![] },
               ],
           }
       }
   }
   ```
3. In the gate dispatch logic, iterate `config.gate.rungs` instead of matching on `Rung` enum variants
4. Remove the 3 permanently skipped rungs (Symbol, PropertyTest, Integration) — they become opt-in via config

### 8.2 Workspace abstraction

**Files**:
- `crates/roko-core/src/workspace.rs` (new)
- All callers that use raw `PathBuf` for workspace paths

**Steps**:
1. Create `crates/roko-core/src/workspace.rs`:
   ```rust
   use std::path::{Path, PathBuf};

   pub struct Workspace {
       root: PathBuf,
       roko_dir: PathBuf,
   }

   impl Workspace {
       pub fn open(root: impl AsRef<Path>) -> Result<Self> {
           let root = root.as_ref().canonicalize()
               .context("failed to canonicalize workspace root")?;
           let roko_dir = root.join(".roko");
           if !roko_dir.exists() {
               bail!("not a roko workspace: {} (no .roko/ directory)", root.display());
           }
           Ok(Self { root, roko_dir })
       }

       pub fn create(root: impl AsRef<Path>) -> Result<Self> {
           let root = root.as_ref().to_path_buf();
           let roko_dir = root.join(".roko");
           fs::create_dir_all(&roko_dir)?;
           fs::create_dir_all(roko_dir.join("state"))?;
           fs::create_dir_all(roko_dir.join("runtime"))?;
           Ok(Self { root, roko_dir })
       }

       pub fn ephemeral(prefix: &str) -> Result<Self> {
           let dir = tempfile::TempDir::new()?;
           let root = dir.path().to_path_buf();
           let ws = Self::create(&root)?;
           std::mem::forget(dir); // Keep temp dir alive
           Ok(ws)
       }

       pub fn root(&self) -> &Path { &self.root }
       pub fn roko_dir(&self) -> &Path { &self.roko_dir }
       pub fn state_dir(&self) -> PathBuf { self.roko_dir.join("state") }
       pub fn plans_dir(&self) -> PathBuf { self.roko_dir.join("plans") }
       pub fn episodes_path(&self) -> PathBuf { self.roko_dir.join("episodes.jsonl") }
       pub fn signals_path(&self) -> PathBuf { self.roko_dir.join("signals.jsonl") }
       pub fn log_path(&self) -> PathBuf { self.roko_dir.join("roko.log") }
       pub fn config_path(&self) -> PathBuf { self.root.join("roko.toml") }
   }
   ```
2. Add `pub mod workspace;` to `crates/roko-core/src/lib.rs`
3. Progressively replace `workdir: &Path` parameters with `workspace: &Workspace` in callers

### 8.3 Relative section token budgets

**Files**: `crates/roko-compose/src/templates/common.rs` — budget definitions

**Steps**:
1. Define a budget struct that adapts to context window:
   ```rust
   pub struct AdaptiveBudget {
       pub fraction: f32,
       pub min_tokens: usize,
       pub max_tokens: usize,
   }

   impl AdaptiveBudget {
       pub fn compute(&self, total_context: usize) -> usize {
           let target = (total_context as f32 * self.fraction) as usize;
           target.clamp(self.min_tokens, self.max_tokens)
       }
   }
   ```
2. Replace hardcoded budget caps:
   ```rust
   // Before:
   .with_hard_cap(50_000) // plan_spec
   // After:
   .with_hard_cap(AdaptiveBudget {
       fraction: 0.25,
       min_tokens: 5_000,
       max_tokens: 50_000,
   }.compute(model_context_window))
   ```
3. Thread `model_context_window: usize` through `sections()` method of each template

---

## 9. Runner Architecture

### 9.1 Global gate semaphore singleton prevents true concurrency

**File**: `crates/roko-cli/src/runner/gate_dispatch.rs:20-26`

**Problem**: `OnceLock<Arc<Semaphore>>` initialized with `Semaphore::new(1)` — a process-level singleton. All gate evaluations funnel through a single permit, making `max_concurrent_plans: 4` (event_loop.rs:117) and `max_concurrent_tasks` from config effectively inert. If two `roko plan run` invocations happen simultaneously, one starves the other.

**Steps**:
1. Remove the `OnceLock` global from `gate_dispatch.rs`
2. Add `gate_concurrency: usize` to `RunConfig` (default: 1, configurable)
3. Create the semaphore in the event loop setup and pass it into `RunContext`:
   ```rust
   let gate_sem = Arc::new(Semaphore::new(config.gate_concurrency));
   ```
4. Replace `gate_semaphore()` calls in `spawn_gate` with `ctx.gate_sem.clone()`
5. Update `ExecutorConfig` to derive `gate_concurrency` from `max_concurrent_tasks`

### 9.2 Single agent_handle slot prevents multi-agent concurrency

**File**: `crates/roko-cli/src/runner/event_loop.rs:95-96, 2015-2023`

**Problem**: `RunContext` holds `agent_handle: &mut Option<AgentHandle>` — a single slot. When a second plan's `SpawnAgent` fires, the check `ctx.state.agent_active || ctx.agent_handle.is_some()` suppresses the spawn silently. Multi-plan parallelism is broken at the agent level.

**Steps**:
1. Change `agent_handle` from `Option<AgentHandle>` to `HashMap<String, AgentHandle>` keyed by `plan_id`
2. Update the duplicate-spawn guard at line 2015 to check per-plan:
   ```rust
   if ctx.agent_handles.contains_key(&plan_id) {
       debug!(plan_id, "agent already active for this plan — suppressing");
       return;
   }
   ```
3. Update `stop_active_agent` to operate on a specific plan's handle
4. Update `is_exited` handling to check the per-plan handle
5. Track `agent_active` per-plan in `RunState` (see 9.4)

### 9.3 FailPlan attributes failure to wrong plan

**File**: `crates/roko-cli/src/runner/event_loop.rs:2657-2676`

**Problem**: `FailPlan { plan_id, reason }` calls `ctx.state.task_failed()` which uses `ctx.state.plan_id` (current plan) not the `plan_id` from the action. TUI shows failure under wrong plan, cost attribution is wrong, and `RunReport.failure_reasons` omits these failures.

**Steps**:
1. In the `FailPlan` handler, use the action's `plan_id` parameter for all operations:
   ```rust
   ExecutorAction::FailPlan { plan_id, reason } => {
       warn!(plan_id = %plan_id, reason = %reason, "plan failed");
       ctx.state.record_task_failure(&plan_id, &reason);
       ctx.tui.task_completed(&plan_id, "plan", "failed");
   }
   ```
2. Add `record_task_failure(plan_id: &str, reason: &str)` to `RunState` that accepts explicit plan_id

### 9.4 Sentinel task resolution sorts by string ID, not DAG order

**File**: `crates/roko-cli/src/runner/event_loop.rs:1981-1989`

**Problem**: Ready task resolution uses `a.id.cmp(&b.id)` — lexicographic sort. Task IDs like `"T1"`, `"T2"`, `"T10"` sort as `T1, T10, T2`. Plans with named tasks like `"implement-api"` execute in alphabetical order, not definition order.

**Steps**:
1. Add `sequence: usize` field to `TaskDef` (set from Vec index during parsing)
2. In `task_parser.rs`, when building `TaskDef` from TOML, set `sequence` from the array index:
   ```rust
   for (i, task) in raw_tasks.iter().enumerate() {
       let mut td = TaskDef::from(task);
       td.sequence = i;
       tasks.insert(td.id.clone(), td);
   }
   ```
3. Replace `a.id.cmp(&b.id)` with `a.sequence.cmp(&b.sequence)` in the sentinel resolution

### 9.5 agent_output grows unbounded within a task

**File**: `crates/roko-cli/src/runner/agent_events.rs:103, 114, 131`

**Problem**: `state.agent_output` accumulates all text deltas, tool markers, and truncated tool outputs with no cap within a task. A long-running agent can accumulate 10-100 MB. Only 2000 chars are ever used (for replan context at event_loop.rs:850).

**Steps**:
1. Add a constant `const MAX_AGENT_OUTPUT: usize = 32_768;` (32 KB) to `agent_events.rs`
2. In `handle_agent_event`, after each `push_str`, check and trim:
   ```rust
   AgentEvent::MessageDelta { text } => {
       state.agent_output.push_str(text);
       if state.agent_output.len() > MAX_AGENT_OUTPUT {
           let trim_point = state.agent_output.len() - MAX_AGENT_OUTPUT / 2;
           state.agent_output = format!("[...truncated...]\n{}",
               &state.agent_output[trim_point..]);
       }
   }
   ```
3. This preserves the tail (most recent output) which is what replan context needs

### 9.6 `started_at_ms` in snapshot is elapsed-since-start, not epoch timestamp

**File**: `crates/roko-cli/src/runner/event_loop.rs:1640`

**Problem**: `started_at_ms: state.started_at.elapsed().as_millis() as u64` — this is a duration, not a timestamp. Field name implies epoch ms. Cross-run comparisons and dashboards will misinterpret this.

**Steps**:
1. Add `start_timestamp_ms: u64` to `RunState`, set at initialization:
   ```rust
   start_timestamp_ms: std::time::SystemTime::now()
       .duration_since(std::time::UNIX_EPOCH)
       .unwrap_or_default()
       .as_millis() as u64,
   ```
2. Use `state.start_timestamp_ms` in the snapshot instead of computing from `Instant::elapsed`

### 9.7 `iteration` is a single field shared across plans

**File**: `crates/roko-cli/src/runner/state.rs:72, 403-408`

**Problem**: `state.iteration` tracks retry count for the current task but is a single `u32`. In multi-plan runs, gate results for earlier plans carry the wrong attempt number because `iteration` was overwritten by the most-recently-dispatched task.

**Steps**:
1. Replace `iteration: u32` with `iterations: HashMap<String, u32>` keyed by `"{plan_id}:{task_id}"`
2. Update `reset_for_task` to initialize the entry: `self.iterations.insert(key, 0);`
3. Update `current_attempt_ref()` to read from the map using current plan_id + task_id

### 9.8 MCP config hardcoded `None` in `plan run`

**File**: `crates/roko-cli/src/commands/plan.rs:460`

**Problem**: `RunConfig` is constructed with `mcp_config: None`, ignoring `roko_config.agent.mcp_config`. The `AgentDispatchRequest` reads from `ctx.config.mcp_config.clone()` which is always `None`. MCP tools (code intelligence, GitHub, Slack) are never available to agents in `plan run`.

**Steps**:
1. In `cmd_plan`, after loading `roko_config`, resolve MCP config:
   ```rust
   let mcp_config = roko_config.agent.mcp_config.as_ref()
       .map(|p| workdir.join(p))
       .filter(|p| p.exists());
   ```
2. Set `mcp_config` in `RunConfig` construction
3. This is the stated "MCP config passthrough" capability — currently broken

### 9.9 Dream consolidation runs when `roko_config` is `None` (inverted logic)

**File**: `crates/roko-cli/src/runner/event_loop.rs:3077-3091`

**Problem**: `let Some(roko_config) = config.roko_config.as_ref() else { run_dream_consolidation(config).await; return; };` — the `else` branch runs dream consolidation when there's NO config. This triggers a 120-second blocking consolidation on every headless CI run.

**Steps**:
1. Invert the logic:
   ```rust
   let Some(roko_config) = config.roko_config.as_ref() else {
       return; // no config — skip consolidation
   };
   if !roko_config.learning.dream_on_completion {
       return;
   }
   run_dream_consolidation(config).await;
   ```

### 9.10 `RunnerFailureKind::Permanent` is classified as retryable

**File**: `crates/roko-cli/src/runner/types.rs:45-50`

**Problem**: `is_retryable()` returns `true` for `Permanent`, making "permanent" failures retry with zero cooldown. Tasks that will never succeed are retried blindly at full cost.

**Steps**:
1. Remove `Self::Permanent` from the `is_retryable()` match:
   ```rust
   pub const fn is_retryable(self) -> bool {
       matches!(self, Self::Transient | Self::Structural | Self::Unknown)
   }
   ```
2. Update the test at line 1585 that asserts `permanent.is_retryable()`
3. Consider renaming `Permanent` to `NonRetryable` for clarity

### 9.11 `dispatch_action` error paths swallow `apply_event(Fatal)` result

**File**: `crates/roko-cli/src/runner/event_loop.rs:2068-2074, 2164-2170, 2389-2393`

**Problem**: Three locations use `let _ = ctx.executor.apply_event(plan_id, &ExecutorEvent::Fatal(...))`. If the executor rejects the transition (plan already terminal), the plan hangs forever — the event loop never sees it become terminal. The plan timeout is the only recovery.

**Steps**:
1. At each of the 3 locations, check the result:
   ```rust
   if let Err(e) = ctx.executor.apply_event(plan_id, &ExecutorEvent::Fatal(reason)) {
       error!(plan_id, error = %e, "failed to apply Fatal event — plan may hang");
       // Force-mark plan as terminal in state
       ctx.state.force_plan_terminal(plan_id);
   }
   ```
2. Add `force_plan_terminal(plan_id: &str)` to `RunState` as a last-resort escape hatch

### 9.12 Per-turn budget exceeded is only warned, not enforced

**File**: `crates/roko-cli/src/runner/event_loop.rs:416-426`

**Problem**: Per-turn budget check logs a warning but does not stop the agent or apply a Fatal event. The per-plan budget at line 2041 does abort, but per-turn is purely advisory. Users relying on `max_turn_usd` as cost control are silently over-charged.

**Steps**:
1. After the warning, kill the agent and apply Fatal:
   ```rust
   if max_turn > 0.0 && state.cost_usd > max_turn {
       warn!(...);
       stop_active_agent(ctx).await;
       let _ = ctx.executor.apply_event(
           &state.plan_id,
           &ExecutorEvent::Fatal(format!(
               "turn cost ${:.2} exceeded per-turn limit ${:.2}",
               state.cost_usd, max_turn)),
       );
   }
   ```
2. Add a `budget.enforce_per_turn: bool` config option (default: `true`) for users who want warning-only behavior

### 9.13 Plan timeout can fire twice

**File**: `crates/roko-cli/src/runner/event_loop.rs:1009-1051`

**Problem**: Branch 5 (plan timeout) fires in the `select!`, AND a post-select check at line 1039 (`if Instant::now() >= plan_deadline`) can also trigger `handle_plan_timeout`. If a different branch fires after the deadline passes, the post-select check calls `handle_plan_timeout`, then Branch 5 fires next iteration — calling it again. `shutdown_subsystems` and duplicate `run.completed` events result.

**Steps**:
1. Add a `timed_out: bool` flag to the loop state
2. After the first `handle_plan_timeout` call, set `timed_out = true`
3. Guard both timeout paths:
   ```rust
   if !timed_out && tokio::time::Instant::now() >= plan_deadline {
       handle_plan_timeout(...).await?;
       timed_out = true;
   }
   ```

### 9.14 Extension chain hooks hardcode `role = "implementer"`

**File**: `crates/roko-cli/src/runner/event_loop.rs:2843-2845, 2876-2878`

**Problem**: Both `fire_pre_inference_hook` and `fire_post_inference_hook` set `role: "implementer".to_string()` regardless of actual task role. Extension hooks gating on role receive incorrect information.

**Steps**:
1. Add `role: &str` parameter to both functions
2. Replace `"implementer".to_string()` with `role.to_string()`
3. At the call sites (~line 2226), pass the actual `role` variable

### 9.15 Feedback facade spawns unbounded tasks per event

**File**: `crates/roko-cli/src/runner/event_loop.rs:1466-1479`

**Problem**: Every runner event that maps to a `FeedbackEvent` spawns a new `tokio::spawn`. For a 100-task run with retries, this generates hundreds of fire-and-forget tasks with no back-pressure. On shutdown, these tasks are abandoned mid-flight.

**Steps**:
1. Replace unbounded spawns with a `JoinSet` with capacity:
   ```rust
   let mut feedback_tasks = JoinSet::new();
   // ... in the event handler:
   if feedback_tasks.len() >= 32 {
       feedback_tasks.join_next().await; // back-pressure
   }
   feedback_tasks.spawn(async move { facade.on_event(&feedback).await });
   ```
2. In `shutdown_subsystems`, drain the JoinSet:
   ```rust
   while feedback_tasks.join_next().await.is_some() {}
   ```

---

## 10. Compose & Template System

### 10.1 Section budget caps only cover 5 of 11 section names

**File**: `crates/roko-compose/src/system_prompt_builder.rs:823-832`

**Problem**: `section_budget_cap()` only matches `conventions`, `tool_instructions`, `anti_patterns`, `domain_context`, `context_layer`, `pheromone_signals`, `gate_feedback`, `relevant_techniques`. The sections `role_identity`, `task_context`, `affect_guidance`, and `tool_hints` fall through to `None` — no cap even with a `PromptBudget` set. A large `role_identity` (e.g., huge AGENTS.md) can exhaust the entire token budget.

**Steps**:
1. Add caps for the 4 missing sections:
   ```rust
   "role_identity" => Some(budget.role_identity.unwrap_or(8_000)),
   "task_context" => Some(budget.plan),
   "affect_guidance" => Some(budget.instructions),
   "tool_hints" => Some(budget.skills),
   ```
2. Add `role_identity: Option<usize>` to `PromptBudget` with default `8_000`

### 10.2 O(N² log N) section measurement in `build_with_counter`

**File**: `crates/roko-compose/src/system_prompt_builder.rs:383-423`

**Problem**: The selection loop calls `candidate_fits` for each candidate, which calls `assemble_selected_sections` (triggering a full sort + reassembly) to measure token count. With N sections and binary-search truncation, this is O(N² log N).

**Steps**:
1. Cache the assembled string of already-selected sections
2. On each `candidate_fits` probe, append only the candidate section to the cached string for measurement:
   ```rust
   let mut cached_assembly = String::new();
   for &idx in &kept {
       cached_assembly += &rendered_sections[idx].content;
       cached_assembly += "\n\n";
   }
   // For candidate check:
   let probe = format!("{}{}", cached_assembly, rendered_sections[candidate].content);
   let fits = token_counter.count(&probe) <= budget;
   ```
3. Update `cached_assembly` incrementally when a candidate is accepted

### 10.3 Three parallel section-name registries must stay in sync

**File**: `crates/roko-compose/src/system_prompt_builder.rs` — lines 1192-1206 (`section_order_rank`), 823-832 (`section_budget_cap`), 1076-1095 (`render_section`)

**Problem**: Three `match` blocks enumerate section names as string literals. Adding a section requires updating all three. `tool_hints` is already missing from `section_order_rank` (gets rank 11, sorted last) and `section_budget_cap` (no cap).

**Steps**:
1. Define a `SectionSpec` struct:
   ```rust
   struct SectionSpec {
       name: &'static str,
       order_rank: u32,
       budget_source: BudgetField,
       cache_layer: CacheLayer,
   }
   ```
2. Create a `const SECTION_SPECS: &[SectionSpec]` table
3. Derive `section_order_rank`, `section_budget_cap`, and `render_section` from this single table
4. Add a compile-time test that asserts all known section names are in the table

### 10.4 DRY violation — `agents_instructions` section identical across all templates

**Files**: `crates/roko-compose/src/templates/implementer.rs:80-85`, `strategist.rs:79-85`, and every other template

**Problem**: Every template pushes `agents_instructions` with identical `SectionPriority::Critical`, `CacheLayer::Role`, `Placement::Start`. Copy-paste risk on any change.

**Steps**:
1. Add to `templates/common.rs`:
   ```rust
   pub fn agents_instructions_section(agents_md: &str) -> PromptSection {
       PromptSection::new("agents_instructions", agents_md)
           .with_priority(SectionPriority::Critical)
           .with_cache_layer(CacheLayer::Role)
           .with_placement(Placement::Start)
   }
   ```
2. Replace each template's duplicate with `sections.push(common::agents_instructions_section(&input.agents_md));`

### 10.5 Conflicting budget limits on `relevant_techniques`

**File**: `crates/roko-compose/src/system_prompt_builder.rs:107, 743-806`

**Problem**: `RELEVANT_TECHNIQUES_TOKEN_BUDGET = 500` limits the greedy-fill loop. Then `apply_budget_profile` applies `budget.skills` (e.g., 8,000 chars) as a hard cap. The inner 500-token limit always wins, making the budget profile cap irrelevant. The two limits operate on different units (tokens vs chars).

**Steps**:
1. Remove `RELEVANT_TECHNIQUES_TOKEN_BUDGET` constant
2. Use `budget.skills / 4` as the greedy loop limit (converting chars to approximate tokens):
   ```rust
   let skill_token_budget = self.budget_profile
       .map(|b| b.skills / 4)
       .unwrap_or(500);
   ```
3. Remove the `apply_budget_profile` override for `relevant_techniques` since the inner limit governs it

---

## 11. Serve / SSE / WebSocket

### 11.1 Health endpoint always returns HTTP 200 regardless of status

**File**: `crates/roko-serve/src/routes/status/health.rs:43-68`

**Problem**: When `status = "down"` (all providers offline), the HTTP status code is still `200 OK`. Load balancers, Kubernetes liveness probes, and uptime monitors will see a healthy endpoint when the server is degraded.

**Steps**:
1. Return appropriate HTTP status codes:
   ```rust
   let http_status = match status {
       "down" => axum::http::StatusCode::SERVICE_UNAVAILABLE,
       "degraded" => axum::http::StatusCode::OK,  // or 207
       _ => axum::http::StatusCode::OK,
   };
   (http_status, Json(json!({ "status": status, ... })))
   ```

### 11.2 Blocking `parking_lot::RwLock` in async `relay_health` handler

**File**: `crates/roko-serve/src/routes/status/health.rs:73`

**Problem**: `state.relay_health.read()` is a synchronous, blocking call in a Tokio async context. Under write contention, this blocks the Tokio worker thread, preventing other tasks from running. The `health()` handler already uses `try_read()` — `relay_health` doesn't.

**Steps**:
1. Use `try_read()` with fallback, consistent with the `health()` handler:
   ```rust
   let health = state.relay_health.try_read()
       .map(|r| r.clone())
       .unwrap_or_default();
   ```
2. Or convert to `tokio::sync::RwLock` and `.read().await`

### 11.3 SSE keep-alive interval not configurable

**File**: `crates/roko-serve/src/routes/sse.rs:63`

**Problem**: `KeepAlive::default()` sends pings every 15 seconds. Many proxies (Railway: 30s, Nginx: 60s) have different timeouts. No way to configure this. The empty ping (`:`-comment) doesn't trigger `message` events in some SSE clients.

**Steps**:
1. Add `sse_keepalive_secs: u64` to `ServeConfig` (default: 8)
2. Use configurable keep-alive:
   ```rust
   Sse::new(stream)
       .keep_alive(KeepAlive::new()
           .interval(Duration::from_secs(config.sse_keepalive_secs))
           .text("keepalive"))
   ```

### 11.4 SSE replay materializes full ring buffer in memory

**File**: `crates/roko-serve/src/routes/sse.rs:37-44`

**Problem**: `state_hub.replay_from(last_event_id)` returns a `Vec<Envelope>` — the full ring buffer — materialized before streaming. A client reconnecting from `seq=0` gets hundreds of events allocated at once.

**Steps**:
1. Add a limit parameter:
   ```rust
   let replay = state.state_hub.replay_from(last_event_id)
       .into_iter()
       .take(query.limit.unwrap_or(256))
       .map(|envelope| { ... });
   ```
2. Or add `replay_from_bounded(cursor, limit)` to StateHub

### 11.5 WebSocket `back_pressure` field parsed but never used

**File**: `crates/roko-serve/src/routes/ws.rs:90, 121-124`

**Problem**: `_back_pressure` is set from client messages but never consulted. `Coalesce` and `ResumeRequired` modes are dead code. Clients requesting `"back_pressure": "coalesce"` silently get `at_most_once`.

**Steps**:
1. Either implement `Coalesce` mode (deduplicate same-type events before send), or
2. Remove the `BackPressureMode` enum and return a protocol error for unsupported modes:
   ```rust
   if cmd.back_pressure.is_some() {
       ws_send(&mut ws, json!({"error": "back_pressure modes not yet supported"})).await;
   }
   ```

### 11.6 13 `RwLock<HashMap>` fields in `AppState` with no lock ordering discipline

**File**: `crates/roko-serve/src/state.rs:380-434`

**Problem**: 13 separate `RwLock<HashMap<...>>` fields. Some handlers hold multiple locks simultaneously (e.g., `discovered_agents` + `heartbeats`). No documented lock acquisition order — lock-inversion deadlocks possible.

**Steps**:
1. Document lock ordering in a comment at the top of `AppState`:
   ```rust
   // Lock acquisition order (acquire outer before inner):
   // 1. active_runs  2. active_plans  3. operations  4. templates
   // 5. deployments  6. discovered_agents  7. heartbeats  ...
   ```
2. For read-heavy maps, replace with `DashMap` (concurrent hashmap, no global lock):
   ```rust
   pub discovered_agents: DashMap<String, DiscoveredAgent>,
   pub aggregator_cache: DashMap<String, CachedJsonValue>,
   ```
3. Priority conversions: `discovered_agents`, `aggregator_cache`, `heartbeats` (read on every request cycle)

---

## 12. Learning Subsystem

### 12.1 LinUCB state not persisted in CascadeSnapshot — routing quality resets on restart

**File**: `crates/roko-learn/src/cascade/persistence.rs` — `CascadeSnapshot` struct

**Problem**: `CascadeSnapshot` persists `model_slugs`, `role_table`, `confidence_stats`, `total_observations`, `stage_transitions`. But NOT the `LinUCBRouter` state (A matrices and b vectors). After restart, if `total_observations >= 200` (stage 3), the router enters UCB mode with empty arm parameters — effectively random routing.

**Steps**:
1. Add a `LinUCBSnapshot` struct:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct LinUCBSnapshot {
       pub a_matrices: Vec<Vec<Vec<f64>>>,  // per-arm A matrix
       pub b_vectors: Vec<Vec<f64>>,         // per-arm b vector
       pub t: usize,                         // total observations
   }
   ```
2. Add `linucb_state: Option<LinUCBSnapshot>` to `CascadeSnapshot` with `#[serde(default)]`
3. Implement `LinUCBRouter::snapshot()` and `LinUCBRouter::restore(snap)`
4. Call them in the cascade router's `save` and `load` paths

### 12.2 Nested mutex acquisition in `observe_internal` — 3-lock chain

**File**: `crates/roko-learn/src/cascade_router.rs:1212-1281`

**Problem**: `observe_internal` acquires `stage_tracking` lock, then `confidence_stats` lock (nested), then `linucb.update_features` (which acquires its own internal mutex). This 3-lock chain creates priority inversion risk under concurrent observations.

**Steps**:
1. Split into two phases:
   ```rust
   fn observe_internal(&self, ...) {
       // Phase 1: update stats (single lock)
       {
           let mut stats = self.confidence_stats.lock();
           let entry = stats.entry(slug.clone()).or_default();
           // ... update stats ...
       } // lock dropped

       // Phase 2: check stage transition (single lock)
       {
           let mut tracking = self.stage_tracking.lock();
           // ... check transitions ...
       } // lock dropped

       // Phase 3: update LinUCB (internal lock)
       self.linucb.update_features(context_vec, model_idx, reward);
   }
   ```

### 12.3 Episode dual identity fields — `id` and `episode_id` both exist

**File**: `crates/roko-learn/src/episode_logger.rs:170-193`

**Problem**: Both `id` (hash-derived) and `episode_id` (stable identifier) exist. `Episode::new()` sets `id` via `derive_id()` but `episode_id = String::new()`. The `same_episode` function checks both, but old episodes without `id` (deserialized from before the field was added) cannot be deduplicated.

**Steps**:
1. In `Episode::new()`, set both to the same value:
   ```rust
   let derived = derive_id(plan_id, task_id, ...);
   Episode { id: derived.clone(), episode_id: derived, ... }
   ```
2. Add a migration in `read_all`:
   ```rust
   for ep in &mut episodes {
       if ep.id.is_empty() && !ep.episode_id.is_empty() {
           ep.id = ep.episode_id.clone();
       }
   }
   ```
3. Mark `episode_id` as `#[deprecated]` with a doc comment pointing to `id`

### 12.4 `CostsLog::append` opens, writes, syncs, and closes file per record

**File**: `crates/roko-learn/src/costs_log.rs:66-80`

**Problem**: Each `append` call does `open → write → fsync → close`. Under high agent throughput (many concurrent tasks), this serializes syscalls and adds 1-10ms per turn on spinning disk.

**Steps**:
1. Add a `append_queued` mode:
   ```rust
   pub async fn append_queued(&self, record: CostRecord) {
       self.queue.lock().await.push(record);
       if self.queue.lock().await.len() >= self.batch_size {
           self.flush_queue().await;
       }
   }

   pub async fn flush_queue(&self) {
       let records: Vec<_> = std::mem::take(&mut *self.queue.lock().await);
       if records.is_empty() { return; }
       self.append_all(&records).await.ok();
   }
   ```
2. Wire `flush_queue` into the event loop's periodic flush branch
3. Keep single-record `append` as the crash-safe path

### 12.5 O(N²) importance scoring with no history cap

**File**: `crates/roko-learn/src/episode_logger.rs:666-684`

**Problem**: `prioritize_by_importance` calls `importance_score(episode, history)` for each episode against the full history. `surprisal_score` and `information_gain_score` iterate all of `history`. For 1,000 episodes, this is 1,000,000 operations.

**Steps**:
1. Add a `history_limit` parameter (default: 256):
   ```rust
   pub fn prioritize_by_importance<'a>(
       episodes: &'a [Episode],
       history: &[Episode],
       history_limit: usize,
   ) -> Vec<&'a Episode> {
       let recent_history = if history.len() > history_limit {
           &history[history.len() - history_limit..]
       } else { history };
       // ... use recent_history instead of history
   }
   ```
2. Update callers to pass 256 as the default

---

## 13. Config System Deep Issues

### 13.1 `ROKO__*` env var overrides documented but never implemented

**File**: `crates/roko-core/src/config/loader.rs:10, 29`

**Problem**: Module docstring says `ROKO__*` provides field-level overrides. `apply_env_overrides: bool` is defined and defaults to `true`. But `apply_process_env()` only handles 12 named env vars (`ROKO_MODEL`, `ROKO_BACKEND`, etc.). There is no hierarchical `ROKO__SECTION__FIELD` override system.

**Steps**:
1. Either implement the `ROKO__*` scheme:
   ```rust
   fn apply_hierarchical_env_overrides(&mut self) {
       for (key, value) in std::env::vars() {
           if let Some(path) = key.strip_prefix("ROKO__") {
               let parts: Vec<&str> = path.split("__").collect();
               self.set_field_by_path(&parts, &value);
           }
       }
   }
   ```
2. Or update the documentation to list the exact 12 supported env vars and remove the misleading `ROKO__*` claim

### 13.2 Deprecated `load_config` runs different code path than `load_config_unified`

**File**: `crates/roko-core/src/config/mod.rs:96-185`

**Problem**: Deprecated `load_config` / `load_config_strict` use `load_config_impl`, which does NOT merge global config, does NOT apply env overrides, does NOT walk ancestors. Callers still using the deprecated functions get silently different behavior.

**Steps**:
1. Make deprecated functions delegate to the new implementation:
   ```rust
   #[deprecated(note = "Use load_config_with_options instead")]
   pub fn load_config(workdir: &Path) -> Result<RokoConfig> {
       load_config_with_options(workdir, &LoadOptions::default())
   }
   ```
2. Search for remaining callers: `grep -rn 'load_config(' crates/ | grep -v target | grep -v deprecated`
3. Migrate all callers to `load_config_with_options`

### 13.3 `merge_global_into` only merges 3 of 20+ config sections

**File**: `crates/roko-core/src/config/loader.rs:358-400`

**Problem**: Only `providers`, `models`, and 2 `agent` fields are merged from `~/.roko/config.toml`. A global `[budget]`, `[gates]`, `[serve]`, `[conductor]` are silently ignored. Users cannot set global defaults for most settings.

**Steps**:
1. Extend `merge_global_into` with a "global fills project gap" policy for key sections:
   ```rust
   // Merge budget defaults
   if config.budget.max_plan_usd == 0.0 && global.budget.max_plan_usd > 0.0 {
       config.budget.max_plan_usd = global.budget.max_plan_usd;
   }
   // Merge gate defaults
   if config.gates == GateConfig::default() && global.gates != GateConfig::default() {
       config.gates = global.gates;
   }
   // ... for each section that makes sense as a global default
   ```
2. Document which sections are merged in the module docstring

### 13.4 Config diagnostics run on post-env-override config — misleading messages

**File**: `crates/roko-core/src/config/loader.rs:126, 209-262`

**Problem**: `collect_diagnostics` runs on the `migrated` config (after env var application). A diagnostic like "model 'opus' references provider 'anthropic' which is not configured" may be wrong if `ROKO_PROVIDER=anthropic` was set. Conversely, env-only providers won't be validated.

**Steps**:
1. Add env-var context to diagnostic messages:
   ```rust
   if env_overrides_applied {
       diagnostic.note = Some("config was modified by env vars; some diagnostics may be stale");
   }
   ```
2. Or run structural validation on raw config and availability validation on migrated config separately

### 13.5 Env var interpolation (`${VAR}`) only works in provider fields

**File**: `crates/roko-core/src/config/schema.rs:498-520`

**Problem**: `interpolate_env_vars_with` only expands `${VAR}` in `provider.base_url`, `provider.api_key_env`, `provider.command`, `provider.extra_headers`. A user writing `command = "${CLAUDE_PATH}"` in `[agent]` or `slug = "${MODEL_SLUG}"` in `[models]` gets the literal string.

**Steps**:
1. Either document that interpolation only applies to provider fields, or
2. Implement a generic pass that walks all string fields:
   ```rust
   fn interpolate_all_strings(config: &mut RokoConfig, env_fn: &dyn Fn(&str) -> Option<String>) {
       let json = serde_json::to_value(&*config).unwrap();
       let interpolated = walk_and_interpolate(json, env_fn);
       *config = serde_json::from_value(interpolated).unwrap();
   }
   ```

---

## 14. Pipeline Run Findings (from real gpt54-mini end-to-end run)

These findings come from analyzing a successful `roko prd pipeline` run (BTC Funding Alert CLI, gpt54-mini, 4/4 tasks, 161s). They represent systemic issues that explain why the pipeline produces suboptimal results even when it "succeeds."

### 14.1 ImplementerTemplate is built but never wired to runtime dispatch — THE ROOT CAUSE

**Files**: `crates/roko-compose/src/templates/implementer.rs` (has workspace_map, tasks, brief, preflight), `crates/roko-cli/src/runner/dispatch_helpers.rs:83` (uses `RoleSystemPromptSpec` instead)

**Problem**: `ImplementerTemplate` has 11 rich sections including `workspace_map` (crate tree), `tasks` (full tasks.toml), `brief` (strategist summary), `preflight` (tool check), `registry` (completed plans). But the dispatch chain uses `RoleSystemPromptSpec` → `SystemPromptBuilder` which has NONE of these. The implementer agent never sees what other tasks produced, what crates exist, or what the workspace looks like. This is why:
- T2 duplicated roko-core types instead of importing them
- No awareness of cross-task dependencies
- No workspace structure context

**Steps**:
1. In `dispatch_helpers.rs:83` (`build_system_prompt`), wire `ImplementerTemplate`:
   ```rust
   let template_input = ImplementerInput {
       agents_md: agents_md.clone(),
       plan_spec: plan_spec_content,
       brief: strategist_brief,
       tasks: tasks_toml_content,
       workspace_map: generate_workspace_map(&workdir),
       // ... fill from RunContext
   };
   let sections = ImplementerTemplate::new().sections(&template_input);
   builder.add_sections(sections);
   ```
2. Generate `workspace_map` by walking the crate directory tree at dispatch time
3. Read `tasks.toml` and inject as the `tasks` field
4. This single fix addresses 14.2, 14.3, and 14.4 below

### 14.2 PRD content never passed to implementer agents

**File**: `crates/roko-cli/src/task_parser.rs:455` (`build_prompt`)

**Problem**: `build_prompt` only uses `TaskDef` fields (title, description, files, context, verify). The PRD that specified `--symbol`, `--window`, `--threshold`, `--dry-run`, Binance API requirements is never seen by the implementer. Result: hardcoded main.rs with no CLI arg parsing or HTTP client.

**Steps**:
1. Add a `prd_excerpt: Option<String>` field to `TaskDef` or the dispatch context
2. In `plan_loader.rs`, read the PRD from `.roko/prd/published/{slug}.md` and attach a truncated excerpt (first 2000 chars or the requirements section) to the task context
3. In `build_prompt`, append the PRD excerpt:
   ```rust
   if let Some(prd) = &self.prd_excerpt {
       prompt.push_str("\n## PRD Requirements (source document)\n");
       prompt.push_str(prd);
   }
   ```

### 14.3 Scaffold creates Cargo.toml with no `[dependencies]`

**File**: `crates/roko-cli/src/runner/plan_loader.rs:156-164`

**Problem**: Generated Cargo.toml is a minimal stub (name, version, edition) with no dependencies. The agent doesn't know it needs to add dependencies. Result: roko-cli had no dependency on roko-core, types were duplicated.

**Steps**:
1. If a task's `depends_on` references tasks in other crates, add workspace dependencies to the scaffold:
   ```rust
   fn scaffold_cargo_toml(crate_name: &str, deps: &[String]) -> String {
       let mut toml = format!(
           "[package]\nname = \"{crate_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"
       );
       if !deps.is_empty() {
           toml.push_str("\n[dependencies]\n");
           for dep in deps {
               toml.push_str(&format!("{dep} = {{ path = \"../{dep}\" }}\n"));
           }
       }
       toml
   }
   ```
2. In `scaffold_missing_crates`, analyze the tasks to determine inter-crate dependencies from `files` and `depends_on`

### 14.4 No cross-task output injection

**File**: `crates/roko-cli/src/task_parser.rs:455`

**Problem**: T2 doesn't know what T1 produced. `depends_on` only gates execution order — it doesn't inject T1's output files into T2's prompt. The agent starts blind.

**Steps**:
1. After each task completes, record a summary of files modified:
   ```rust
   // In event_loop.rs, after task completion:
   let modified_files = git_diff_names_since_task_start(&workdir);
   ctx.state.task_outputs.insert(task_id, modified_files);
   ```
2. In `build_prompt`, for each `depends_on` task, inject:
   ```rust
   prompt.push_str(&format!(
       "\n## Completed by prior task {dep_id}:\nFiles created/modified: {}\n",
       ctx.state.task_outputs[dep_id].join(", ")));
   ```
3. Optionally inline the content of key files created by prior tasks using `context.read_files`

### 14.5 Cost tracking $0.00 — `model_profile` not passed to ToolLoop

**File**: `crates/roko-agent/src/provider/openai_compat.rs:378`

**Problem**: `ToolLoop::new()` is called without `.with_model_profile(model.clone())`. Per-turn cost accumulation never fires. `fill_cost_from_profile` (dispatch_v2.rs:924) is a recovery mechanism but may also fail if token field names differ between models (`prompt_tokens` vs `input_tokens`).

**Steps**:
1. Add `.with_model_profile(model.clone())` to the ToolLoop construction:
   ```rust
   let tool_loop = ToolLoop::new(translator, dispatcher, backend)
       .with_max_iterations(tool_loop_max_iterations())
       .with_context_token_limit(...)
       .with_model_profile(model.clone());  // ADD THIS
   ```
2. In `translate/openai.rs:parse_usage_observation`, add fallback for `input_tokens`:
   ```rust
   let input = usage.get("prompt_tokens")
       .or_else(|| usage.get("input_tokens"))
       .and_then(|v| v.as_u64());
   ```

### 14.6 Raw JSON `{"tool_uses":[...]}` leaks to stdout during `prd plan`

**File**: `crates/roko-cli/src/prd.rs:1054`

**Problem**: `run_agent_capture_logged` uses `echo_output: true`, which prints the agent's raw text output before TOML extraction runs. Failed tool call attempts appear as literal JSON text in the output.

**Steps**:
1. Change line 1054 from `run_agent_capture_logged` to `run_agent_capture_silent`:
   ```rust
   let (exit_code, output) = run_agent_capture_silent(
       AgentExecOpts { ... },
       AgentExecEpisode { ... },
   ).await?;
   ```
2. The retry path at line 1148 already correctly uses the silent variant

### 14.7 `model_hint` survives validation despite "NEVER set model_hint" rule

**File**: `crates/roko-cli/src/prd.rs:2040-2058`

**Problem**: `validate_and_fix_generated_plan` removes `model_hint` only when the model is NOT in config. If `claude-sonnet-4-6` is a known model (which it is), the hint is kept. This contradicts the prompt instruction to never set model_hint.

**Steps**:
1. Unconditionally strip `model_hint` from generated plans:
   ```rust
   if task.contains_key("model_hint") {
       let hint = task.get("model_hint").and_then(|v| v.as_str()).unwrap_or("");
       eprintln!("info: removing model_hint '{hint}' from generated plan (runtime selects model)");
       task.remove("model_hint");
   }
   ```

### 14.8 `max_loc` is advisory text only — no gate enforcement

**Files**: `crates/roko-cli/src/dispatch_helpers.rs:68-72`, `crates/roko-gate/src/`

**Problem**: `max_loc` is a soft prompt hint ("roughly N lines"). No gate reads `task_def.max_loc` and fails the task if exceeded. T2 wrote 248 lines despite max_loc=150. The word "roughly" weakens the constraint further.

**Steps**:
1. Add a `DiffLocGate` to `crates/roko-gate/src/`:
   ```rust
   pub struct DiffLocGate {
       pub max_loc: usize,
       pub tolerance: f32,  // 1.5 = allow 50% over
   }
   ```
2. Wire into the gate dispatch for tasks that have `max_loc` set
3. Change the prompt text from "roughly" to "strictly under" when a gate enforces it

### 14.9 Dream path double-nesting creates `.roko/.roko/`

**File**: `crates/roko-cli/src/runner/event_loop.rs:3111`

**Problem**: `DreamRunner::new(workdir.join(".roko"), ...)` passes `.roko/` as the workspace root. `DreamRunner` internally does `self.workdir.join(".roko")`, producing `.roko/.roko/dreams/` and looking for episodes at the wrong path (0 episodes processed).

**Steps**:
1. Fix the call:
   ```rust
   // Before
   let dream_runner = DreamRunner::new(workdir.join(".roko"), dream_config);
   // After
   let dream_runner = DreamRunner::new(workdir.clone(), dream_config);
   ```

### 14.10 Gate rung IDs use u32::MAX sentinels — breaks display

**Files**: `crates/roko-cli/src/runner/gate_dispatch.rs:161,204`, `crates/roko-cli/src/runner/merge.rs:516`

**Problem**: Plan-verify uses `rung: u32::MAX`, merge uses `rung: u32::MAX - 1`. These flow into `gate-thresholds.json` and display as `rung 4294967295: pass_rate=100%`.

**Steps**:
1. Define named constants:
   ```rust
   pub const RUNG_PLAN_VERIFY: u32 = 1000;
   pub const RUNG_MERGE: u32 = 1001;
   ```
2. Replace `u32::MAX` and `u32::MAX - 1` with these constants
3. In status display, map these IDs to human-readable names

### 14.11 Gate threshold schema mismatch between writer and reader

**Files**: `crates/roko-cli/src/runner/persist.rs` (writes `GateThresholdStats`), `crates/roko-gate/src/adaptive_threshold.rs` (reads `RungStats`)

**Problem**: The runner writes `{"pass_count": N, "total_count": N, "ema_pass_rate": 1.0}` but `cmd_status` reads with `RungStats` expecting `total_observations`, `consecutive_passes`, `cusum_high`. Field names don't match, so `total_observations` always deserializes as 0.

**Steps**:
1. Unify on one struct. Add `#[serde(alias = "total_count")]` to `total_observations`:
   ```rust
   #[serde(default, alias = "total_count")]
   pub total_observations: u64,
   ```
2. Or migrate the runner to write `RungStats` directly instead of `GateThresholdStats`

### 14.12 Episode data all zeros (duration, tokens, cost)

**File**: `crates/roko-cli/src/runner/event_loop.rs:1504-1519`

**Problem**: `runner_event_to_feedback()` constructs `AgentOutcome` with hardcoded zeros for all fields. The comment says "Per-attempt usage is not stored on RunnerEvent". Actual data is in `RunState` but not transferred.

**Steps**:
1. Thread `RunState` data into the event translation:
   ```rust
   let agent_outcome = AgentOutcome {
       tokens_in: state.tokens_in,
       tokens_out: state.tokens_out,
       cost_usd: state.cost_usd,
       duration_ms: state.task_elapsed_ms(),
       // ...
   };
   ```
2. Or add per-task token/cost fields to `RunnerEvent::TaskAttemptCompleted`

### 14.13 Gate verdicts not written to substrate — status shows 0/0

**File**: `crates/roko-cli/src/runner/event_loop.rs` (no `Kind::GateVerdict` substrate writes)

**Problem**: Runner v2 stores gate results in-memory and in `gate-thresholds.json`, but never writes `Kind::GateVerdict` engrams to the substrate. `cmd_status` queries `substrate.query(Query::of_kind(Kind::GateVerdict))` and finds nothing.

**Steps**:
1. After each gate completion, write a verdict engram:
   ```rust
   let engram = Engram::new(Kind::GateVerdict, json!({
       "plan_id": plan_id, "task_id": task_id,
       "rung": completion.rung, "passed": completion.passed,
   }));
   substrate.append(&engram).await;
   ```

### 14.14 Playbook usage never recorded — ID mismatch

**File**: `crates/roko-cli/src/orchestrate.rs:11340`

**Problem**: `self.playbook.record(&task_def.id, result.success)` looks up by task ID (`"T1"`, `"T2"`), but seeded playbooks have IDs like `"compile-check-loop"`, `"test-first"`. No match → counts stay at zero.

**Steps**:
1. At dispatch time, record which playbooks were included in the prompt:
   ```rust
   let used_playbook_ids = prompt_sections.iter()
       .filter(|s| s.name == "playbooks")
       .flat_map(|s| s.playbook_ids.iter())
       .collect::<Vec<_>>();
   ```
2. After task completion, record outcome for each used playbook:
   ```rust
   for pb_id in &used_playbook_ids {
       self.playbook.record(pb_id, result.success).await;
   }
   ```

### 14.15 `.roko/memory/` duplicates `.roko/learn/` — dual data stores

**Files**: `crates/roko-cli/src/commands/util.rs:1553` (opens `.roko/memory/`), `crates/roko-cli/src/orchestrate.rs:4442` (opens `.roko/learn/`)

**Problem**: Some code paths use `.roko/memory/` and others use `.roko/learn/` as the root for `LearningRuntime`. Both directories get populated with identical data (cascade-router, costs, efficiency, etc.).

**Steps**:
1. Standardize on `.roko/learn/` as the canonical path
2. Search for all `.roko/memory` references: `grep -rn '\.roko/memory\|roko.*memory' crates/`
3. Replace with `.roko/learn/` everywhere
4. Add a migration: if `.roko/memory/` exists and `.roko/learn/` doesn't, rename it

### 14.16 Cascade router tracks wrong model slug (`gpt-4o` for `gpt-5.4-mini`)

**File**: `crates/roko-learn/src/cascade_router.rs:1075`

**Problem**: `record_confidence_outcome` returns `false` silently when the model slug isn't in the router's `model_slugs` list. If the router was initialized with hardcoded defaults (`claude-sonnet-4-5`, `claude-haiku-4-5`), `gpt-5.4-mini` is unknown and gets dropped.

**Steps**:
1. When a slug is not found, auto-register it:
   ```rust
   pub fn record_confidence_outcome(&self, slug: &str, success: bool) -> bool {
       if self.model_index_for_slug(slug).is_none() {
           self.register_model_slug(slug);
       }
       // ... existing logic
   }
   ```
2. Or initialize the router from `roko_config.models` keys instead of hardcoded defaults

### 14.17 INDEX.md episode count reads wrong path

**File**: `crates/roko-cli/src/index.rs:328`

**Problem**: `rebuild_master_index` reads `.roko/memory/episodes.jsonl` but episodes are written to `.roko/episodes.jsonl`. Path mismatch → count = 0.

**Steps**:
1. Fix the path:
   ```rust
   // Before
   let episodes_path = workdir.join(".roko/memory/episodes.jsonl");
   // After
   let episodes_path = workdir.join(".roko/episodes.jsonl");
   ```

### 14.18 Plans INDEX.md stale — `plan run` never writes back to `tasks.toml`

**File**: `crates/roko-cli/src/index.rs:161`

**Problem**: `rebuild_plans_index` counts `status = "done"` strings in `tasks.toml` on disk. But `plan run` never patches `tasks.toml` — completion lives only in `executor.json`. Result: always shows 0/N done.

**Steps**:
1. After `plan run` completes, write task status back to `tasks.toml`:
   ```rust
   fn update_tasks_toml_status(plan_dir: &Path, completed_tasks: &[String]) -> Result<()> {
       let toml_path = plan_dir.join("tasks.toml");
       let mut doc = std::fs::read_to_string(&toml_path)?
           .parse::<toml_edit::DocumentMut>()?;
       for task in doc["task"].as_array_of_tables_mut()... {
           if completed_tasks.contains(&task["id"].as_str()...) {
               task["status"] = toml_edit::value("done");
           }
       }
       std::fs::write(&toml_path, doc.to_string())?;
       Ok(())
   }
   ```
2. Call from event_loop.rs after `build_report` and before `shutdown_subsystems`

### 14.19 PRD `plans_generated` field never updated after `prd plan`

**File**: `crates/roko-cli/src/prd.rs` — no code writes back to PRD after plan generation

**Steps**:
1. After successful plan generation, update the PRD frontmatter:
   ```rust
   fn update_prd_plans_generated(prd_path: &Path, plan_slug: &str) -> Result<()> {
       let content = std::fs::read_to_string(prd_path)?;
       let updated = content.replace(
           "plans_generated: []",
           &format!("plans_generated: [\"{plan_slug}\"]"),
       );
       std::fs::write(prd_path, updated)?;
       Ok(())
   }
   ```

### 14.20 `plan.md` is a stub — agent prompt doesn't request it

**File**: `crates/roko-cli/src/prd.rs:1191-1208`

**Problem**: The plan generation prompt asks for `tasks.toml` but never asks for a `plan.md` narrative. The fallback writes a stub ("Generated plan.").

**Steps**:
1. Add to the plan generation prompt:
   ```
   In addition to the ```toml block with tasks.toml, also produce a ```markdown block
   labeled `plan.md` that contains:
   - A 2-3 sentence plan summary
   - Key architectural decisions
   - Risk areas to watch
   ```
2. This gives `extract_fenced_block(&output, "plan.md")` something to find

### 14.21 Inject slug explicitly into plan generation prompt

**File**: `crates/roko-cli/src/prd.rs:1030-1048`

**Problem**: The model guesses `meta.plan` from PRD content, producing truncated slugs like `"btc-fundincli"`. The validator auto-corrects but the TOML quality is still poor.

**Steps**:
1. Add explicit slug to the task prompt:
   ```rust
   format!("Plan slug (use exactly): {slug}\n\n{source}")
   ```
2. Add to the TOML quality checklist in the prompt: `"meta.plan MUST be exactly: {slug}"`

### 14.22 Remove `mcp_servers` from plan generator prompt examples

**File**: `crates/roko-cli/src/plan_generate.rs:207,248`

**Problem**: Both example tasks in `PLAN_GENERATOR_SYSTEM_PROMPT` include `mcp_servers = ["filesystem"]`. The model cargo-cults this onto every task. In sandboxed/ephemeral workspaces, the `filesystem` MCP server doesn't exist.

**Steps**:
1. Remove `mcp_servers = ["filesystem"]` from both example tasks
2. Add guidance: `"Only set mcp_servers if the task requires a specific MCP server not available via default tooling."`

### 14.23 Generated code doesn't match PRD type specs

**Root cause**: The implementer agent doesn't see the PRD (see 14.2). Even if it did, the PRD specifies interface types (`FundingAlertConfig.symbol`, `FundingObservation.timestamp_ms`) but the TOML task description doesn't reproduce them. The agent invents its own API (`market`, `funding_rate_bps`, `observed_at`).

**Steps**: Fixed by 14.2 (inject PRD excerpt). Additionally:
1. In `prd plan` generation, instruct the plan generator to embed key PRD type signatures into task `description` fields:
   ```
   When the PRD defines specific types, structs, or interfaces, include the exact
   type signatures in the task description so implementers know the required API.
   ```

### 14.24 No git commits for generated code

**File**: `crates/roko-cli/src/runner/merge.rs`

**Problem**: In in-place mode (no plan branch), the generated code is written to the working tree but never committed. The workspace has only the initial `workspace init` commit.

**Steps**:
1. After each task's gates pass, commit the changes:
   ```rust
   fn commit_task_changes(workdir: &Path, plan_id: &str, task_id: &str) -> Result<()> {
       let msg = format!("[roko] {plan_id}: {task_id} completed");
       Command::new("git").args(["add", "-A"]).current_dir(workdir).status()?;
       Command::new("git").args(["commit", "-m", &msg]).current_dir(workdir).status()?;
       Ok(())
   }
   ```
2. This enables `git diff` for subsequent tasks to see what changed

### 14.25 `rebuild_all` uses `current_dir()` instead of run workspace

**File**: `crates/roko-cli/src/main.rs:2100`

**Problem**: `rebuild_all(&std::env::current_dir()...)` rebuilds indexes for the developer's cwd, not the run's `--workdir`.

**Steps**:
1. Pass the resolved workdir to `rebuild_all`:
   ```rust
   let _ = roko_cli::index::rebuild_all(&resolved_workdir);
   ```

### 14.26 Config version warning fires per-subprocess

**File**: `crates/roko-core/src/config/schema.rs:164` (`static WARNED: Once` — per-process only)

**Problem**: Each agent child process has its own `Once` guard. With 3+ tasks, the warning appears 3+ times. The warning also fires falsely when `config_version = 2` is set correctly.

**Steps**:
1. Check the actual `config_version` value before warning:
   ```rust
   if config.config_version < CURRENT_CONFIG_VERSION {
       // Only warn if truly outdated, not just because from_toml defaults to 1
   }
   ```
2. Use the file's raw `config_version` text (already parsed in `text_has_config_version`) instead of the deserialized default

### 14.27 Efficiency data shows correct tokens but episodes show zeros

**Root cause**: Two separate data pipelines: efficiency.jsonl gets real data from `RunState`; episodes get zeros from `runner_event_to_feedback()` hardcoded zeros. This is the same issue as 14.12 but the consequence is that the neuro knowledge store (`knowledge.jsonl`) also shows "0ms" duration.

**Steps**: Same fix as 14.12 — thread `RunState` data into the event translation.

### 14.28 Alert logic correctness — evaluates single observation, not rolling average

**Impact**: Even when the pipeline "succeeds" (4/4 tasks pass), the generated code has a logic bug: `main()` prints the rolling average but evaluates the threshold against the latest single observation. The PRD specifies average-based alerting.

**Root cause**: The agent invented the function signature `evaluate_funding_threshold(config, observation)` to check a single observation, not the average. Without PRD context in the prompt (14.2), the agent doesn't know the requirement.

**Steps**: Fixed by 14.2 (PRD in prompt). Additionally, task T2's description should specify the exact function signatures required rather than leaving it to the agent.

---

## Priority Summary (Full — 103 items)

### Tier 0: Systemic (address the root causes, not symptoms)

| # | Improvement | Category | Impact | Effort | Section |
|---|-------------|----------|--------|--------|---------|
| 1 | **Wire ImplementerTemplate to runtime dispatch** | Pipeline Quality | Critical | 3h | 14.1 |
| 2 | **Inject PRD excerpt into implementer prompts** | Pipeline Quality | Critical | 1h | 14.2 |
| 3 | **Cost tracking: pass model_profile to ToolLoop** | Pipeline Quality | Critical | 15m | 14.5 |
| 4 | **Cross-task output injection** | Pipeline Quality | Critical | 2h | 14.4 |
| 5 | **Dream path double-nesting fix** | Pipeline Bug | High | 5m | 14.9 |
| 6 | **Episode data zeros → thread RunState** | Pipeline Bug | High | 30m | 14.12 |
| 7 | **Gate verdicts → write to substrate** | Pipeline Bug | High | 30m | 14.13 |
| 8 | **Unconditionally strip model_hint from generated plans** | Pipeline Bug | High | 10m | 14.7 |
| 9 | **Suppress tool_uses JSON leak in prd plan** | Pipeline Bug | High | 5m | 14.6 |
| 10 | **Unify .roko/memory/ and .roko/learn/ paths** | Pipeline Bug | High | 1h | 14.15 |

### Tier 1: Critical (fix immediately)

| # | Improvement | Category | Impact | Effort | Section |
|---|-------------|----------|--------|--------|---------|
| 11 | Gate channel send failure fix | Critical Bug | Critical | 30m | 1.1 |
| 12 | Chain client unwrap fix | Critical Bug | Critical | 15m | 1.2 |
| 13 | MCP config never wired to plan run | Runner | Critical | 15m | 9.8 |
| 14 | Dream consolidation inverted logic | Runner | Critical | 5m | 9.9 |
| 15 | Health endpoint 200 on "down" | Serve | Critical | 10m | 11.1 |
| 16 | Shell injection in demo terminal | Critical Bug | High | 10m | 1.6 |
| 17 | Config validation on load | Critical Bug | High | 30m | 1.4 |
| 18 | Lock poisoning fix | Critical Bug | High | 30m | 1.3 |
| 19 | Permanent classified as retryable | Runner | High | 15m | 9.10 |
| 20 | Fatal event result swallowed | Runner | High | 30m | 9.11 |

### Tier 2: High (before next demo)

| # | Improvement | Category | Impact | Effort | Section |
|---|-------------|----------|--------|--------|---------|
| 21 | Workspace context in prompts | Prompt Quality | Critical | 1h | 4.1 |
| 22 | LinUCB state not persisted | Learning | High | 2h | 12.1 |
| 23 | Fix model_hint contradiction | Prompt Quality | High | 30m | 4.2 |
| 24 | TOML repair pipeline | Speed | High | 2h | 2.1 |
| 25 | Atomic state writes | Reliability | High | 1h | 3.1 |
| 26 | Typed error taxonomy | Reliability | High | 2h | 3.2 |
| 27 | Log daimon/substrate errors | Code Health | High | 1h | 5.2 |
| 28 | Warm cargo cache | Speed | High | 30m | 2.2 |
| 29 | Extract dispatch helper | Design Patterns | High | 2h | 5.1 |
| 30 | Schema-driven TOML validation | Reliability | High | 3h | 3.4 |
| 31 | Scaffold Cargo.toml with deps | Pipeline Quality | High | 1h | 14.3 |
| 32 | max_loc gate enforcement | Pipeline Quality | High | 2h | 14.8 |
| 33 | Gate rung sentinel constants | Pipeline Bug | Medium | 15m | 14.10 |
| 34 | Gate threshold schema mismatch | Pipeline Bug | Medium | 30m | 14.11 |
| 35 | Playbook ID mismatch fix | Pipeline Bug | Medium | 1h | 14.14 |
| 36 | Cascade router auto-register slugs | Learning | Medium | 30m | 14.16 |
| 37 | INDEX.md episode path fix | Pipeline Bug | Low | 5m | 14.17 |
| 38 | Plans INDEX → read executor.json | Pipeline Bug | Medium | 1h | 14.18 |
| 39 | PRD plans_generated update | Pipeline Bug | Low | 15m | 14.19 |
| 40 | plan.md prompt addition | Pipeline Quality | Low | 15m | 14.20 |
| 41 | Inject slug into plan prompt | Pipeline Quality | Low | 10m | 14.21 |
| 42 | Remove mcp_servers from examples | Pipeline Quality | Low | 5m | 14.22 |
| 43 | Git commit after task gates pass | Pipeline Quality | Medium | 30m | 14.24 |
| 44 | rebuild_all uses run workdir | Pipeline Bug | Low | 5m | 14.25 |
| 45 | Config version warning fix | Pipeline Bug | Low | 15m | 14.26 |

### Tier 3-4: Medium and Low (see sections 1-13 above)

Items 46-103 are the previously documented improvements from sections 1-13, renumbered. See the section-by-section details above for full implementation steps.

**Estimated total effort**: ~105 hours for all 103 items
**Tier 0 (systemic root causes)**: ~9 hours — highest ROI, addresses WHY pipeline output is poor
**Tier 0+1 (critical path)**: ~14 hours — must be done for reliable pipeline
**Demo-ready (Tier 0+1+2)**: ~35 hours

### Tier 1: Critical (fix immediately)

| # | Improvement | Category | Impact | Effort | Section |
|---|-------------|----------|--------|--------|---------|
| 1 | Gate channel send failure fix | Critical Bug | Critical | 30m | 1.1 |
| 2 | Chain client unwrap fix | Critical Bug | Critical | 15m | 1.2 |
| 3 | MCP config never wired to plan run | Runner | Critical | 15m | 9.8 |
| 4 | Dream consolidation inverted logic | Runner | Critical | 5m | 9.9 |
| 5 | Health endpoint 200 on "down" | Serve | Critical | 10m | 11.1 |
| 6 | Shell injection in demo terminal | Critical Bug | High | 10m | 1.6 |
| 7 | Config validation on load | Critical Bug | High | 30m | 1.4 |
| 8 | Lock poisoning fix | Critical Bug | High | 30m | 1.3 |
| 9 | Permanent classified as retryable | Runner | High | 15m | 9.10 |
| 10 | Fatal event result swallowed | Runner | High | 30m | 9.11 |

### Tier 2: High (before next demo)

| # | Improvement | Category | Impact | Effort | Section |
|---|-------------|----------|--------|--------|---------|
| 11 | Workspace context in prompts | Prompt Quality | Critical | 1h | 4.1 |
| 12 | LinUCB state not persisted | Learning | High | 2h | 12.1 |
| 13 | Fix model_hint contradiction | Prompt Quality | High | 30m | 4.2 |
| 14 | TOML repair pipeline | Speed | High | 2h | 2.1 |
| 15 | Atomic state writes | Reliability | High | 1h | 3.1 |
| 16 | Typed error taxonomy | Reliability | High | 2h | 3.2 |
| 17 | Log daimon/substrate errors | Code Health | High | 1h | 5.2 |
| 18 | Warm cargo cache | Speed | High | 30m | 2.2 |
| 19 | Extract dispatch helper | Design Patterns | High | 2h | 5.1 |
| 20 | Schema-driven TOML validation | Reliability | High | 3h | 3.4 |
| 21 | Scaffold Cargo.toml via parser | Reliability | High | 1h | 3.5 |
| 22 | Synthesized profile validation | Critical Bug | High | 15m | 1.5 |
| 23 | ROKO__* env override: implement or remove doc | Config | High | 1h | 13.1 |
| 24 | Per-turn budget enforcement | Runner | High | 30m | 9.12 |
| 25 | Global gate semaphore → per-run | Runner | High | 1h | 9.1 |

### Tier 3: Medium (next sprint)

| # | Improvement | Category | Impact | Effort | Section |
|---|-------------|----------|--------|--------|---------|
| 26 | Single agent_handle → per-plan map | Runner | Medium | 2h | 9.2 |
| 27 | FailPlan → wrong plan attribution | Runner | Medium | 30m | 9.3 |
| 28 | Sentinel task sorts by string not DAG | Runner | Medium | 30m | 9.4 |
| 29 | agent_output unbounded growth | Runner | Medium | 30m | 9.5 |
| 30 | iteration shared across plans | Runner | Medium | 30m | 9.7 |
| 31 | Plan timeout fires twice | Runner | Medium | 15m | 9.13 |
| 32 | Section budget caps: 5/11 covered | Compose | Medium | 30m | 10.1 |
| 33 | Section measurement O(N²) | Compose | Medium | 1h | 10.2 |
| 34 | Three section-name registries | Compose | Medium | 1.5h | 10.3 |
| 35 | Blocking RwLock in relay_health | Serve | Medium | 10m | 11.2 |
| 36 | WS back_pressure field ignored | Serve | Medium | 30m | 11.5 |
| 37 | 13 RwLock maps, no lock ordering | Serve | Medium | 3h | 11.6 |
| 38 | Nested mutex in cascade_router | Learning | Medium | 1h | 12.2 |
| 39 | Episode dual identity fields | Learning | Medium | 30m | 12.3 |
| 40 | Deprecated config loader divergence | Config | Medium | 1h | 13.2 |
| 41 | Global merge covers 3/20 sections | Config | Medium | 1h | 13.3 |
| 42 | Few-shot TOML example | Prompt Quality | Medium | 30m | 4.4 |
| 43 | Failure recovery in prompts | Prompt Quality | Medium | 30m | 4.3 |
| 44 | Pluggable output sinks | Design Patterns | Medium | 3h | 5.4 |
| 45 | SafetyLayer required | Design Patterns | Medium | 1h | 5.3 |
| 46 | Data-driven gate rungs | Generalization | Medium | 4h | 8.1 |
| 47 | Env var parse warnings | Code Health | Medium | 30m | 5.5 |
| 48 | Gate channel buffer sizing | Speed | Medium | 15m | 2.4 |
| 49 | Batch gate execution | Speed | Medium | 2h | 2.3 |
| 50 | Prevent state leakage | Reliability | Medium | 30m | 3.3 |
| 51 | Connection pooling | Speed | Medium | 1h | 2.5 |
| 52 | Role-tool mapping | Prompt Quality | Medium | 30m | 4.5 |
| 53 | File path consolidation | Prompt Quality | Medium | 15m | 4.6 |
| 54 | Single command definitions | Demo | Medium | 1h | 7.3 |
| 55 | Validate crate names | Reliability | Medium | 15m | 3.6 |
| 56 | Top 10 unwrap replacements | Code Health | Medium | 4h | 6.1 |
| 57 | Workspace abstraction | Generalization | Medium | 3h | 8.2 |
| 58 | Feedback facade bounded tasks | Runner | Medium | 1h | 9.15 |

### Tier 4: Low (backlog)

| # | Improvement | Category | Impact | Effort | Section |
|---|-------------|----------|--------|--------|---------|
| 59 | started_at_ms epoch fix | Runner | Low | 15m | 9.6 |
| 60 | Extension hooks hardcoded role | Runner | Low | 15m | 9.14 |
| 61 | Template DRY: agents_instructions | Compose | Low | 15m | 10.4 |
| 62 | Budget conflict on techniques | Compose | Low | 15m | 10.5 |
| 63 | SSE keep-alive configurable | Serve | Low | 15m | 11.3 |
| 64 | SSE replay memory spike | Serve | Low | 15m | 11.4 |
| 65 | CostsLog batch append | Learning | Low | 1h | 12.4 |
| 66 | O(N²) importance scoring | Learning | Low | 30m | 12.5 |
| 67 | Config diagnostics misleading | Config | Low | 30m | 13.4 |
| 68 | Env var interpolation scope | Config | Low | 1h | 13.5 |
| 69 | Timeout configuration | Demo | Low | 1h | 7.1 |
| 70 | Structured command errors | Demo | Low | 30m | 7.2 |
| 71 | Metrics AbortController | Demo | Low | 15m | 7.4 |
| 72 | Hardcoded model extraction | Code Health | Low | 2h | 6.2 |
| 73 | Timeout centralization | Code Health | Low | 3h | 6.3 |
| 74 | Relative section budgets | Generalization | Low | 2h | 8.3 |
| 75 | Health check debouncing | Reliability | Low | 30m | — |

**Estimated total effort**: ~75 hours for all 75 items
**Critical path (Tier 1, items 1-10)**: ~3 hours — fix immediately
**Demo-ready (Tier 1+2, items 1-25)**: ~18 hours — complete before next demo
