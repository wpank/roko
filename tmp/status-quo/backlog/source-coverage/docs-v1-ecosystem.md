# docs/v1 Ecosystem Source Coverage

Plan: `tmp/status-quo/backlog/plans/DOC-v1-ecosystem/tasks.toml`

## Summary

- Source corpus: `docs/v1/08-chain`, `docs/v1/11-safety`, `docs/v1/12-interfaces`, `docs/v1/13-coordination`, `docs/v1/14-identity-economy`, `docs/v1/15-code-intelligence`, `docs/v1/18-tools`, `docs/v1/19-deployment`, `docs/v1/20-technical-analysis`, `docs/v1/21-references`
- Source markdown files covered: 185
- Tasks authored: 10
- Coverage rule: every source path below appears in at least one `[task.context].read_files` entry in the plan.

## Task Coverage

| Task id | Source directory | Docs |
|---|---|---:|
| DOC-V1-ECO-01 | `docs/v1/21-references` | 27 |
| DOC-V1-ECO-02 | `docs/v1/11-safety` | 18 |
| DOC-V1-ECO-03 | `docs/v1/12-interfaces` | 25 |
| DOC-V1-ECO-04 | `docs/v1/18-tools` | 19 |
| DOC-V1-ECO-05 | `docs/v1/14-identity-economy` | 17 |
| DOC-V1-ECO-06 | `docs/v1/20-technical-analysis` | 16 |
| DOC-V1-ECO-07 | `docs/v1/15-code-intelligence` | 12 |
| DOC-V1-ECO-08 | `docs/v1/13-coordination` | 14 |
| DOC-V1-ECO-09 | `docs/v1/08-chain` | 21 |
| DOC-V1-ECO-10 | `docs/v1/19-deployment` | 16 |

## Source Ledger

