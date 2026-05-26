# M152 — Wire Event Source Dispatch Loop

## Objective
Wire the event source dispatch loop in `roko-runtime`. Create a `SubscriptionMatcher` that pattern-matches incoming Pulses (from cron/watch/webhook sources) against `subscriptions.toml` entries. Respect concurrency limits and cooldown periods. Spawn agent templates as Flows when matches fire. Wire into the daemon event loop so subscriptions are active when `roko daemon` is running.

## Scope
- Crates: `roko-runtime`, `roko-cli`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/` (new triggers module or extend event_bus.rs)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/daemon.rs` (wire into daemon loop)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/` (subscription schema)
- Depth doc: `tmp/unified-depth/14-deployment/` (event-driven triggers)

## Steps
1. Read existing subscription/trigger infrastructure:
   ```bash
   grep -rn 'subscription\|Subscription\|trigger\|Trigger\|cron\|watch\|webhook' /Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/ --include='*.rs' | head -15
   grep -rn 'subscription\|Subscription' /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/ --include='*.rs' | head -10
   grep -rn 'subscriptions' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/ --include='*.rs' | head -10
   ```

2. Read the daemon event loop:
   ```bash
   grep -n 'pub async fn\|tokio::select\|loop\|event_loop' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/daemon.rs | head -20
   ```

3. Define `SubscriptionMatcher` in roko-runtime:
   ```rust
   /// Matches incoming Pulses against configured subscriptions.
   pub struct SubscriptionMatcher {
       subscriptions: Vec<SubscriptionEntry>,
       active_runs: HashMap<String, ActiveRun>,
       cooldowns: HashMap<String, Instant>,
   }

   #[derive(Debug, Clone)]
   pub struct SubscriptionEntry {
       pub id: String,
       pub pattern: String,         // glob or regex pattern for Pulse topic
       pub agent_template: String,  // agent config to spawn
       pub concurrency_limit: u32,  // max simultaneous runs
       pub cooldown_secs: u64,      // minimum seconds between triggers
       pub enabled: bool,
   }

   struct ActiveRun {
       started_at: Instant,
       handle: tokio::task::JoinHandle<()>,
   }
   ```

4. Implement pattern matching:
   ```rust
   impl SubscriptionMatcher {
       /// Check if a Pulse topic matches any subscription.
       pub fn matches(&self, topic: &str) -> Vec<&SubscriptionEntry> {
           self.subscriptions.iter()
               .filter(|s| s.enabled && self.topic_matches(&s.pattern, topic))
               .filter(|s| self.within_concurrency_limit(s))
               .filter(|s| self.cooldown_elapsed(s))
               .collect()
       }

       fn topic_matches(&self, pattern: &str, topic: &str) -> bool {
           // Support glob patterns: "gate.*" matches "gate.passed", "gate.failed"
           // Support exact: "heartbeat.delta" matches only "heartbeat.delta"
       }
   }
   ```

5. Implement dispatch:
   ```rust
   impl SubscriptionMatcher {
       /// Dispatch an agent for a matched subscription.
       pub async fn dispatch(&mut self, entry: &SubscriptionEntry, pulse: &Pulse) -> Result<()> {
           // 1. Record cooldown
           self.cooldowns.insert(entry.id.clone(), Instant::now());
           // 2. Spawn agent template
           let handle = tokio::spawn(async move {
               // Run agent with pulse payload as context
           });
           // 3. Track active run
           self.active_runs.insert(entry.id.clone(), ActiveRun { started_at: Instant::now(), handle });
           Ok(())
       }
   }
   ```

6. Wire into daemon event loop:
   - Load subscriptions from config (`subscriptions.toml` or `roko.toml [subscriptions]`)
   - Subscribe to Bus events
   - On each event, call `matcher.matches(topic)` and dispatch

7. Write tests:
   - Glob pattern "gate.*" matches "gate.passed"
   - Exact pattern "heartbeat.delta" does not match "heartbeat.gamma"
   - Concurrency limit prevents over-spawning
   - Cooldown prevents rapid re-triggering

## Verification
```bash
cargo check -p roko-runtime
cargo clippy -p roko-runtime --no-deps -- -D warnings
cargo test -p roko-runtime -- subscription
cargo check -p roko-cli
```

## What NOT to do
- Do NOT add a full cron parser dependency — simple interval-based subscriptions are sufficient
- Do NOT implement webhook HTTP listener here — that belongs in roko-serve
- Do NOT spawn real agents in tests — mock the dispatch
- Do NOT modify the Bus trait — subscribe using the existing receiver API
- Do NOT block the event loop on agent execution — always spawn async
