# Audit: Refinements 17-21 (Moat & Modularity Arc)

**Auditor**: Claude (cross-referencing proposals against actual codebase)
**Date**: 2026-04-17
**Scope**: Refinements 17 (Plugin SPI), 18 (Competitive Moat), 19 (Net-New Innovations), 20 (Modularity/Composability), 21 (From-Scratch Redesigns)

---

## Refinement 17: Plugin & Extension Architecture

**Verdict: DEFER (Tiers 1-3 can SIMPLIFY into something small; Tiers 4-5 REJECT for now)**

### What it proposes

A five-tier plugin SPI ranging from "drop a TOML file" (Tier 1) to "WASM sandboxed extensions" (Tier 5), a `roko plugin` CLI with 8 subcommands, a plugin registry at `plugins.roko.dev`, manifest-driven discovery, and a new `roko-spi` crate for ABI stability. Also proposes a `roko-wasm-host` crate with a full WASM host surface (7 host imports).

### What the codebase actually has

- `roko-plugin` already exists as a crate (`crates/roko-plugin/`). It is a single `lib.rs` file (~200 lines) defining `EventSource`, `FeedbackCollector`, `FeedbackOutcome`, and `FeedbackSignal`. It depends on `roko-core`, `notify` (file watching), `cron` (scheduling), and `globset`. It is *not* the SPI described in doc 17 -- it is a narrow, concrete SDK for event sources and feedback loops.
- Tools live in `roko-std/src/tool/builtin/` -- 18 builtin tool handlers (bash, grep, read_file, write_file, etc). These are Rust implementations registered via `ToolRegistry`. Adding a tool means writing Rust and adding it to the registry.
- Role templates live in `roko-compose/src/templates/` -- 9 template modules (implementer, reviewer, strategist, etc) mixed with the builder/engine code.
- MCP servers exist as separate crates (`roko-mcp-code`, `roko-mcp-github`, `roko-mcp-slack`, `roko-mcp-scripts`, `roko-mcp-stdio`). These are already effectively "plugins" in the MCP protocol sense.
- The six kernel traits (`Substrate`, `Scorer`, `Gate`, `Router`, `Composer`, `Policy`) in `roko-core/src/traits.rs` are clean, well-documented, and *already* the extension surface. Anyone can implement them today by adding a crate to the workspace.

### Honest assessment

1. **How many plugin authors exist today?** Zero. The project has one developer (Will). There are no external contributors, no third-party deployments, no community.

2. **Is the Tier 3 declarative tool manifest genuinely useful?** Yes, but only to Will. Being able to drop a TOML file to add a tool instead of writing Rust would be a real ergonomic win for the single user. But you do not need a five-tier SPI for this. You need a `plugins/tools/*.toml` loader and ~200 lines of Rust in the tool dispatcher.

3. **Are Tiers 4-5 premature?** Massively. Tier 4 proposes a C-FFI ABI bridge (`roko-extension-abi`) for cdylib loading. Tier 5 proposes a full WASM runtime with 7 host imports, CPU budgeting, memory limits, and rate limiting. These are multi-month engineering efforts that solve the problem of untrusted third-party code execution. There are no third parties.

4. **The `roko-wasm-host` crate with host imports like `engram_get`, `bus_publish`, `substrate_query_similar`**: These reference types (`Pulse`, `Engram` graduation, HDC substrate queries) that do not exist in the codebase. There is no `Pulse` type anywhere. There is no `substrate_query_similar`. The WASM host surface is specified against a future codebase that has not been built.

5. **The plugin registry (`plugins.roko.dev`)**: This is aspirational infrastructure for a community that does not exist. The doc acknowledges this is Phase 2+ but still specifies it in detail.

### What to do instead

- **Extract the declarative tool loader (Tier 3 only)**: Add TOML-based tool manifests that the existing `ToolRegistry` can load. This is ~300 lines of code and gives the single user a real workflow improvement.
- **Separate templates from engine in `roko-compose`**: Move template files to `plugins/prompts/` or equivalent. This is a file-move, not a new crate.
- **Defer everything else** until there is at least one external user asking for it.

