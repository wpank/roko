/**
 * Roko Portal — API Type Definitions
 *
 * All types used across the portal that correspond to the roko-serve HTTP/SSE API.
 * Pure data shapes are declared with `type`; extensible contracts use `interface`.
 */

// ---------------------------------------------------------------------------
// Primitive / shared value types
// ---------------------------------------------------------------------------

export type PlanStatus =
  | "pending"
  | "running"
  | "gating"
  | "completed"
  | "failed"
  | "paused"
  | "cancelled";

export type TaskStatus =
  | "pending"
  | "dispatching"
  | "running"
  | "gating"
  | "completed"
  | "failed";

export type AgentStatus = "idle" | "active" | "completed" | "failed" | "stopped";

export type ProviderStatus = "healthy" | "degraded" | "unhealthy" | "unconfigured";

export type CircuitState = "closed" | "open" | "half_open";

export type GatePassResult = "pass" | "fail" | "skip";

export type Severity = "low" | "medium" | "high" | "critical";

export type AlertSeverity = "error" | "warning" | "success" | "info";

export type ConnectionStatus = "connected" | "connecting" | "disconnected" | "error";

export type KnowledgeTier = "transient" | "working" | "consolidated" | "persistent";

export type LearningStage = "static" | "confidence" | "ucb";

export type CfactorTrend = "up" | "down" | "flat";

export type ExperimentStatus = "collecting" | "trending" | "significant" | "concluded";

export type DreamPhase = "hypnagogia" | "imagination" | "consolidation";

export type InboxItemKind =
  | "gate_failure"
  | "safety_incident"
  | "budget_warning"
  | "provider_degraded"
  | "approval_needed"
  | "plan_completed"
  | "cost_anomaly";

// ---------------------------------------------------------------------------
// Core state shapes
// ---------------------------------------------------------------------------

export interface PlanState {
  id: string;
  name: string;
  status: PlanStatus;
  /** Aggregate task progress counters. */
  progress: {
    total: number;
    completed: number;
    failed: number;
  };
  /** Ordered list of task IDs belonging to this plan. */
  tasks: string[];
  /** Accumulated cost in USD for the entire plan. */
  costUsd: number;
  /** Optional spend cap in USD. `null` means unlimited. */
  budgetUsd: number | null;
  startedAt: string | null;
  completedAt: string | null;
}

export interface TaskState {
  id: string;
  planId: string;
  name: string;
  status: TaskStatus;
  /** Parallel execution wave index (0-based). */
  wave: number;
  agentName: string | null;
  model: string | null;
  costUsd: number;
  startedAt: string | null;
  completedAt: string | null;
  /** Task IDs that must complete before this task can start. */
  dependsOn: string[];
  /** Gate results recorded for this task in rung order. */
  gateResults: GateResult[];
  /** Human-readable task description from tasks.toml. */
  description?: string;
  /** Source files this task is expected to produce or modify. */
  files?: string[];
  /** Cognitive tier classification (e.g. "focused", "mechanical", "integrative"). */
  tier?: string;
}

export interface AgentState {
  id: string;
  name: string;
  role: string;
  model: string;
  provider: string;
  status: AgentStatus;
  /** ID of the task currently being executed, if any. */
  currentTask: string | null;
  tokensIn: number;
  tokensOut: number;
  costUsd: number;
  /** Context window utilisation expressed as a percentage 0–100. */
  contextPct: number;
  startedAt: string | null;
}

export interface GateResult {
  id: string;
  taskId: string;
  planId: string;
  gateName: string;
  /** Pipeline rung index, 0–6. */
  rung: 0 | 1 | 2 | 3 | 4 | 5 | 6;
  passed: boolean;
  /** Short human-readable outcome description. */
  summary: string;
  /** Full gate output / log text. */
  output: string;
  durationMs: number;
  timestamp: string;
}

export interface EpisodeEntry {
  id: string;
  agentId: string;
  taskId: string;
  model: string;
  tokensIn: number;
  tokensOut: number;
  costUsd: number;
  /** HDC vector fingerprint hex string for this episode, if computed. */
  hdcFingerprint: string | null;
  gateResult: GatePassResult;
  timestamp: string;
}

export interface ErrorEntry {
  id: string;
  source: string;
  message: string;
  severity: Severity;
  timestamp: string;
  planId?: string;
  taskId?: string;
}

export interface InboxItem {
  id: string;
  kind: InboxItemKind;
  severity: AlertSeverity;
  title: string;
  description: string;
  /** Deep-link URL to the relevant resource in the portal. */
  actionUrl?: string;
  /** ISO timestamp when the item was dismissed; `null` if still active. */
  dismissedAt: string | null;
  timestamp: string;
}

export interface VitalsSnapshot {
  activeAgents: number;
  totalAgents: number;
  /** Gate pass rate across all tasks today, 0–1. */
  gatePassRate: number;
  costToday: number;
  cfactor: number;
  healthyProviders: number;
  totalProviders: number;
  /** Current affect label word (e.g. "focused", "stressed"). */
  affectWord: string;
}