| Source path | Task id |
|---|---|
| `docs/v1/08-chain/00-vision-and-framing.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/01-korai-chain-spec.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/02-korai-token-economics.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/03-hdc-on-chain-precompile.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/06-erc-8004-registries.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/09-peer-scoring-3-layer.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/10-spore-job-market.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/11-sparrow-power-of-two-choices.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/12-three-hiring-models.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/13-vickrey-reputation-auction.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/14-reputation-system-7-domain.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/15-chainwitness-event-watching.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/16-triage-curiosity-midas.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/17-chain-client-wallet-traits.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/18-mirage-rs-evm-simulator.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/19-chain-agent-heartbeat.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/20-x402-micropayments.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/21-isfr-clearing-settlement.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/23-knowledge-futures-market.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/24-current-status-and-6-contracts.md` | DOC-V1-ECO-09 |
| `docs/v1/08-chain/INDEX.md` | DOC-V1-ECO-09 |
| `docs/v1/11-safety/00-defense-in-depth.md` | DOC-V1-ECO-02 |
| `docs/v1/11-safety/01-capability-tokens.md` | DOC-V1-ECO-02 |
| `docs/v1/11-safety/02-audit-chain.md` | DOC-V1-ECO-02 |
| `docs/v1/11-safety/03-taint-tracking.md` | DOC-V1-ECO-02 |
| `docs/v1/11-safety/04-permits-allowlists.md` | DOC-V1-ECO-02 |
| `docs/v1/11-safety/05-loop-detection.md` | DOC-V1-ECO-02 |
| `docs/v1/11-safety/06-sandboxing.md` | DOC-V1-ECO-02 |
| `docs/v1/11-safety/07-prompt-security.md` | DOC-V1-ECO-02 |
| `docs/v1/11-safety/08-threat-model.md` | DOC-V1-ECO-02 |
| `docs/v1/11-safety/09-adaptive-risk.md` | DOC-V1-ECO-02 |
| `docs/v1/11-safety/10-mev-protection.md` | DOC-V1-ECO-02 |
| `docs/v1/11-safety/11-temporal-logic.md` | DOC-V1-ECO-02 |
| `docs/v1/11-safety/12-witness-dag.md` | DOC-V1-ECO-02 |
| `docs/v1/11-safety/13-formal-verification.md` | DOC-V1-ECO-02 |
| `docs/v1/11-safety/14-cognitive-kernel-safety.md` | DOC-V1-ECO-02 |
| `docs/v1/11-safety/15-forensic-ai.md` | DOC-V1-ECO-02 |
| `docs/v1/11-safety/16-critical-integration-gap.md` | DOC-V1-ECO-02 |
| `docs/v1/11-safety/INDEX.md` | DOC-V1-ECO-02 |
| `docs/v1/12-interfaces/00-cli-overview.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/01-cli-command-reference.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/02-roko-new-scaffolders.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/03-progressive-help-and-explain.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/04-configuration-layered-resolution.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/05-http-api-roko-serve.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/06-websocket-streaming.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/07-rosedust-design-language.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/08-tui-main-layout.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/09-tui-29-screens.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/10-spectre-creature-visualization.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/11-spectre-rendering-per-interface.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/12-spectre-as-collective-display.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/13-web-portal.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/14-agent-onboarding-flow.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/15-generative-interfaces-a2ui.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/16-sonification-reframed.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/17-accessibility-and-current-status.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/18-ux-innovation-proposals.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/19-rust-sdk-developer-ux.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/20-ide-integration-strategy.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/21-user-ux-running-agents.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/22-statehub-projection-layer.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/23-rich-ux-primitives.md` | DOC-V1-ECO-03 |
| `docs/v1/12-interfaces/INDEX.md` | DOC-V1-ECO-03 |
| `docs/v1/13-coordination/00-stigmergy-theory.md` | DOC-V1-ECO-08 |
| `docs/v1/13-coordination/01-stigmergy-beyond-termites.md` | DOC-V1-ECO-08 |
| `docs/v1/13-coordination/02-git-as-stigmergy.md` | DOC-V1-ECO-08 |
| `docs/v1/13-coordination/03-digital-pheromones.md` | DOC-V1-ECO-08 |
| `docs/v1/13-coordination/04-pheromone-kinds.md` | DOC-V1-ECO-08 |
| `docs/v1/13-coordination/05-pheromone-scope.md` | DOC-V1-ECO-08 |
| `docs/v1/13-coordination/06-agent-mesh-sync.md` | DOC-V1-ECO-08 |
| `docs/v1/13-coordination/07-morphogenetic-specialization.md` | DOC-V1-ECO-08 |
| `docs/v1/13-coordination/08-permissioned-subnets.md` | DOC-V1-ECO-08 |
| `docs/v1/13-coordination/09-stigmergy-scaling.md` | DOC-V1-ECO-08 |
| `docs/v1/13-coordination/10-exponential-flywheel.md` | DOC-V1-ECO-08 |
| `docs/v1/13-coordination/11-collective-intelligence-metrics.md` | DOC-V1-ECO-08 |
| `docs/v1/13-coordination/12-current-status-and-gaps.md` | DOC-V1-ECO-08 |
| `docs/v1/13-coordination/INDEX.md` | DOC-V1-ECO-08 |
| `docs/v1/14-identity-economy/00-vision-and-a16z-framing.md` | DOC-V1-ECO-05 |
| `docs/v1/14-identity-economy/01-erc-8004-three-registries.md` | DOC-V1-ECO-05 |
| `docs/v1/14-identity-economy/02-korai-passport.md` | DOC-V1-ECO-05 |
| `docs/v1/14-identity-economy/03-passport-tiers.md` | DOC-V1-ECO-05 |
| `docs/v1/14-identity-economy/04-reputation-7-domain-ema.md` | DOC-V1-ECO-05 |
| `docs/v1/14-identity-economy/05-knowledge-marketplace.md` | DOC-V1-ECO-05 |
| `docs/v1/14-identity-economy/06-commerce-bazaar.md` | DOC-V1-ECO-05 |
| `docs/v1/14-identity-economy/07-mpp-machine-payment-protocol.md` | DOC-V1-ECO-05 |
| `docs/v1/14-identity-economy/08-x402-micropayments.md` | DOC-V1-ECO-05 |
| `docs/v1/14-identity-economy/09-agent-economy.md` | DOC-V1-ECO-05 |
| `docs/v1/14-identity-economy/10-korai-tokenomics.md` | DOC-V1-ECO-05 |
| `docs/v1/14-identity-economy/11-vickrey-reputation-auction.md` | DOC-V1-ECO-05 |
| `docs/v1/14-identity-economy/12-three-hiring-models.md` | DOC-V1-ECO-05 |
| `docs/v1/14-identity-economy/13-isfr-clearing-settlement.md` | DOC-V1-ECO-05 |
| `docs/v1/14-identity-economy/14-knowledge-futures-market.md` | DOC-V1-ECO-05 |
| `docs/v1/14-identity-economy/15-regulatory-moat-and-current-status.md` | DOC-V1-ECO-05 |
| `docs/v1/14-identity-economy/INDEX.md` | DOC-V1-ECO-05 |
| `docs/v1/15-code-intelligence/00-vision.md` | DOC-V1-ECO-07 |
| `docs/v1/15-code-intelligence/01-tree-sitter-parsing.md` | DOC-V1-ECO-07 |
| `docs/v1/15-code-intelligence/02-symbol-extraction.md` | DOC-V1-ECO-07 |
| `docs/v1/15-code-intelligence/03-dependency-graph.md` | DOC-V1-ECO-07 |
| `docs/v1/15-code-intelligence/04-pagerank-symbol-importance.md` | DOC-V1-ECO-07 |
| `docs/v1/15-code-intelligence/05-hdc-fingerprints.md` | DOC-V1-ECO-07 |
| `docs/v1/15-code-intelligence/06-context-assembly-from-code.md` | DOC-V1-ECO-07 |
| `docs/v1/15-code-intelligence/07-mcp-context-server.md` | DOC-V1-ECO-07 |
| `docs/v1/15-code-intelligence/08-index-db-scaling.md` | DOC-V1-ECO-07 |
| `docs/v1/15-code-intelligence/09-snapshot-optimization.md` | DOC-V1-ECO-07 |
| `docs/v1/15-code-intelligence/10-current-status-and-gaps.md` | DOC-V1-ECO-07 |
| `docs/v1/15-code-intelligence/INDEX.md` | DOC-V1-ECO-07 |
| `docs/v1/18-tools/00-tool-architecture.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/01-builtin-tools.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/02-tool-categories.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/03-chain-domain-tools.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/04-safety-hooks.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/05-tool-profiles.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/06-wallet-management.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/07-tool-testing.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/08-service-integrations.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/09-mcp-architecture.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/10-mcp-github.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/11-mcp-slack.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/12-mcp-scripts.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/13-mcp-stdio.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/14-plugin-sdk.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/15-16-agent-templates.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/15-event-sources.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/16-plugin-loading.md` | DOC-V1-ECO-04 |
| `docs/v1/18-tools/INDEX.md` | DOC-V1-ECO-04 |
| `docs/v1/19-deployment/00-packaging-and-distribution.md` | DOC-V1-ECO-10 |
| `docs/v1/19-deployment/01-native-x86-arm.md` | DOC-V1-ECO-10 |
| `docs/v1/19-deployment/02-wasm-browser-edge.md` | DOC-V1-ECO-10 |
| `docs/v1/19-deployment/03-docker.md` | DOC-V1-ECO-10 |
| `docs/v1/19-deployment/04-daemon-launchd-macos.md` | DOC-V1-ECO-10 |
| `docs/v1/19-deployment/05-daemon-systemd-linux.md` | DOC-V1-ECO-10 |
| `docs/v1/19-deployment/06-cloud-fly-io.md` | DOC-V1-ECO-10 |
| `docs/v1/19-deployment/07-edge-embedded.md` | DOC-V1-ECO-10 |
| `docs/v1/19-deployment/08-subscription-configuration.md` | DOC-V1-ECO-10 |
| `docs/v1/19-deployment/09-multi-repo-coordination.md` | DOC-V1-ECO-10 |
| `docs/v1/19-deployment/10-secret-management.md` | DOC-V1-ECO-10 |
| `docs/v1/19-deployment/11-remote-orchestrator.md` | DOC-V1-ECO-10 |
| `docs/v1/19-deployment/12-production-hardening.md` | DOC-V1-ECO-10 |
| `docs/v1/19-deployment/13-current-status-and-port-allocation.md` | DOC-V1-ECO-10 |
| `docs/v1/19-deployment/14-observability-and-telemetry.md` | DOC-V1-ECO-10 |
| `docs/v1/19-deployment/INDEX.md` | DOC-V1-ECO-10 |
| `docs/v1/20-technical-analysis/00-vision-ta-generalized.md` | DOC-V1-ECO-06 |
| `docs/v1/20-technical-analysis/01-oracle-trait.md` | DOC-V1-ECO-06 |
| `docs/v1/20-technical-analysis/02-chain-oracles.md` | DOC-V1-ECO-06 |
| `docs/v1/20-technical-analysis/03-coding-oracles.md` | DOC-V1-ECO-06 |
| `docs/v1/20-technical-analysis/04-research-oracles.md` | DOC-V1-ECO-06 |
| `docs/v1/20-technical-analysis/05-witness-as-ta-generalized.md` | DOC-V1-ECO-06 |
| `docs/v1/20-technical-analysis/06-hyperdimensional-ta.md` | DOC-V1-ECO-06 |
| `docs/v1/20-technical-analysis/07-spectral-liquidity-manifolds.md` | DOC-V1-ECO-06 |
| `docs/v1/20-technical-analysis/08-adaptive-signal-metabolism.md` | DOC-V1-ECO-06 |
| `docs/v1/20-technical-analysis/09-causal-microstructure-discovery.md` | DOC-V1-ECO-06 |
| `docs/v1/20-technical-analysis/10-predictive-geometry-and-resonant-patterns.md` | DOC-V1-ECO-06 |
| `docs/v1/20-technical-analysis/11-adversarial-signal-robustness.md` | DOC-V1-ECO-06 |
| `docs/v1/20-technical-analysis/12-somatic-ta-and-emergent-multiscale.md` | DOC-V1-ECO-06 |
| `docs/v1/20-technical-analysis/13-predictive-foraging-and-active-inference.md` | DOC-V1-ECO-06 |
| `docs/v1/20-technical-analysis/14-sheaf-tropical-geometry.md` | DOC-V1-ECO-06 |
| `docs/v1/20-technical-analysis/INDEX.md` | DOC-V1-ECO-06 |
| `docs/v1/21-references/00-lifecycle-and-finite-agency.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/01-memory-consolidation.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/02-affective-computing.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/03-dreams-and-offline-learning.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/04-coordination-and-multi-agent.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/05-biological-analogues.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/06-self-learning-systems.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/07-context-engineering.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/08-security-and-provenance.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/09-hdc-vsa.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/10-market-microstructure.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/11-streaming-algorithms.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/12-signal-processing.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/13-philosophy.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/14-agent-harnesses-and-tool-use.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/15-cybernetics-and-vsm.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/16-active-inference.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/17-process-reward-models.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/18-collective-intelligence.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/19-regulatory-compliance.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/20-cognitive-architectures.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/21-mechanism-design.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/22-protocol-standards.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/23-generational-and-evolutionary.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/24-additions-2025.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/25-research-to-runtime.md` | DOC-V1-ECO-01 |
| `docs/v1/21-references/INDEX.md` | DOC-V1-ECO-01 |