---

## Refinement 18: Competitive Moat

**Verdict: SKEPTICAL**

### What it proposes

Five structural moat components: (1) architectural coherence (Substrate + Bus + HDC + demurrage + c-factor as mutually reinforcing), (2) a heuristic commons with cross-deployment sharing, (3) a plugin ecosystem with network effects, (4) a replication ledger for scientific self-correction, (5) Rust-level correctness guarantees.

### What the codebase actually has

Let me check each proposed moat component against reality:

1. **"Substrate + Bus + HDC + demurrage + c-factor integrated"**:
   - Substrate: exists and works (`roko-core/src/traits.rs`, `roko-fs/`).
   - Bus: exists as `EventBus<RokoEvent>` in `roko-runtime/src/event_bus.rs` (~430 lines). Has 2 event types: `PlanRevision` and `PrdPublished`. This is a concrete, working event bus, not the abstract "Bus as a kernel trait" the refinements envision.
   - HDC: exists in `roko-primitives/src/hdc.rs` (10,240-bit vectors, XOR bind, bundle, Hamming similarity). Used by `roko-learn`, `roko-neuro`, `roko-dreams`, `roko-serve`. This is real and works.
   - Demurrage: does not exist. `grep -r demurrage crates/` returns zero results. The `Decay` enum in `roko-core` has `Exponential`, `Linear`, `Step`, `None` variants -- standard decay, not economic demurrage.
   - c-factor: partially exists. `roko-core/src/cfactor.rs` and `roko-learn/src/cfactor.rs` define `CFactor` and `CFactorPolicy`. The cascade router uses it for model routing. But this is a single numeric signal used in routing, not the continuously-computed Woolley collective-intelligence metric the doc describes.

2. **Heuristic commons**: Does not exist. There is one `HeuristicRule` struct in `roko-neuro/src/tier_progression.rs`. No cross-deployment sharing, no commons, no curation mechanism.

3. **Plugin ecosystem**: See doc 17 audit above. Does not exist and has no users.

4. **Replication ledger**: Does not exist. Zero matches for "replication ledger" in the codebase.

5. **Rust-level correctness**: Real, but this is a property of the language choice, not a defensible moat. Any Rust project gets this. The specific claims about "trait contracts actually hold" and "Bus backpressure is actually backpressure" are true of any well-written Rust code.

### Honest assessment

The moat doc describes a system that does not exist yet. Of the five components:
- 1 fully exists (HDC)
- 1 partially exists (c-factor)
- 1 exists but in a much simpler form than described (Bus)
- 2 do not exist at all (demurrage, replication ledger)
- 1 is a language property, not a product property (Rust correctness)
- 1 depends on an ecosystem that has zero participants (plugins)
- 1 depends on cross-deployment sharing that has zero deployments (heuristic commons)

The switching-cost table in section 11 is honest about the timeline (day-30 switching cost is "an afternoon") but projects forward to day-720 with accumulated assets that depend entirely on features being built. This is a fundraising narrative, not an engineering assessment.

### What this gets right

- Section 7 ("anti-moat failures to avoid") is genuinely useful guidance.
- Section 8 ("where the moat doesn't apply") is honest about IDE vendors and model providers being existential threats.
- Section 10 ("the non-moat that matters") correctly states that none of this matters if the product does not deliver value today.
- The framing that *composition* of features can be defensive even when individual features are not is correct in principle.

### What to do instead

- Stop writing moat docs and ship features. The moat is the working product, not the architecture diagram.
- If you want to make the moat argument honestly, list only what exists today: a working Rust agent orchestrator with multi-backend LLM dispatch, a 7-rung gate pipeline, HDC fingerprinting, episode logging, and a TUI. That is already more than most agent frameworks have.

---

## Refinement 19: Net-New Innovations Catalog

**Verdict: SIMPLIFY (honest about 3 of 10; the rest is aspiration)**

