# M095 — Agent Execution Tiers

**[BLOCKED:depth]** -- This item depends on `tmp/unified-depth/20-deployment/` depth docs. The depth docs specify isolation boundaries, resource limits per tier, and promotion/demotion criteria.

## Objective
Implement the Agent execution tier system: T0 (in-process, current default), T1 (sidecar via existing roko-agent-server), T2 (container via Docker), T3 (VM via Firecracker, future), T4 (cluster via k8s, future). Tier selection is configured in workspace.toml. An Agent can run at T0 or T1 based on config, with T2+ as future extensions.

## Scope
- Crates: `roko-agent`, `roko-agent-server`
- Files: `crates/roko-agent/src/tiers.rs` (new), `crates/roko-agent-server/src/`
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.8
- Spec ref: `tmp/unified/20-DEPLOYMENT.md` SS3
- Depth docs: `tmp/unified-depth/20-deployment/` (pending)

## Steps
1. Check the current execution modes:
   ```bash
   grep -rn 'T0\|T1\|tier\|sidecar\|in.process\|ExecutionMode' crates/roko-agent/src/ --include='*.rs' | head -15
   grep -rn 'tier\|execution_mode' crates/roko-agent-server/src/ --include='*.rs' | head -10
   ```

2. Define tier types in `crates/roko-agent/src/tiers.rs`:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   pub enum ExecutionTier {
       T0InProcess,        // Direct function calls, shared memory
       T1Sidecar,          // HTTP sidecar (roko-agent-server)
       T2Container,        // Docker container (future)
       T3Vm,               // Firecracker VM (future)
       T4Cluster,          // Kubernetes pod (future)
   }

   pub struct TierConfig {
       pub tier: ExecutionTier,
       pub resource_limits: ResourceLimits,
   }

   pub struct ResourceLimits {
       pub max_memory_mb: Option<u64>,
       pub max_cpu_pct: Option<u64>,
       pub max_budget_usd: Option<f64>,
       pub network_policy: NetworkPolicy,
   }

   pub enum NetworkPolicy {
       Unrestricted,
       AllowList(Vec<String>),
       Deny,
   }
   ```

3. Implement tier dispatch:
   ```rust
   pub async fn dispatch_at_tier(
       tier: ExecutionTier,
       agent_config: &AgentConfig,
       task: TaskInput,
   ) -> Result<TaskOutput> {
       match tier {
           ExecutionTier::T0InProcess => dispatch_in_process(agent_config, task).await,
           ExecutionTier::T1Sidecar => dispatch_via_sidecar(agent_config, task).await,
           _ => Err(Error::TierNotImplemented(tier)),
       }
   }
   ```

4. T0: use existing in-process dispatch (current default behavior).
5. T1: use existing roko-agent-server HTTP API (`/message` endpoint).
6. T2-T4: return `TierNotImplemented` error with a message about future support.

7. Configure in workspace.toml:
   ```toml
   [agent.execution]
   default_tier = "T0"

   [agent.execution.overrides]
   "security-audit" = { tier = "T1", max_memory_mb = 2048 }
   ```

8. Write tests:
   - T0 dispatch works (current behavior unchanged)
   - T1 dispatch calls sidecar HTTP endpoint
   - T2+ returns appropriate error
   - Config-based tier selection works

## Verification
```bash
cargo check -p roko-agent
cargo clippy -p roko-agent --no-deps -- -D warnings
cargo test -p roko-agent -- tiers
```

## What NOT to do
- Do NOT implement Docker or Firecracker integration -- just define the types and T0/T1
- Do NOT break existing T0 in-process dispatch -- it must remain the default
- Do NOT proceed with T2+ without depth docs
- Do NOT add automatic tier promotion/demotion logic -- that is a follow-up
