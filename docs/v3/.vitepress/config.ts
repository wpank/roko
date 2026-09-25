import { defineConfig } from 'vitepress'
import { withMermaid } from 'vitepress-plugin-mermaid'
import type { Plugin } from 'vite'

/**
 * Vite plugin that pre-processes markdown files to escape angle-bracket
 * patterns before VitePress/Vue sees them.
 *
 * The docs contain Rust generics (`Vec<Signal>`), placeholders (`<dir>`),
 * and other angle-bracket syntax that Vue's SFC parser treats as unclosed
 * HTML elements.
 *
 * Strategy: before VitePress processes the markdown, scan each line that
 * is NOT inside a fenced code block, and escape lone `<` characters that
 * aren't part of known HTML tags, Vue components, or markdown-it directives.
 */
function escapeRustGenerics(): Plugin {
  return {
    name: 'escape-rust-generics',
    enforce: 'pre',
    transform(code: string, id: string) {
      if (!id.endsWith('.md')) return null

      // Only transform files within our docs directory
      if (!id.includes('/docs/v3/')) return null

      const lines = code.split('\n')
      let inCodeBlock = false
      let changed = false
      const result: string[] = []

      for (const line of lines) {
        const trimmed = line.trimStart()

        // Track fenced code blocks (``` or ~~~)
        if (trimmed.startsWith('```') || trimmed.startsWith('~~~')) {
          inCodeBlock = !inCodeBlock
          result.push(line)
          continue
        }

        if (inCodeBlock) {
          result.push(line)
          continue
        }

        // Skip frontmatter lines
        if (trimmed === '---') {
          result.push(line)
          continue
        }

        // Strategy: only KEEP angle brackets that are clearly HTML/Vue tags.
        // Everything else gets escaped. This is safer than trying to
        // enumerate every non-HTML pattern.
        //
        // Split by inline code segments to avoid escaping inside backticks.
        const parts = line.split(/(`[^`]*`)/g)
        let lineChanged = false

        // Known HTML tags + Vue components that should NOT be escaped
        const keepPattern = /^<\/?(?:a|abbr|address|area|article|aside|audio|b|base|bdi|bdo|blockquote|body|br|button|canvas|caption|cite|code|col|colgroup|data|datalist|dd|del|details|dfn|dialog|div|dl|dt|em|embed|fieldset|figcaption|figure|footer|form|h[1-6]|head|header|hgroup|hr|html|i|iframe|img|input|ins|kbd|label|legend|li|link|main|map|mark|menu|meta|meter|nav|noscript|object|ol|optgroup|option|output|p|picture|pre|progress|q|rp|rt|ruby|s|samp|script|search|section|select|slot|small|source|span|strong|style|sub|summary|sup|table|tbody|td|template|textarea|tfoot|th|thead|time|title|tr|track|u|ul|var|video|wbr|svg|path|circle|rect|line|polyline|polygon|text|g|defs|use|symbol|clippath|CitationRef|CrateLink|InteractiveDiagram|CitationDatabase|ClientOnly|Content|Badge|SpectreHero|ParticleBackground|CrateGraph3D|PipelineFlow3D|SignalLifecycle3D|ArchitectureExplorer|DataFlowAnimator)[\s\/>]/i

        const escapedParts = parts.map((part, idx) => {
          // Odd-indexed parts are inside backticks -- leave alone
          if (idx % 2 === 1) return part

          // Replace all `<` except known HTML tags, Vue components,
          // HTML comments (<!--), and already-escaped (&lt;)
          return part.replace(/<(?!!--)/g, (match, offset) => {
            // Check if already escaped
            if (part.slice(Math.max(0, offset - 3), offset) === '&lt') return match
            // Check if this is a known tag
            const rest = part.slice(offset)
            if (keepPattern.test(rest)) return match
            lineChanged = true
            return '&lt;'
          })
        })

        if (lineChanged) changed = true
        result.push(escapedParts.join(''))
      }

      if (changed) {
        return { code: result.join('\n'), map: null }
      }
      return null
    },
  }
}

// Sidebar section builder: each top-level chapter links to its .md,
// and its depth/ sub-files are listed as collapsed children.
function chapterItem(
  text: string,
  file: string,
  depthDir?: string,
  depthFiles?: { text: string; link: string }[]
) {
  const item: any = { text, link: `/${file}` }
  if (depthDir && depthFiles && depthFiles.length > 0) {
    item.collapsed = true
    item.items = depthFiles.map((f) => ({
      text: f.text,
      link: `/depth/${depthDir}/${f.link}`,
    }))
  }
  return item
}

// Convert kebab-case filenames to Title Case for sidebar display
function titleCase(slug: string): string {
  return slug
    .replace(/^\d+-/, '') // strip leading number prefix
    .split('-')
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ')
}

// Depth file definitions for each chapter.
// Each array entry is [filename-without-md, display-title].

const depth00 = [
  ['vision-and-thesis', 'Vision and Thesis'],
  ['five-layer-taxonomy', 'Five-Layer Taxonomy'],
  ['synapse-traits-12', '12 Synapse Traits'],
  ['substrate-trait', 'Substrate Trait'],
  ['compositional-kinds', 'Compositional Kinds'],
  ['crate-map-and-dependencies', 'Crate Map and Dependencies'],
  ['naming-and-glossary', 'Naming and Glossary'],
  ['configuration-schema', 'Configuration Schema'],
  ['bus-transport-fabric', 'Bus Transport Fabric'],
  ['cognitive-energy-model', 'Cognitive Energy Model'],
  ['cognitive-immune-system', 'Cognitive Immune System'],
  ['cognitive-cross-cuts', 'Cognitive Cross-Cuts'],
  ['attention-as-currency', 'Attention as Currency'],
  ['c-factor-collective-intelligence', 'C-Factor Collective Intelligence'],
  ['autocatalytic-and-cybernetics', 'Autocatalytic and Cybernetics'],
  ['score-7-axis-appraisal', '7-Axis Score Appraisal'],
  ['scorer-gate-router-composer-policy', 'Scorer/Gate/Router/Composer Policy'],
  ['decay-variants-and-tier-matrix', 'Decay Variants and Tier Matrix'],
  ['temporal-knowledge-topology', 'Temporal Knowledge Topology'],
  ['provenance-and-attestation', 'Provenance and Attestation'],
  ['emergent-goal-structures', 'Emergent Goal Structures'],
  ['cross-pollination-innovations', 'Cross-Pollination Innovations'],
  ['synergy-integration-map', 'Synergy Integration Map'],
  ['cross-section-integration-map', 'Cross-Section Integration Map'],
  ['error-handling-recovery', 'Error Handling and Recovery'],
  ['performance-numerical-stability', 'Performance and Numerical Stability'],
  ['comprehensive-test-strategy', 'Comprehensive Test Strategy'],
  ['design-principles-frontier-summary', 'Design Principles Frontier Summary'],
  ['implementation-readiness-audit', 'Implementation Readiness Audit'],
  ['architectural-analysis-improvements', 'Architectural Analysis Improvements'],
]

const depth01 = [
  ['signal-type-anatomy', 'Signal Type Anatomy'],
  ['signal-lifecycle-and-serialization', 'Signal Lifecycle and Serialization'],
  ['signal-provenance-chain', 'Signal Provenance Chain'],
  ['signal-scoring-dimensions', 'Signal Scoring Dimensions'],
  ['pulse-ephemeral-events', 'Pulse Ephemeral Events'],
]

const depth02 = [
  ['cell-trait-contract', 'Cell Trait Contract'],
  ['cell-composition-patterns', 'Cell Composition Patterns'],
  ['cell-context-dual-implementations', 'Cell Context Dual Implementations'],
  ['protocol-ids-9', '9 Protocol IDs'],
]

const depth03 = [
  ['dag-topology', 'DAG Topology'],
  ['parallel-wave-execution', 'Parallel Wave Execution'],
  ['cognitive-cells-7', '7 Cognitive Cells'],
  ['immune-decision-graph-5-stage', '5-Stage Immune Decision Graph'],
  ['cost-state-and-budgets', 'Cost State and Budgets'],
  ['graph-fingerprinting-and-resume', 'Graph Fingerprinting and Resume'],
  ['production-plan-topology', 'Production Plan Topology'],
]

const depth04 = [
  ['engine-convergence', 'Engine Convergence'],
  ['runtime-services-builder', 'RuntimeServices Builder'],
  ['runtime-harness', 'Runtime Harness'],
  ['plan-discovery', 'Plan Discovery'],
  ['plan-phases', 'Plan Phases'],
  ['plan-to-graph-conversion', 'Plan to Graph Conversion'],
  ['unified-task-dag', 'Unified Task DAG'],
  ['parallel-executor-waves', 'Parallel Executor Waves'],
  ['executor-actions', 'Executor Actions'],
  ['flow-lifecycle', 'Flow Lifecycle'],
  ['checkpoint-and-resume', 'Checkpoint and Resume'],
  ['snapshot-recovery', 'Snapshot Recovery'],
  ['event-log-and-episodes', 'Event Log and Episodes'],
  ['merge-queue', 'Merge Queue'],
  ['worktree-isolation', 'Worktree Isolation'],
]

const depth05 = [
  ['agent-trait', 'Agent Trait'],
  ['agent-roles', 'Agent Roles'],
  ['agent-pools', 'Agent Pools'],
  ['provider-adapters-12', '12 Provider Adapters'],
  ['provider-registry', 'Provider Registry'],
  ['dispatcher-architecture', 'Dispatcher Architecture'],
  ['tool-loop-protocol', 'Tool Loop Protocol'],
  ['mcp-integration', 'MCP Integration'],
  ['harness-engineering', 'Harness Engineering'],
  ['safety-layer', 'Safety Layer'],
  ['chat-types', 'Chat Types'],
  ['format-translation', 'Format Translation'],
  ['temperament-profiling', 'Temperament Profiling'],
  ['dual-process-routing', 'Dual-Process Routing'],
  ['extensibility', 'Extensibility'],
  ['creation-sites', 'Creation Sites'],
  ['domain-profiles', 'Domain Profiles'],
  ['cognitive-autonomy-e23', 'Cognitive Autonomy (E23)'],
]

const depth06 = [
  ['system-prompt-builder-9-layer', '9-Layer System Prompt Builder'],
  ['5-stage-assembly-pipeline', '5-Stage Assembly Pipeline'],
  ['enrichment-pipeline-13-step', '13-Step Enrichment Pipeline'],
  ['composer-trait', 'Composer Trait'],
  ['prompt-composer', 'Prompt Composer'],
  ['role-templates-11', '11 Role Templates'],
  ['token-budget-management', 'Token Budget Management'],
  ['symbol-resolution', 'Symbol Resolution'],
  ['active-inference-context-selection', 'Active Inference Context Selection'],
  ['affect-modulated-retrieval', 'Affect-Modulated Retrieval'],
  ['predictive-foraging-mvt', 'Predictive Foraging (MVT)'],
  ['vcg-attention-auction', 'VCG Attention Auction'],
  ['lost-in-the-middle-u-shape', 'Lost-in-the-Middle U-Shape'],
  ['distributed-context-engineering', 'Distributed Context Engineering'],
]

const depth07 = [
  ['gate-trait', 'Gate Trait'],
  ['gate-pipeline', 'Gate Pipeline'],
  ['gate-implementations-19', '19 Gate Implementations'],
  ['7-rung-selector', '7-Rung Selector'],
  ['gate-dispatch-wiring', 'Gate Dispatch Wiring'],
  ['adaptive-thresholds-ema', 'Adaptive Thresholds (EMA)'],
  ['verdicts-as-signals', 'Verdicts as Signals'],
  ['agent-feedback-from-gates', 'Agent Feedback from Gates'],
  ['artifact-store', 'Artifact Store'],
  ['evaluation-lifecycle', 'Evaluation Lifecycle'],
  ['process-reward-models', 'Process Reward Models'],
  ['autonomous-eval-generation', 'Autonomous Eval Generation'],
  ['evoskills', 'EvoSkills'],
  ['ratcheting', 'Ratcheting'],
  ['forensic-ai-causal-replay', 'Forensic AI Causal Replay'],
]

const depth08 = [
  ['episode-logger', 'Episode Logger'],
  ['cascade-router', 'Cascade Router'],
  ['experiments-ab', 'A/B Experiments'],
  ['playbook-store', 'Playbook Store'],
  ['bandits-ucb-thompson-linucb', 'Bandits (UCB, Thompson, LinUCB)'],
  ['thompson-sampling-drift', 'Thompson Sampling Drift'],
  ['hdc-clustering', 'HDC Clustering'],
  ['c-factor-governance', 'C-Factor Governance'],
  ['autocatalytic-compounding', 'Autocatalytic Compounding'],
  ['hindsight-adjustments', 'Hindsight Adjustments'],
  ['cost-normalization', 'Cost Normalization'],
  ['task-metrics-and-baselines', 'Task Metrics and Baselines'],
  ['provider-health-circuit-breaker', 'Provider Health Circuit Breaker'],
  ['stability-mechanisms', 'Stability Mechanisms'],
  ['regression-detection', 'Regression Detection'],
  ['pattern-discovery-trigram', 'Pattern Discovery (Trigram)'],
  ['collective-calibration', 'Collective Calibration'],
  ['pareto-frontier-pruning', 'Pareto Frontier Pruning'],
  ['skill-library-voyager', 'Skill Library (Voyager)'],
  ['self-improvement-frameworks', 'Self-Improvement Frameworks'],
  ['8-missing-feedback-loops', 'Missing Feedback Loops'],
]

const depth09 = [
  ['knowledge-store-neurostore', 'Knowledge Store (NeuroStore)'],
  ['six-knowledge-types', '6 Knowledge Types'],
  ['four-validation-tiers', '4 Validation Tiers'],
  ['4-tier-distillation-pipeline', '4-Tier Distillation Pipeline'],
  ['hdc-vsa-foundations', 'HDC/VSA Foundations'],
  ['hdc-knowledge-encoding', 'HDC Knowledge Encoding'],
  ['hdc-operations', 'HDC Operations'],
  ['ebbinghaus-decay-with-tier', 'Ebbinghaus Decay with Tier'],
  ['type-half-lives', 'Type Half-Lives'],
  ['temporal-query-gc', 'Temporal Query and GC'],
  ['knowledge-query-api', 'Knowledge Query API'],
  ['cross-domain-hdc-transfer', 'Cross-Domain HDC Transfer'],
  ['knowledge-backup-restore', 'Knowledge Backup and Restore'],
  ['somatic-integration', 'Somatic Integration'],
  ['antiknowledge-challenge', 'Anti-Knowledge Challenge'],
  ['false-positive-math', 'False Positive Math'],
  ['library-of-babel', 'Library of Babel'],
]

const depth10 = [
  ['three-phase-cycle', 'Three-Phase Cycle'],
  ['hypnagogia-engine', 'Hypnagogia Engine'],
  ['nrem-replay', 'NREM Replay'],
  ['rem-imagination', 'REM Imagination'],
  ['consolidation-and-staging', 'Consolidation and Staging'],
  ['scheduling-and-triggers', 'Scheduling and Triggers'],
  ['dream-journals', 'Dream Journals'],
  ['dream-routing-advice', 'Dream Routing Advice'],
  ['hdc-counterfactual-synthesis', 'HDC Counterfactual Synthesis'],
  ['sleep-time-compute', 'Sleep-Time Compute'],
  ['threat-simulation', 'Threat Simulation'],
  ['divergence-and-alpha', 'Divergence and Alpha'],
  ['inner-worlds-and-rendering', 'Inner Worlds and Rendering'],
  ['hauntology-in-dreams', 'Hauntology in Dreams'],
  ['oneirography', 'Oneirography'],
  ['cross-system-integration', 'Cross-System Integration'],
  ['dream-evolution', 'Dream Evolution'],
  ['advanced-dream-concepts', 'Advanced Dream Concepts'],
]

const depth11 = [
  ['daimon-state-and-affect-engine', 'Daimon State and Affect Engine'],
  ['pad-vector', 'PAD Vector'],
  ['occ-scherer-appraisal', 'OCC/Scherer Appraisal'],
  ['somatic-markers-damasio', 'Somatic Markers (Damasio)'],
  ['six-behavioral-states', '6 Behavioral States'],
  ['8-dimensional-strategy-space', '8-Dimensional Strategy Space'],
  ['alma-three-layer-temporal', 'ALMA Three-Layer Temporal'],
  ['behavioral-state-to-tier-routing', 'Behavioral State to Tier Routing'],
  ['energy-accounting', 'Energy Accounting'],
  ['mood-congruent-memory', 'Mood-Congruent Memory'],
  ['15-percent-contrarian-retrieval', '15% Contrarian Retrieval'],
  ['collective-emotional-contagion', 'Collective Emotional Contagion'],
  ['coding-agent-integration', 'Coding Agent Integration'],
  ['integration-points', 'Integration Points'],
]

const depth12 = [
  ['threat-model', 'Threat Model'],
  ['defense-in-depth', 'Defense in Depth'],
  ['corrigibility-5-head', '5-Head Corrigibility'],
  ['taint-tracking-ifc', 'Taint Tracking (IFC)'],
  ['sandboxing-5-level', '5-Level Sandboxing'],
  ['capability-tokens', 'Capability Tokens'],
  ['permits-allowlists', 'Permits and Allowlists'],
  ['cognitive-kernel-safety', 'Cognitive Kernel Safety'],
  ['prompt-security', 'Prompt Security'],
  ['loop-detection', 'Loop Detection'],
  ['audit-chain', 'Audit Chain'],
  ['witness-dag', 'Witness DAG'],
  ['forensic-ai', 'Forensic AI'],
  ['formal-verification', 'Formal Verification'],
  ['temporal-logic', 'Temporal Logic'],
  ['adaptive-risk', 'Adaptive Risk'],
  ['mev-protection', 'MEV Protection'],
]

const depth13 = [
  ['lens-protocol', 'Lens Protocol'],
  ['built-in-executors-11', '11 Built-in Executors'],
  ['event-variants-39', '39 Event Variants'],
  ['statehub-projections', 'StateHub Projections'],
  ['breaker-controls', 'Breaker Controls'],
  ['observability-and-telemetry', 'Observability and Telemetry'],
]

const depth14 = [
  ['01-feed-sources', 'Feed Sources'],
  ['02-bus-bridge', 'Bus Bridge'],
  ['03-recipe-dags', 'Recipe DAGs'],
  ['04-lifecycle', 'Lifecycle'],
]

const depth15 = [
  ['01-coordinator-architecture', 'Coordinator Architecture'],
  ['02-chain-event-pipeline', 'Chain Event Pipeline'],
  ['03-space-capability-enforcement', 'Space/Capability Enforcement'],
  ['04-mutual-tls-transport', 'Mutual TLS Transport'],
  ['05-filter-admission-pipeline', 'Filter Admission Pipeline'],
]

const depth16 = [
  ['01-stigmergy-theory', 'Stigmergy Theory'],
  ['02-stigmergy-beyond-termites', 'Stigmergy Beyond Termites'],
  ['03-git-as-stigmergy', 'Git as Stigmergy'],
  ['04-digital-pheromones', 'Digital Pheromones'],
  ['05-pheromone-kinds', 'Pheromone Kinds'],
  ['06-pheromone-scope', 'Pheromone Scope'],
  ['07-agent-mesh-sync', 'Agent Mesh Sync'],
  ['08-morphogenetic-specialization', 'Morphogenetic Specialization'],
  ['09-permissioned-subnets', 'Permissioned Subnets'],
  ['10-stigmergy-scaling', 'Stigmergy Scaling'],
  ['11-exponential-flywheel', 'Exponential Flywheel'],
  ['12-collective-intelligence-metrics', 'Collective Intelligence Metrics'],
  ['13-conductor-integration', 'Conductor Integration'],
]

const depth17 = [
  ['invitation-membership', 'Invitation and Membership'],
  ['coordination-patterns', 'Coordination Patterns'],
  ['knowledge-pheromone-flows', 'Knowledge and Pheromone Flows'],
  ['bus-publication', 'Bus Publication'],
  ['privacy-filtering', 'Privacy Filtering'],
]

const depth18 = [
  ['01-connect-contract', 'Connect Contract'],
  ['02-wire-protocol', 'Wire Protocol'],
  ['03-relay-client', 'Relay Client'],
  ['04-subscription-relay', 'Subscription Relay'],
]

const depth19 = [
  ['tool-architecture', 'Tool Architecture'],
  ['tool-categories', 'Tool Categories'],
  ['tool-profiles', 'Tool Profiles'],
  ['builtin-tools', 'Built-in Tools'],
  ['mcp-architecture', 'MCP Architecture'],
  ['mcp-github', 'MCP GitHub'],
  ['plugin-sdk', 'Plugin SDK'],
  ['plugin-loading-and-admission', 'Plugin Loading and Admission'],
  ['wasm-hooks-23', '23 WASM Hooks'],
  ['safety-hooks', 'Safety Hooks'],
  ['event-sources', 'Event Sources'],
  ['service-integrations', 'Service Integrations'],
  ['tool-testing', 'Tool Testing'],
]

const depth20 = [
  ['01-pipeline-stages', 'Pipeline Stages'],
  ['02-cache', 'Cache'],
  ['03-cost-accounting', 'Cost Accounting'],
  ['04-backpressure', 'Backpressure'],
  ['05-handles-batches', 'Handles and Batches'],
  ['06-routing-research', 'Routing Research'],
]

const depth21 = [
  ['01-schema-sections', 'Schema Sections'],
  ['02-provenance-internals', 'Provenance Internals'],
  ['03-migration-mechanics', 'Migration Mechanics'],
  ['04-hot-reload-protocol', 'Hot Reload Protocol'],
  ['05-presets-and-profiles', 'Presets and Profiles'],
  ['06-env-registry', 'Env Registry'],
]

const depth22 = [
  ['projection-contract', 'Projection Contract'],
  ['openapi-surface-routes', 'OpenAPI Surface Routes'],
  ['surface-events', 'Surface Events'],
  ['legacy-tab-mapping', 'Legacy Tab Mapping'],
]

const depth23 = [
  ['cost-tracking', 'Cost Tracking'],
  ['budget-enforcement', 'Budget Enforcement'],
  ['agent-economy', 'Agent Economy'],
]

const depth24 = [
  ['middleware', 'Middleware'],
  ['tokens', 'Tokens'],
  ['cli-credentials', 'CLI Credentials'],
]

const depth25 = [
  ['01-tab-architecture', 'Tab Architecture'],
  ['02-statehub-bridge', 'StateHub Bridge'],
  ['03-rosedust-palette', 'Rosedust Palette'],
  ['04-spectre-creature', 'Spectre Creature'],
  ['05-spectre-rendering', 'Spectre Rendering'],
  ['06-spectre-collective', 'Spectre Collective'],
  ['07-sonification', 'Sonification'],
  ['08-a2ui-generative', 'A2UI Generative'],
]

const depth26 = [
  ['01-route-inventory-376', 'Route Inventory (376)'],
  ['02-sse-websocket', 'SSE and WebSocket'],
  ['03-openapi-spec', 'OpenAPI Spec'],
  ['04-sidecar-api', 'Sidecar API'],
  ['05-middleware-stack', 'Middleware Stack'],
]

const depth27 = [
  ['01-acp-protocol', 'ACP Protocol'],
  ['02-cursor-integration', 'Cursor Integration'],
  ['03-budget-enforcement', 'Budget Enforcement'],
  ['04-mutation-consent-experiments', 'Mutation Consent and Experiments'],
]

const depth28 = [
  ['01-command-reference', 'Command Reference'],
  ['02-scaffolders-roko-new', 'Scaffolders (roko new)'],
  ['03-progressive-help-explain', 'Progressive Help and Explain'],
  ['04-ide-integration', 'IDE Integration'],
  ['05-sdk-developer-ux', 'SDK Developer UX'],
  ['06-completions-and-shell', 'Completions and Shell'],
]

const depth29 = [
  ['three-cognitive-speeds-t0-t1-t2', 'Three Cognitive Speeds (T0/T1/T2)'],
  ['dual-process-t0-t1-t2', 'Dual Process (T0/T1/T2)'],
  ['gamma-reactive-loop', 'Gamma Reactive Loop'],
  ['theta-reflective-loop', 'Theta Reflective Loop'],
  ['coala-9-step-pipeline', 'CoALA 9-Step Pipeline'],
  ['adaptive-clock', 'Adaptive Clock'],
  ['attention-auction-and-gating', 'Attention Auction and Gating'],
  ['active-inference-compute-allocation', 'Active Inference Compute Allocation'],
  ['active-inference-state-space', 'Active Inference State Space'],
  ['delta-consolidation-loop', 'Delta Consolidation Loop'],
  ['chain-heartbeat-variant', 'Chain Heartbeat Variant'],
  ['universal-loop-mapping', 'Universal Loop Mapping'],
  ['16-t0-probes', 'T0 Probes'],
]

const depth30 = [
  ['conductor-architecture', 'Conductor Architecture'],
  ['watcher-ensemble-12', '12-Watcher Ensemble'],
  ['circuit-breaker', 'Circuit Breaker'],
  ['diagnosis-engine', 'Diagnosis Engine'],
  ['health-monitors', 'Health Monitors'],
  ['cognitive-signals', 'Cognitive Signals'],
  ['adaptive-timeouts-state-machine', 'Adaptive Timeouts State Machine'],
  ['stuck-detection', 'Stuck Detection'],
  ['graduated-interventions', 'Graduated Interventions'],
  ['process-supervision-wiring', 'Process Supervision Wiring'],
  ['anomaly-detection-learning', 'Anomaly Detection Learning'],
  ['conductor-learning-federation', 'Conductor Learning Federation'],
  ['good-regulator-self-model', 'Good Regulator Self-Model'],
  ['ooda-cybernetic-loop', 'OODA Cybernetic Loop'],
  ['yerkes-dodson-pressure', 'Yerkes-Dodson Pressure'],
  ['production-failure-catalog', 'Production Failure Catalog'],
]

const depth31 = [
  ['01-rsi-taxonomy', 'RSI Taxonomy'],
  ['02-replan-loop', 'Replan Loop'],
  ['03-grasp-admission', 'GRASP Admission'],
  ['04-autocatalytic-compounding', 'Autocatalytic Compounding'],
  ['05-dgm-and-adas', 'DGM and ADAS'],
  ['06-dogfood-evidence', 'Dogfood Evidence'],
]

const depth32 = [
  ['docker', 'Docker'],
  ['cloud-railway', 'Cloud: Railway'],
  ['cloud-fly-io', 'Cloud: Fly.io'],
  ['daemon-systemd-linux', 'Daemon: systemd (Linux)'],
  ['daemon-launchd-macos', 'Daemon: launchd (macOS)'],
  ['native-x86-arm', 'Native x86/ARM'],
  ['wasm-browser-edge', 'WASM Browser/Edge'],
  ['edge-embedded', 'Edge Embedded'],
  ['port-allocation', 'Port Allocation'],
  ['secret-management', 'Secret Management'],
  ['production-hardening', 'Production Hardening'],
  ['packaging-and-distribution', 'Packaging and Distribution'],
  ['subscription-configuration', 'Subscription Configuration'],
  ['multi-repo-coordination', 'Multi-Repo Coordination'],
  ['remote-orchestrator', 'Remote Orchestrator'],
]

const depth33 = [
  ['01-functor-model', 'Functor Model'],
  ['02-memory-functor', 'Memory Functor'],
  ['03-daimon-functor', 'Daimon Functor'],
  ['04-dreams-functor', 'Dreams Functor'],
  ['05-safety-functor', 'Safety Functor'],
  ['06-arbitration', 'Arbitration'],
  ['07-cascade', 'Cascade'],
]

const depth34 = [
  ['tree-sitter-parsing', 'Tree-sitter Parsing'],
  ['symbol-extraction', 'Symbol Extraction'],
  ['dependency-graph', 'Dependency Graph'],
  ['pagerank-symbol-importance', 'PageRank Symbol Importance'],
  ['hdc-fingerprints', 'HDC Fingerprints'],
  ['context-assembly-from-code', 'Context Assembly from Code'],
  ['mcp-context-server', 'MCP Context Server'],
  ['index-db-scaling', 'Index DB Scaling'],
  ['language-providers', 'Language Providers'],
  ['snapshot-optimization', 'Snapshot Optimization'],
  ['vision', 'Vision'],
]

const depth35 = [
  ['newcomer-overview', 'Newcomer Overview'],
  ['crate-dependency-graph', 'Crate Dependency Graph'],
  ['data-flow-diagrams', 'Data Flow Diagrams'],
  ['design-decisions-and-rationale', 'Design Decisions and Rationale'],
]

const depth36 = [
  ['agent-creation', 'Agent Creation'],
  ['new-agent-creation', 'New Agent Creation'],
  ['agent-onboarding-flow', 'Agent Onboarding Flow'],
  ['agent-deletion-8-step', '8-Step Agent Deletion'],
  ['provisioning', 'Provisioning'],
  ['funding-and-budgets', 'Funding and Budgets'],
  ['configuration-operator-model', 'Configuration Operator Model'],
  ['knowledge-demurrage', 'Knowledge Demurrage'],
  ['ebbinghaus-for-knowledge', 'Ebbinghaus for Knowledge'],
  ['knowledge-transfer-via-mesh', 'Knowledge Transfer via Mesh'],
  ['knowledge-backup-export', 'Knowledge Backup and Export'],
  ['selective-restore', 'Selective Restore'],
  ['academic-foundations', 'Academic Foundations'],
]

const depth37 = [
  ['01-reputation-ema', 'Reputation EMA'],
  ['02-discipline-states', 'Discipline States'],
  ['03-slash-rates', 'Slash Rates'],
  ['04-recovery-paths', 'Recovery Paths'],
  ['05-tracerank', 'TraceRank'],
  ['06-fork-attribution', 'Fork Attribution'],
  ['07-passports', 'Passports'],
  ['08-delegation', 'Delegation'],
  ['09-collusion-detection', 'Collusion Detection'],
  ['10-sybil-detection', 'Sybil Detection'],
  ['11-interaction-graph', 'Interaction Graph'],
  ['12-pricing-tiers', 'Pricing Tiers'],
  ['13-validation-registry', 'Validation Registry'],
  ['14-knowledge-registry', 'Knowledge Registry'],
  ['15-vickrey-auction', 'Vickrey Auction'],
]

const depth38 = [
  ['01-vision', 'Vision'],
  ['02-oracle-trait', 'Oracle Trait'],
  ['03-coding-oracles', 'Coding Oracles'],
  ['04-research-oracles', 'Research Oracles'],
  ['05-attestation', 'Attestation'],
  ['06-hdc-analysis', 'HDC Analysis'],
  ['07-signal-metabolism', 'Signal Metabolism'],
  ['08-causal-discovery', 'Causal Discovery'],
  ['09-predictive-geometry', 'Predictive Geometry'],
  ['10-adversarial-robustness', 'Adversarial Robustness'],
  ['11-somatic-multiscale', 'Somatic Multiscale'],
  ['12-predictive-foraging', 'Predictive Foraging'],
  ['13-sheaf-tropical', 'Sheaf Tropical'],
]

const depth39 = [
  ['00-lifecycle-and-finite-agency', 'Lifecycle and Finite Agency'],
  ['01-memory-consolidation', 'Memory Consolidation'],
  ['02-affective-computing', 'Affective Computing'],
  ['03-dreams-and-offline-learning', 'Dreams and Offline Learning'],
  ['04-coordination-and-multi-agent', 'Coordination and Multi-Agent'],
  ['05-biological-analogues', 'Biological Analogues'],
  ['06-self-learning-systems', 'Self-Learning Systems'],
  ['07-context-engineering', 'Context Engineering'],
  ['08-security-and-provenance', 'Security and Provenance'],
  ['09-hdc-vsa', 'HDC/VSA'],
  ['10-market-microstructure', 'Market Microstructure'],
  ['11-streaming-algorithms', 'Streaming Algorithms'],
  ['12-signal-processing', 'Signal Processing'],
  ['13-philosophy', 'Philosophy'],
  ['14-agent-harnesses-and-tool-use', 'Agent Harnesses and Tool Use'],
  ['15-cybernetics-and-vsm', 'Cybernetics and VSM'],
  ['16-active-inference', 'Active Inference'],
  ['17-process-reward-models', 'Process Reward Models'],
  ['18-collective-intelligence', 'Collective Intelligence'],
  ['19-regulatory-compliance', 'Regulatory Compliance'],
  ['20-cognitive-architectures', 'Cognitive Architectures'],
  ['21-mechanism-design', 'Mechanism Design'],
  ['22-protocol-standards', 'Protocol Standards'],
  ['23-generational-and-evolutionary', 'Generational and Evolutionary'],
  ['24-additions-2025-2026', 'Additions 2025-2026'],
  ['25-research-to-runtime', 'Research to Runtime'],
]

// Build sidebar depth items from [filename, title] arrays
function depthItems(arr: string[][]): { text: string; link: string }[] {
  return arr.map(([file, title]) => ({ text: title, link: file }))
}

export default withMermaid(
  defineConfig({
    title: 'Roko \u2014 Agent Toolkit',
    description:
      'Documentation for Roko, a Rust toolkit for building self-improving AI agents',

    appearance: 'force-dark',

    head: [
      ['link', { rel: 'icon', type: 'image/svg+xml', href: '/favicon.svg' }],
      ['meta', { name: 'theme-color', content: '#c77d8f' }],
    ],

    lastUpdated: true,
    cleanUrls: true,

    // The existing 400+ markdown files contain cross-references using
    // relative paths (../../26-HTTP, ./../../11-AUTONOMY, etc.) that
    // don't match VitePress's flat routing. Ignore dead links during
    // build rather than requiring bulk edits to existing documentation.
    ignoreDeadLinks: true,

    themeConfig: {
      logo: '/logo.svg',
      siteTitle: 'Roko',

      search: {
        provider: 'local',
        options: {
          detailedView: true,
        },
      },

      nav: [
        { text: 'Guide', link: '/35-ARCHITECTURE' },
        { text: 'Specification', link: '/00-INDEX' },
        { text: 'Explorer', link: '/explorer/' },
        {
          text: 'Crates',
          items: [
            { text: 'roko-core', link: '/depth/00-architecture/synapse-traits-12' },
            { text: 'roko-agent', link: '/05-AGENT' },
            { text: 'roko-graph', link: '/03-GRAPH' },
            { text: 'roko-cli', link: '/28-CLI' },
            { text: 'roko-serve', link: '/26-HTTP-API' },
          ],
        },
        { text: 'References', link: '/REFERENCES' },
      ],

      sidebar: {
        '/': [
          {
            text: 'Getting Started',
            items: [
              chapterItem('Architecture Guide', '35-ARCHITECTURE', '35-architecture', depthItems(depth35)),
              chapterItem('Master Index', '00-INDEX', '00-architecture', depthItems(depth00)),
              chapterItem('Signal', '01-SIGNAL', '01-signal', depthItems(depth01)),
              chapterItem('Cell', '02-CELL', '02-cell', depthItems(depth02)),
              chapterItem('Graph', '03-GRAPH', '03-graph', depthItems(depth03)),
            ],
          },
          {
            text: 'Explorer',
            items: [
              { text: 'Overview', link: '/explorer/' },
              { text: 'Architecture Explorer', link: '/explorer/architecture' },
              { text: 'Data Flow Animator', link: '/explorer/data-flow' },
              { text: 'Crate Map', link: '/explorer/crate-map' },
            ],
          },
          {
            text: 'Execution',
            items: [
              chapterItem('Execution Engine', '04-EXECUTION', '04-execution', depthItems(depth04)),
              chapterItem('Agent', '05-AGENT', '05-agent', depthItems(depth05)),
              chapterItem('Composition', '06-COMPOSITION', '06-composition', depthItems(depth06)),
              chapterItem('Gates', '07-GATES', '07-gates', depthItems(depth07)),
            ],
          },
          {
            text: 'Intelligence',
            items: [
              chapterItem('Learning', '08-LEARNING', '08-learning', depthItems(depth08)),
              chapterItem('Memory', '09-MEMORY', '09-memory', depthItems(depth09)),
              chapterItem('Dreams', '10-DREAMS', '10-dreams', depthItems(depth10)),
              chapterItem('Affect', '11-AFFECT', '11-affect', depthItems(depth11)),
              chapterItem('Cross-Cuts', '33-CROSS-CUTS', '33-cross-cuts', depthItems(depth33)),
            ],
          },
          {
            text: 'Safety',
            items: [
              chapterItem('Safety', '12-SAFETY', '12-safety', depthItems(depth12)),
              chapterItem('Auth', '24-AUTH', '24-auth', depthItems(depth24)),
            ],
          },
          {
            text: 'Infrastructure',
            items: [
              chapterItem('Telemetry', '13-TELEMETRY', '13-telemetry', depthItems(depth13)),
              chapterItem('Feeds and Recipes', '14-FEEDS-RECIPES', '14-feeds', depthItems(depth14)),
              chapterItem('Triggers', '15-TRIGGERS', '15-triggers', depthItems(depth15)),
              chapterItem('Coordination', '16-COORDINATION', '16-coordination', depthItems(depth16)),
              chapterItem('Groups', '17-GROUPS', '17-groups', depthItems(depth17)),
              chapterItem('Connectivity', '18-CONNECTIVITY', '18-connectivity', depthItems(depth18)),
              chapterItem('Tools and Plugins', '19-TOOLS-PLUGINS', '19-tools', depthItems(depth19)),
              chapterItem('Gateway', '20-GATEWAY', '20-gateway', depthItems(depth20)),
              chapterItem('Configuration', '21-CONFIG', '21-config', depthItems(depth21)),
            ],
          },
          {
            text: 'Surfaces',
            items: [
              chapterItem('Named Surfaces', '22-SURFACES', '22-surfaces', depthItems(depth22)),
              chapterItem('TUI', '25-TUI', '25-tui', depthItems(depth25)),
              chapterItem('HTTP API', '26-HTTP-API', '26-http', depthItems(depth26)),
              chapterItem('ACP', '27-ACP', '27-acp', depthItems(depth27)),
              chapterItem('CLI', '28-CLI', '28-cli', depthItems(depth28)),
            ],
          },
          {
            text: 'Workflows',
            items: [
              chapterItem('Heartbeat', '29-HEARTBEAT', '29-heartbeat', depthItems(depth29)),
              chapterItem('Conductor', '30-CONDUCTOR', '30-conductor', depthItems(depth30)),
              chapterItem('Self-Hosting', '31-SELF-HOSTING', '31-self-hosting', depthItems(depth31)),
              chapterItem('Deployment', '32-DEPLOYMENT', '32-deployment', depthItems(depth32)),
              chapterItem('Code Intelligence', '34-CODE-INTELLIGENCE', '34-code-intel', depthItems(depth34)),
              chapterItem('Lifecycle', '36-LIFECYCLE', '36-lifecycle', depthItems(depth36)),
            ],
          },
          {
            text: 'Economy',
            items: [
              chapterItem('Payments', '23-PAYMENTS-ECONOMY', '23-payments', depthItems(depth23)),
              chapterItem('Shared Economy', '37-SHARED-ECONOMY', '37-shared-economy', depthItems(depth37)),
            ],
          },
          {
            text: 'Research',
            items: [
              chapterItem('Signal Analysis', '38-SIGNAL-ANALYSIS', '38-signal-analysis', depthItems(depth38)),
              chapterItem('Roadmap', '39-ROADMAP'),
              chapterItem('References', 'REFERENCES', '39-references', depthItems(depth39)),
            ],
          },
        ],
      },

      socialLinks: [
        { icon: 'github', link: 'https://github.com/nunchi/roko' },
      ],

      editLink: {
        pattern: 'https://github.com/nunchi/roko/edit/main/docs/v3/:path',
        text: 'Edit this page on GitHub',
      },

      footer: {
        message: 'Built with agents that build themselves.',
        copyright: 'Copyright 2024-2026 Nunchi',
      },

      outline: {
        level: [2, 3],
        label: 'On this page',
      },
    },

    markdown: {
      lineNumbers: true,
      image: {
        lazyLoading: true,
      },
      config(md) {
        // Disable raw HTML pass-through in markdown-it. The docs contain
        // hundreds of Rust generics (Vec<Signal>, Option<String>), placeholder
        // syntax (<dir>, <subcommand>), and other angle brackets that Vue's
        // SFC compiler would interpret as unclosed HTML elements.
        //
        // Vue components in markdown files use <script setup> blocks which
        // VitePress handles separately from markdown-it's HTML processing.
        // The escapeRustGenerics Vite plugin provides additional safety.
        md.disable(['html_inline', 'html_block'])
      },
    },

    // Mermaid configuration
    mermaid: {
      theme: 'dark',
      themeVariables: {
        primaryColor: '#c77d8f',
        primaryTextColor: '#e5e7eb',
        primaryBorderColor: '#8b5e6b',
        lineColor: '#8b5e6b',
        secondaryColor: '#1a1726',
        tertiaryColor: '#12101a',
        background: '#0a0a0f',
        mainBkg: '#1a1726',
        nodeBkg: '#1a1726',
        nodeBorder: '#8b5e6b',
        clusterBkg: '#12101a',
        clusterBorder: '#8b5e6b',
        titleColor: '#e5e7eb',
        edgeLabelBackground: '#12101a',
        fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, monospace',
      },
    },

    mermaidPlugin: {
      class: 'mermaid-diagram',
    },

    vite: {
      plugins: [escapeRustGenerics()],
      server: {
        fs: {
          allow: ['..'],
        },
      },
      ssr: {
        noExternal: ['three'],
      },
      optimizeDeps: {
        include: ['three'],
      },
    },
  })
)