### What it proposes

A pitch-deck catalog of 10 primitives, 7 patterns, and 6 APIs that are claimed to be net-new innovations no other agent framework has.

### What the codebase actually has

Checking each claimed innovation:

| Claimed innovation | Exists in code? | Notes |
|---|---|---|
| 1.1 Pulse as first-class type | No | No `struct Pulse` anywhere. The Bus has `RokoEvent`, which is a concrete enum, not a typed ephemeral medium. |
| 1.2 HDC fingerprint on every Engram | Partial | HDC exists and is used, but Engrams do not have a fingerprint field. The `Engram` struct has no HDC vector. Fingerprinting happens in `roko-learn` and `roko-dreams` as a side-channel, not as a per-Engram property. |
| 1.3 Demurrage | No | Zero code. |
| 1.4 Heuristic with explicit falsifier | No | One `HeuristicRule` struct in `roko-neuro` with `condition` and `action` fields. No falsifier field, no calibration, no Bayesian updating. |
| 1.5 Replication ledger | No | Zero code. |
| 1.6 c-factor as runtime signal | Partial | `CFactor` struct exists, used in routing. Not continuously computed, not surfaced in dashboards. |
| 1.7 Worldview as emergent object | No | Zero matches for "worldview" or "Worldview" in crates/. |
| 1.8 Two-fabric operator generalization | No | All six traits operate on `Engram` only. No `Pulse` medium, no dual-fabric dispatch. |
| 1.9 Demurrage-taxed learned parameters | No | Zero code. |
| 1.10 Prediction markets on heuristics | No | Zero code. |
| 2.1 Predict-publish-correct loops | No | No prediction-correction wiring. |
| 2.2 Stigmergy via Engrams | Partial | Agents read/write Engrams, which is trivially stigmergic, but not as a designed coordination pattern. |
| 2.4 Dream cycles | Partial | `roko-dreams` exists (~6K lines) with `DreamReplayPolicy`, `DreamReplayMode`, cycle/hypnagogia/imagination modules. But it is Phase 2+ and not wired into the runtime. |
| 3.1 `roko heuristic` CLI | No | Not a CLI command. |
| 3.2 `roko dashboard` with c-factor tile | Partial | Dashboard exists. Whether it has a c-factor tile is unclear but c-factor is defined in the dashboard snapshot types. |
| 3.3 `roko plugin` CLI | No | Not a CLI command. |

### Honest assessment

Section 8 of the doc itself is admirably honest: "Rereading the list with an honest eye, three entries are genuinely primitive." Those three (falsifier heuristic, replication ledger, c-factor as runtime signal) do not exist in the codebase either. The doc is being honest about the *design* novelty while the *implementation* novelty is approximately zero.

This is a pitch deck for features that have not been built. As a roadmap of what to build, it has value. As a "what does Roko let you do that nothing else does" answer, the honest answer today is: multi-backend LLM orchestration with a compile/test/clippy gate pipeline, in Rust, with persistence and resume. That is real and useful. Everything in this catalog is aspiration.

### What to do instead

- Maintain this as a roadmap, not a pitch deck. Rename it "Innovation Roadmap" and add a status column showing what exists vs. what is planned.
- Ship one primitive from this list before writing more docs about it.

---

## Refinement 20: Modularity, Composability, and Cleaner Dependencies

**Verdict: SIMPLIFY (one extraction is justified; the rest is premature)**

### What it proposes

Three new kernel crates (`roko-bus`, `roko-hdc`, `roko-spi`), two crate splits (`roko-std` -> `roko-defaults` + `roko-tools`, `roko-compose` -> `roko-compose-core` + `roko-templates`), a strict dependency graph with layer rules, CI enforcement of the graph, and a multi-phase migration plan.

### What the codebase actually has

Current workspace: 29 crate directories under `crates/roko-*/`, plus 3 apps. The workspace `Cargo.toml` lists 28 members. This is already a lot of crates for a single-developer project.

Actual coupling analysis:

1. **`roko-agent` depends on `roko-learn`**: Only in `dev-dependencies` (tests). The doc claims `roko-agent` "reaches into `roko-learn` to persist efficiency events" -- this is wrong. `roko-agent/Cargo.toml` has `roko-learn` only under `[dev-dependencies]`. `grep 'use roko_learn' crates/roko-agent/src/` returns zero matches. The stated problem does not exist.

2. **`roko-cli` imports from almost everything**: True, and the doc acknowledges this is "warranted (it's the main binary)." This is not a problem to solve.

3. **`roko-primitives` (HDC) leaked into crates**: `roko-primitives` is a dependency of 7 crates. Of those, `roko-compose`, `roko-serve`, `roko-fs`, and `roko-neuro` depend on it behind feature flags (`hdc = ["dep:roko-primitives"]`). This is already well-managed. Only `roko-core`, `roko-learn`, and `roko-dreams` have unconditional dependencies.

4. **No `roko-bus` crate**: The event bus in `roko-runtime/src/event_bus.rs` is ~430 lines. It is used by 4 files: `roko-cli/src/prd.rs`, `roko-cli/src/orchestrate.rs`, `roko-serve/src/routes/prds.rs`, and `roko-core/src/state_hub.rs`. This is modest coupling. Extracting it into a separate crate does not solve a practical problem today.

5. **Role templates live next to template engines**: True. `roko-compose/src/templates/` has 9 role modules alongside the builder code. Separating data from engine is reasonable but does not require a new crate -- a directory restructure within `roko-compose` suffices.

### Honest assessment

The proposal would take the workspace from 29 to 34 crates (adding `roko-bus`, `roko-hdc`, `roko-spi`, `roko-defaults`, `roko-tools`, `roko-compose-core`, `roko-templates` while removing `roko-std` and `roko-compose`; net +5). For a single-developer project with no external consumers, this is pure overhead:

- More `Cargo.toml` files to maintain.
- More `pub use` shims during migration.
- More import paths to remember.
- More CI build graph complexity.
- Zero benefit to the end user, who runs `roko plan run`.

The dependency graph in section 3 is beautifully drawn but solves for a problem (multiple teams working on independent subsystems, substrate/bus swaps, plugin ABI stability) that does not exist and may never exist.

The "non-goals" section (9) says "every new crate and trait boundary must justify its existence with an actual use case within the next 6 months." By this standard, none of the proposed crates pass:
- `roko-bus`: no one is swapping the bus.
- `roko-hdc`: already well-managed behind feature flags.
- `roko-spi`: no plugin ecosystem exists.
- `roko-defaults` / `roko-tools`: no one needs a minimal runtime without builtins.
- `roko-compose-core` / `roko-templates`: no third-party template contributors.

### What to do instead

- **Do nothing** with the crate structure for now. The current coupling is modest and well-managed with feature flags.
- **If any extraction is justified**, it is moving HDC from `roko-primitives` to a standalone `roko-hdc` crate, since `roko-primitives` currently contains two unrelated concerns (HDC vectors and tier routing). But even this can wait.
- **Add the CI dep-check script** (section 11) -- this is cheap and prevents future coupling mistakes regardless of crate structure.

---

## Refinement 21: From-Scratch Redesigns

**Verdict: DEFER (all five rewrites are premature; incremental refactoring is sufficient)**

### What it proposes

Five from-scratch rewrite candidates:
1. `roko-core` kernel: Add `Pulse` as second medium, expand to 7 operators, semver-major bump. 2-3 weeks.
2. `roko-learn` reorganization: Split into 5 focused crates (`roko-episode`, `roko-playbook`, `roko-bandit`, `roko-experiment`, `roko-heuristic`). 2 weeks.
3. Substrate trait rewrite: Add `query(predicate)`, `scan(range)`, `freeze/thaw` for cold tier. 1 week.
4. Gate pipeline: Replace state machine with pure-function composition combinators. 1-2 weeks.
5. `roko-compose` engine: Replace fixed templates with query-driven compose (HDC retrieval). 2 weeks.