export interface AffectState {
  /** Valence/pleasure dimension, clamped to [-1, 1]. */
  pleasure: number;
  /** Arousal dimension, clamped to [-1, 1]. */
  arousal: number;
  /** Dominance/control dimension, clamped to [-1, 1]. */
  dominance: number;
  /** Human-readable label derived from the PAD coordinates. */
  word: string;
  updatedAt: string;
}

export interface ProviderState {
  id: string;
  name: string;
  status: ProviderStatus;
  modelCount: number;
  requestCount: number;
  /** Request success rate, 0–1. */
  passRate: number;
  avgLatencyMs: number;
  costUsd: number;
  circuitState: CircuitState;
}

export interface LearningState {
  cfactor: number;
  /** Delta relative to previous observation window. */
  cfactorDelta: number;
  cfactorTrend: CfactorTrend;
  learningStage: LearningStage;
  observationCount: number;
  /** Map from tier name to entry count in the knowledge store. */
  knowledgeTierCounts: Record<string, number>;
  activeExperiments: number;
}

export interface KnowledgeEntry {
  id: string;
  content: string;
  tier: KnowledgeTier;
  domain: string;
  /** Confidence score, 0–1. */
  confidence: number;
  tags: string[];
  confirmations: number;
  /** HDC fingerprint hex string, if backfilled. */
  hdcFingerprint: string | null;
  createdAt: string;
}

export interface ExperimentEntry {
  id: string;
  name: string;
  status: ExperimentStatus;
  /** Label for variant A (control). */
  variantA: string;
  /** Label for variant B (treatment). */
  variantB: string;
  sampleCount: number;
  /** p-value from the significance test; `null` while still collecting. */
  pValue: number | null;
  /** `"a"` or `"b"` once concluded; `null` otherwise. */
  winnerVariant: "a" | "b" | null;
  startedAt: string;
  /** Success rate for variant A (0–1). Present when real server data is available. */
  variantARate?: number;
  /** Success rate for variant B (0–1). Present when real server data is available. */
  variantBRate?: number;
  /** All variants (for experiments with more than two). */
  allVariants?: Array<{ id: string; name: string; trials: number; successRate: number }>;
}

export interface DreamJournalEntry {
  id: string;
  phase: DreamPhase;
  promotedCount: number;
  demotedCount: number;
  createdCount: number;
  insightsGained: number;
  durationMs: number;
  timestamp: string;
}

export interface PlaybookEntry {
  id: string;
  /** The condition pattern that triggers this playbook rule. */
  whenCondition: string;
  /** The action text applied when the condition matches. */
  thenAction: string;
  hitCount: number;
  lastAppliedAt: string | null;
  createdAt: string;
}

export interface ConnectionProfile {
  id: string;
  name: string;
  url: string;
  apiKey?: string;
  /** `true` for a local roko-serve instance (localhost). */
  isLocal: boolean;
}

// ---------------------------------------------------------------------------
// DashboardSnapshot — full replacement payload for gap recovery
// ---------------------------------------------------------------------------

export interface DashboardSnapshot {
  plans: Record<string, PlanState>;
  tasks: Record<string, TaskState>;
  agents: Record<string, AgentState>;
  recentGates: GateResult[];
  recentEpisodes: EpisodeEntry[];
  recentErrors: ErrorEntry[];
  inboxItems: InboxItem[];
  vitals: VitalsSnapshot;
  affect: AffectState | null;
  providers: Record<string, ProviderState>;
  learning: LearningState;
}

// ---------------------------------------------------------------------------
// DashboardEvent — discriminated union on `type`
// ---------------------------------------------------------------------------

interface EventBase {
  /** ISO 8601 timestamp when the server emitted this event. */
  timestamp: string;
}

// --- Plan lifecycle ---

export interface PlanStartedEvent extends EventBase {
  type: "plan_started";
  planId: string;
  name: string;
  totalTasks: number;
  budgetUsd: number | null;
}

export interface PlanCompletedEvent extends EventBase {
  type: "plan_completed";
  planId: string;
  costUsd: number;
  durationMs: number;
}

export interface PlanFailedEvent extends EventBase {
  type: "plan_failed";
  planId: string;
  reason: string;
  costUsd: number;
}

export interface PlanPausedEvent extends EventBase {
  type: "plan_paused";
  planId: string;
}

export interface PlanResumedEvent extends EventBase {
  type: "plan_resumed";
  planId: string;
}

// --- Task lifecycle ---

export interface TaskStartedEvent extends EventBase {
  type: "task_started";
  taskId: string;
  planId: string;
  name: string;
  wave: number;
  agentName: string | null;
  model: string | null;
}

export interface TaskCompletedEvent extends EventBase {
  type: "task_completed";
  taskId: string;
  planId: string;
  costUsd: number;
  durationMs: number;
}

export interface TaskFailedEvent extends EventBase {
  type: "task_failed";
  taskId: string;
  planId: string;
  reason: string;
  gateRung: number | null;
}

export interface TaskRestartedEvent extends EventBase {
  type: "task_restarted";
  taskId: string;
  planId: string;
  attempt: number;
}

