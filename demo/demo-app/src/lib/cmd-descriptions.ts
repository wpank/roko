export const CMD_DESCRIPTIONS: Record<string, string> = {
  'status':
    'Queries the signal store and reports counts across episodes, efficiency metrics, and workspace health.',
  'init':
    'Bootstraps a .roko/ directory with default config, signal store, and learning state. Required once per workspace.',
  'run --plan':
    'Writes a plan (tasks.toml) for the request, shows it, then runs it: tasks in dependency order, each checked by gates.',
  'run plans/':
    'Runs an existing plan directory, the same run as `roko plan run`.',
  'run':
    'Runs a prompt: a small change as one task checked by gates (compile/test/clippy), a larger one as a plan written first.',
  'doctor':
    'Diagnoses workspace state: checks config, providers, dependencies, and reports any missing or misconfigured components.',
  'learn all':
    'Displays all learning state: cascade router weights, prompt experiments, gate thresholds, and efficiency metrics.',
  'learn efficiency':
    'Shows per-turn efficiency events: tokens used, cost, latency, and model selection decisions.',
  'config providers list':
    'Lists all configured LLM providers with their status, API key presence, and available models.',
  'config models list':
    'Shows the full model catalog across all providers, with routing tiers and capability tags.',
  'knowledge stats':
    'Reports durable knowledge store statistics: entry count, tiers, memory usage, and last distillation time.',
  'knowledge query':
    'Searches the neuro knowledge store by topic. Returns relevant entries ranked by relevance and recency.',
  'agent list':
    'Lists all registered agents with their status, domain, and last activity timestamp.',
  'agent create':
    'Creates a new agent from a manifest with a name, domain, and optional tool/model constraints.',
  'bench demo':
    'Runs a simulated benchmark comparing naive single-model execution against cascade-routed optimization.',
  'research topic':
    'Dispatches a research agent to investigate a topic using web search and returns structured findings with citations.',
  'research enhance-plan':
    'Improves an existing plan with research findings: context, prior art, and implementation references.',
  'research analyze':
    'Analyzes execution data (episodes, efficiency events) and produces insights about agent performance patterns.',
  'plan generate':
    'Writes a plan from a prompt (plans/<slug>/tasks.toml and plan.md) without running it.',
  'plan list':
    'Lists all implementation plans in the workspace with their completion status and task counts.',
  'plan run':
    'Executes a plan — runs tasks in dependency order, dispatches agents, validates with gates, persists results. The main orchestration loop.',
  'plan validate':
    'Lints tasks.toml without executing. Checks DAG validity, dependency cycles, and missing fields.',
  'dashboard':
    'Opens the interactive ratatui TUI with F1-F7 tabs for monitoring agents, plans, episodes, and metrics.',
  'learn tune gates':
    'Adjusts adaptive gate thresholds based on recent pass/fail rates.',
  'learn tune routing':
    'Updates cascade router model weights based on cost/quality tradeoffs.',
  'config validate':
    'Validates roko.toml against the schema. Reports any invalid fields or missing required sections.',
  'config set-secret':
    'Securely stores an API key or secret, encrypted at rest.',
  'replay':
    'Walks the signal DAG from a given hash, showing the chain of transforms that produced it.',
  'explain':
    'Concept explainer with 3 depth levels (brief/standard/deep). Uses the knowledge store for context.',
};

export function lookupCmdDesc(cmd: string): string | null {
  const stripped = cmd.replace(/^(\.\/target\/(release|debug)\/)?roko\s+/, '');
  for (const [pattern, desc] of Object.entries(CMD_DESCRIPTIONS)) {
    if (stripped.startsWith(pattern)) return desc;
  }
  return null;
}