Total: ~10 weeks of focused rewrite work.

### What the codebase actually has

1. **`roko-core` kernel** is ~41K lines. The "1 noun + 6 verbs" framing is coherent and well-documented. The Engram type is clean. The six traits are clean. The doc says "the current framing actively misrepresents the system" because there is no Pulse -- but the system does not *have* Pulses. The framing is accurate for what exists. Adding Pulse means building a new concept, not correcting a misrepresentation.

2. **`roko-learn`** is ~36K lines across 42 files. It is large and has mixed concerns (episodes, bandits, cascade routing, experiments, HDC clustering, pattern discovery, efficiency tracking). But splitting it into 5 crates does not make the code better -- it makes it spread across 5 `Cargo.toml` files. The coupling between components (e.g., cascade router using c-factor, efficiency events feeding into episodes) is *feature-level*, not *accidental*. Breaking these apart means adding `pub` APIs and `Bus` subscriptions where direct function calls currently work.

3. **Substrate trait** has `put/get/query/prune/len/is_empty/name`. The proposed additions (`scan`, `freeze/thaw`, `subscribe-style notifications`) serve demurrage and cold-tier graduation, which do not exist. Adding API surface for unbuilt features violates YAGNI.

4. **Gate pipeline** in `roko-gate/` is ~11K lines across 24 files. It has 11 gate implementations, a 7-rung pipeline, and adaptive thresholds. The doc's own verdict: "Maybe. Gates are already working; this is cleaner but not unlocking a specific user-facing capability." Correct. The current gates work. Leave them alone.

5. **`roko-compose` engine** is ~25K lines. The 6-layer prompt builder with role templates works. The proposed "query-driven compose" (assemble prompts from HDC-retrieved Engrams instead of fixed templates) is a fascinating idea but depends on (a) HDC fingerprints being on every Engram (they are not), (b) the Substrate having a `query_similar` method (it does not), and (c) enough Engrams existing in the Substrate to make retrieval useful (unknown).

### Honest assessment

The five rewrites are "build the features from the refinement docs" disguised as "clean up existing code." Let me be specific:

- **Rewrite 2.1 (kernel)**: This is not a rewrite of existing code. It is adding a new concept (Pulse) that does not exist. Call it what it is: a new feature.
- **Rewrite 2.2 (learn)**: This is a crate split. The code does not get better; it gets reorganized. The doc says "no public API break if the CLI retains its current shape" -- correct, which means the user sees no benefit.
- **Rewrite 2.3 (substrate)**: This adds new methods. The existing methods do not change. This is an API extension, not a rewrite.
- **Rewrite 2.4 (gates)**: The doc says "maybe." Trust the maybe.
- **Rewrite 2.5 (compose)**: This is a new feature (query-driven compose). The doc says "short-term the existing engine is fine." Trust that.

The doc's own heuristic for when a rewrite is justified (section 1) requires "at least three of five" criteria. For most candidates, only one or two criteria are met. The doc then argues for the rewrites anyway, which undermines the heuristic.

Section 8 ("what we risk by not committing") argues that incremental patching produces a "Frankenstein." This is a valid concern *if* the features from docs 2-16 are all being built. But demurrage, replication ledger, prediction markets, worldviews, and dream cycles are Phase 2+ or unbuilt. The "Frankenstein" scenario is hypothetical because the features that would cause it are hypothetical.

### What to do instead

- **Do not rewrite anything.** Build the features you want (Pulse, demurrage, etc.) when you want them, as new code alongside existing code. The existing code works.
- **When adding Pulse**: add it as a new type in `roko-core`. Do not rewrite Engram. Let both exist. If they converge on a shared operator trait later, refactor then.
- **When adding `query_similar` to Substrate**: add it as a default method on the trait that returns `Ok(vec![])`. Implementations opt in. No rewrite needed.
- **Leave gates and compose alone** until a specific user-facing problem demands a change.

---

## Cross-Cutting Assessment: Is the "Moat" Framing Honest?