// --- Gate pipeline ---

export interface GateStartedEvent extends EventBase {
  type: "gate_started";
  taskId: string;
  planId: string;
  gateName: string;
  rung: number;
}

export interface GateResultEvent extends EventBase {
  type: "gate_result";
  result: GateResult;
}

// --- Agent lifecycle ---

export interface AgentSpawnedEvent extends EventBase {
  type: "agent_spawned";
  agentId: string;
  name: string;
  role: string;
  model: string;
  provider: string;
  taskId: string | null;
}

export interface AgentCompletedEvent extends EventBase {
  type: "agent_completed";
  agentId: string;
  tokensIn: number;
  tokensOut: number;
  costUsd: number;
}

export interface AgentOutputEvent extends EventBase {
  type: "agent_output";
  agentId: string;
  taskId: string | null;
  /** Streaming text chunk from the agent. */
  chunk: string;
  /** `true` if this is the final chunk for the current turn. */
  done: boolean;
}

export interface AgentRouteMetricsEvent extends EventBase {
  type: "agent_route_metrics";
  agentId: string;
  model: string;
  provider: string;
  tokensIn: number;
  tokensOut: number;
  latencyMs: number;
  costUsd: number;
}

// --- Cost and budget ---

export interface CostUpdatedEvent extends EventBase {
  type: "cost_updated";
  planId: string | null;
  taskId: string | null;
  agentId: string | null;
  delta: number;
  totalUsd: number;
}

export interface BudgetWarningEvent extends EventBase {
  type: "budget_warning";
  planId: string;
  spentUsd: number;
  budgetUsd: number;
  /** Fraction spent, 0–1. */
  fraction: number;
}

// --- Affect ---

export interface AffectUpdatedEvent extends EventBase {
  type: "affect_updated";
  affect: AffectState;
}

// --- Knowledge ---

export interface KnowledgeEntryCreatedEvent extends EventBase {
  type: "knowledge_entry_created";
  entry: KnowledgeEntry;
}

export interface KnowledgeTierChangedEvent extends EventBase {
  type: "knowledge_tier_changed";
  entryId: string;
  previousTier: KnowledgeTier;
  newTier: KnowledgeTier;
}

// --- Learning / C-factor ---

export interface CfactorUpdatedEvent extends EventBase {
  type: "cfactor_updated";
  cfactor: number;
  delta: number;
  trend: CfactorTrend;
}

export interface LearningStageChangedEvent extends EventBase {
  type: "learning_stage_changed";
  previousStage: LearningStage;
  newStage: LearningStage;
  observationCount: number;
}

// --- Provider health ---

export interface ProviderHealthUpdatedEvent extends EventBase {
  type: "provider_health_updated";
  provider: ProviderState;
}

// --- Experiments ---

export interface ExperimentUpdatedEvent extends EventBase {
  type: "experiment_updated";
  experiment: ExperimentEntry;
}

export interface ExperimentConcludedEvent extends EventBase {
  type: "experiment_concluded";
  experimentId: string;
  winnerVariant: "a" | "b" | null;
  pValue: number;
}

// --- Dreams ---

export interface DreamPhaseChangedEvent extends EventBase {
  type: "dream_phase_changed";
  phase: DreamPhase;
  cycleId: string;
}

export interface DreamCycleCompletedEvent extends EventBase {
  type: "dream_cycle_completed";
  entry: DreamJournalEntry;
}

// --- Errors ---

export interface ErrorOccurredEvent extends EventBase {
  type: "error_occurred";
  error: ErrorEntry;
}

// --- Inbox ---

export interface InboxItemAddedEvent extends EventBase {
  type: "inbox_item_added";
  item: InboxItem;
}

// --- Snapshot (full state replacement for gap recovery) ---

export interface SnapshotEvent extends EventBase {
  type: "snapshot";
  snapshot: DashboardSnapshot;
  /** Server-assigned monotonic cursor the client should store for reconnect. */
  cursor: string;
}

// ---------------------------------------------------------------------------
// Union type
// ---------------------------------------------------------------------------

export type DashboardEvent =
  | PlanStartedEvent
  | PlanCompletedEvent
  | PlanFailedEvent
  | PlanPausedEvent
  | PlanResumedEvent
  | TaskStartedEvent
  | TaskCompletedEvent
  | TaskFailedEvent
  | TaskRestartedEvent
  | GateStartedEvent
  | GateResultEvent
  | AgentSpawnedEvent
  | AgentCompletedEvent
  | AgentOutputEvent
  | AgentRouteMetricsEvent
  | CostUpdatedEvent
  | BudgetWarningEvent
  | AffectUpdatedEvent
  | KnowledgeEntryCreatedEvent
  | KnowledgeTierChangedEvent
  | CfactorUpdatedEvent
  | LearningStageChangedEvent
  | ProviderHealthUpdatedEvent
  | ExperimentUpdatedEvent
  | ExperimentConcludedEvent
  | DreamPhaseChangedEvent
  | DreamCycleCompletedEvent
  | ErrorOccurredEvent
  | InboxItemAddedEvent
  | SnapshotEvent;
