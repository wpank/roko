# M056 — Autonomy Slider in TUI

## Objective
Implement the Autonomy Slider surface as a TUI tab or panel. The slider provides progressive trust control with 5 levels (0: full human control through 4: full autonomy). Trust is configurable per-capability (e.g., FsWrite at level 2, Shell at level 1). Level changes are adjustable at runtime and emit Pulses on Bus. Reducing autonomy causes pending tool calls to require human confirmation.

## Scope
- Crates: `roko-cli`, `roko-core`
- Files: `crates/roko-cli/src/tui/` (new autonomy panel), `crates/roko-core/src/autonomy.rs` (new)
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.8
- Spec ref: `tmp/unified/16-SURFACES.md` SS6 (Autonomy Slider), `tmp/unified/17-SECURITY-MODEL.md` SS6

## Steps
1. Check for existing autonomy or trust-level code:
   ```bash
   grep -rn 'autonomy\|Autonomy\|trust_level\|TrustLevel\|AutonomyLevel' crates/ --include='*.rs' | head -15
   ```

2. Define autonomy types in `crates/roko-core/src/autonomy.rs`:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
   pub enum AutonomyLevel {
       Manual = 0,       // Every action requires confirmation
       Supervised = 1,   // Dangerous actions require confirmation
       Guided = 2,       // Only destructive actions require confirmation
       Autonomous = 3,   // Only irreversible actions require confirmation
       Full = 4,         // No confirmation required
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct AutonomyConfig {
       pub global_level: AutonomyLevel,
       pub per_capability: HashMap<String, AutonomyLevel>,
   }

   impl AutonomyConfig {
       pub fn effective_level(&self, capability: &str) -> AutonomyLevel;
       pub fn requires_confirmation(&self, capability: &str, action_risk: ActionRisk) -> bool;
   }

   #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
   pub enum ActionRisk {
       ReadOnly,
       Reversible,
       Destructive,
       Irreversible,
   }
   ```

3. Map autonomy levels to capability restrictions:
   | Level | FsRead | FsWrite | Shell | Net | Llm |
   |---|---|---|---|---|---|
   | 0 Manual | confirm | confirm | confirm | confirm | confirm |
   | 1 Supervised | auto | confirm | confirm | confirm | auto |
   | 2 Guided | auto | auto | confirm | auto | auto |
   | 3 Autonomous | auto | auto | auto | auto | auto |
   | 4 Full | auto | auto | auto | auto | auto |

4. Implement the TUI panel:
   - Show global autonomy level as a horizontal slider (0-4)
   - Below: per-capability overrides in a table
   - Arrow keys to adjust levels
   - Changes emit `AutonomyLevelChange` SurfaceEvent
   - Visual indicators: red (Manual), yellow (Supervised), green (Autonomous/Full)

5. Emit Pulse on Bus when level changes so the safety layer can intercept pending actions.

6. Export AutonomyConfig and AutonomyLevel from roko-core.

7. Write tests:
   - `effective_level` returns per-capability override when set, global otherwise
   - `requires_confirmation` returns true for Shell at level 1 (Supervised)
   - Level change serializes correctly for persistence in roko.toml

## Verification
```bash
cargo check -p roko-core
cargo check -p roko-cli
cargo clippy -p roko-core --no-deps -- -D warnings
cargo test -p roko-core -- autonomy
cargo test -p roko-cli -- tui::autonomy
```

## What NOT to do
- Do NOT implement the actual confirmation dialog here -- that is a tool dispatch concern
- Do NOT couple autonomy to specific LLM providers -- it is capability-based
- Do NOT make autonomy changes persistent by default -- require explicit save
- Do NOT allow autonomy to be raised above the Space-level cap (security constraint)