### The core claim

Docs 17-21 collectively argue that Roko's defensibility comes from the *composition* of Substrate + Bus + HDC + demurrage + c-factor + heuristics + plugins + replication ledger, and that this composition is expensive to replicate.

### The honest assessment

The composition *as specified* would indeed be hard to replicate. But the composition does not exist. Here is what actually exists vs. what the moat claim depends on:

| Component | Exists in code | Moat claim depends on |
|---|---|---|
| Substrate (Engram storage) | Yes | Substrate + HDC + demurrage + freeze/thaw integrated |
| Bus (event system) | Yes (2 event types) | Bus as kernel trait with topic-based subscribe, backpressure |
| HDC vectors | Yes | HDC fingerprint on every Engram, query_similar |
| Demurrage | No | Central to memory management and cold-tier |
| c-factor | Partial | Continuously computed, surfaced in dashboards |
| Heuristics with falsifiers | No | Calibrated, commons-shared, prediction-market staked |
| Replication ledger | No | Continuously replicated, publishable |
| Plugin ecosystem | No | 50+ plugins with network effects |
| Worldviews | No | Emergent from heuristic citation |
| Prediction markets | No | Belief price discovery among agents |

Of 10 components the moat depends on, 2 exist fully, 2 exist partially, and 6 do not exist at all.

### The danger

The danger is not that these docs are wrong -- they describe a genuinely interesting system. The danger is that writing moat docs *before building the moat* creates a false sense of progress. Every hour spent specifying the WASM host surface for Tier 5 plugins is an hour not spent making `roko plan run` work better for the single user who exists.

The refinement docs contain 35+ documents totaling tens of thousands of words. The codebase is 177K LOC. A substantial fraction of development effort appears to be going into architecture documents rather than shipping features. The "moat" these docs describe would take years to build. The immediate priority is clear from `CLAUDE.md`:

> After items 10-11, roko can fully self-host: read its own PRDs, generate plans, execute them, validate results, learn from failures, and iterate.

Items 10-11 are "automatic plan generation" and "feedback loop." Neither requires new crates, new kernel types, or new plugin architectures. They require wiring existing code: emit a `PrdPublished` event (already defined) and have a subscriber that calls `prd plan` (already a CLI command). This is a day of work, not a two-month rewrite.

### What the moat actually is today

Roko's real moat today is simpler and more honest:

1. **It works.** `roko plan run` executes agent tasks, runs gates, persists state, and can resume. Most agent frameworks demo but do not ship.
2. **It is in Rust.** Performance, safety, and single-binary deployment are real advantages.
3. **It has a complete gate pipeline.** 11 gates, 7 rungs, adaptive thresholds. This is more verification than any Python agent framework offers.
4. **It has multi-backend LLM support.** Claude, Codex, Cursor, OpenAI-compat, Ollama, Gemini, Perplexity. This flexibility is real.
5. **It self-hosts.** The PRD -> plan -> execute -> gate loop works end-to-end.

That is a real product with real capabilities. The moat docs describe a *future* product that would be even more defensible. The gap between the two is approximately 6-12 months of focused implementation work, not architecture documents.

### Recommendation

1. **Stop writing refinement docs.** 35 is enough. The design is over-specified relative to implementation.
2. **Ship items 10-11 from `CLAUDE.md`.** Automatic plan generation and feedback loop. One week of work. Closes the self-hosting loop completely.
3. **If you want one big-bet feature from these docs, pick HDC-per-Engram (innovation 1.2).** The HDC code exists and works. Adding a fingerprint field to `Engram` and populating it at `Substrate::put` time is tractable and would be genuinely novel.
4. **Defer everything else** (Pulse, demurrage, WASM plugins, replication ledger, kernel rewrite, crate splits) until the self-hosting loop has run for a month and produced real data about what needs improving.
5. **When you do build new features, build them incrementally.** Add methods to existing traits with default implementations. Add types alongside existing types. Do not rewrite working code to accommodate unbuilt features.
