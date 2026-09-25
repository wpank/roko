/**
 * Dashboard store — primary runtime state driven by the SSE event stream.
 *
 * All mutations flow through `applyEvent`. `replaceSnapshot` is called only on
 * gap recovery (the server replays a full materialized snapshot when the client
 * has missed more events than the ring buffer retains).
 *
 * Ring buffer limits mirror the server-side constants in
 * `roko-core/src/dashboard_snapshot.rs` so the two ends stay in sync.
 */

import { create } from "zustand";
import type {
  DashboardSnapshot,
  DashboardEvent,
  PlanState,
  TaskState,
  AgentState,
  GateResult,
  EpisodeEntry,
  ErrorEntry,
  InboxItem,
  VitalsSnapshot,
  AffectState,
  ProviderState,
  LearningState,
} from "@/api/types";

// ---------------------------------------------------------------------------
// Activity feed item
// ---------------------------------------------------------------------------

export interface ActivityItem {
  id: string;
  kind:
    | "plan_started"
    | "plan_completed"
    | "plan_failed"
    | "task_completed"
    | "task_failed"
    | "gate_result"
    | "agent_spawned"
    | "agent_completed"
    | "cost_updated"
    | "budget_warning"
    | "affect_updated"
    | "cfactor_updated"
    | "provider_health_updated"
    | "knowledge_tier_changed"
    | "inbox_item_added"
    | "error_occurred";
  summary: string;
  timestamp: string;
}

const MAX_ACTIVITY = 20;

// ---------------------------------------------------------------------------
// Ring buffer capacity constants
// ---------------------------------------------------------------------------

const MAX_GATES = 256;
const MAX_EPISODES = 128;
const MAX_ERRORS = 64;
const MAX_EVENT_LOG = 200;
const MAX_AGENT_OUTPUT_LINES = 500;

// ---------------------------------------------------------------------------
// Default values for nullable slices
// ---------------------------------------------------------------------------

const DEFAULT_VITALS: VitalsSnapshot = {
  activeAgents: 0,
  totalAgents: 0,
  gatePassRate: 0,
  costToday: 0,
  cfactor: 0,
  healthyProviders: 0,
  totalProviders: 0,
  affectWord: "idle",
};

const DEFAULT_LEARNING: LearningState = {
  cfactor: 0,
  cfactorDelta: 0,
  cfactorTrend: "flat",
  learningStage: "static",
  observationCount: 0,
  knowledgeTierCounts: {},
  activeExperiments: 0,
};

// ---------------------------------------------------------------------------
// Store interface
// ---------------------------------------------------------------------------

interface DashboardStore {
  // Connection
  connectionStatus: "connected" | "connecting" | "disconnected" | "error";
  lastEventId: string | null;

  // State slices
  plans: Record<string, PlanState>;
  tasks: Record<string, TaskState>;
  agents: Record<string, AgentState>;
  /** Ring buffer of streaming text per agent, keyed by agent id. */
  agentOutput: Record<string, string[]>;
  recentGates: GateResult[];
  recentEpisodes: EpisodeEntry[];
  recentErrors: ErrorEntry[];
  inboxItems: InboxItem[];
  vitals: VitalsSnapshot;
  affect: AffectState | null;
  providers: Record<string, ProviderState>;
  learning: LearningState;
  /** Recent significant events for the home page activity feed (newest first, max 20). */
  activityFeed: ActivityItem[];

  // Actions
  setConnectionStatus: (
    status: "connected" | "connecting" | "disconnected" | "error"
  ) => void;
  applyEvent: (event: DashboardEvent) => void;
  replaceSnapshot: (snapshot: DashboardSnapshot) => void;
  appendAgentOutput: (agentId: string, line: string) => void;
  dismissInboxItem: (id: string) => void;
  /**
   * Seed vitals from a REST health check response.  Only updates fields that
   * the SSE stream hasn't already set (i.e. keeps the SSE value if non-zero
   * so live events remain authoritative once the stream is warm).
   */
  seedVitalsFromHealth: (patch: Partial<VitalsSnapshot>) => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Prepend `item` to `arr` and return a new array capped at `max`. */
function prepend<T>(arr: T[], item: T, max: number): T[] {
  const next = [item, ...arr];
  return next.length > max ? next.slice(0, max) : next;
}

/** Push a new activity item to the front of the feed, capped at MAX_ACTIVITY. */
function pushActivity(
  feed: ActivityItem[],
  kind: ActivityItem["kind"],
  summary: string,
  timestamp: string,
): ActivityItem[] {
  // Guard: summary must be a non-empty string; timestamp must be a string.
  if (!summary || typeof summary !== 'string') return feed;
  const safeTimestamp = timestamp && typeof timestamp === 'string' ? timestamp : new Date().toISOString();
  const item: ActivityItem = {
    id: `${kind}-${safeTimestamp}-${Math.random().toString(36).slice(2, 7)}`,
    kind,
    summary,
    timestamp: safeTimestamp,
  };
  return prepend(feed, item, MAX_ACTIVITY);
}

/** Append `item` to `arr` and return a new array capped at `max`. */
function append<T>(arr: T[], item: T, max: number): T[] {
  const next = [...arr, item];
  return next.length > max ? next.slice(next.length - max) : next;
}

/** Upsert a record entry, merging `patch` over the existing value. */
function upsert<T extends object>(
  record: Record<string, T>,
  key: string,
  defaults: T,
  patch: Partial<T>
): Record<string, T> {
  return {
    ...record,
    [key]: { ...(record[key] ?? defaults), ...patch },
  };
}

// ---------------------------------------------------------------------------
// Store implementation
// ---------------------------------------------------------------------------

export const useDashboardStore = create<DashboardStore>((set, get) => ({
  // --- Connection state ---
  connectionStatus: "disconnected",
  lastEventId: null,

  // --- Data slices ---
  plans: {},
  tasks: {},
  agents: {},
  agentOutput: {},
  recentGates: [],
  recentEpisodes: [],
  recentErrors: [],
  inboxItems: [],
  vitals: DEFAULT_VITALS,
  affect: null,
  providers: {},
  learning: DEFAULT_LEARNING,
  activityFeed: [],

  // --- Actions ---

  setConnectionStatus: (status) => set({ connectionStatus: status }),

  appendAgentOutput: (agentId, line) => {
    set((state) => {
      const existing = state.agentOutput[agentId] ?? [];
      const next = append(existing, line, MAX_AGENT_OUTPUT_LINES);
      return { agentOutput: { ...state.agentOutput, [agentId]: next } };
    });
  },

  dismissInboxItem: (id) => {
    set((state) => ({
      inboxItems: state.inboxItems.map((item) =>
        item.id === id
          ? { ...item, dismissedAt: new Date().toISOString() }
          : item
      ),
    }));
  },

  seedVitalsFromHealth: (patch) => {
    set((state) => {
      // For each key in the patch, only apply if the current store value is
      // still at the default (0 / 'idle') so live SSE events stay authoritative.
      const current = state.vitals;
      const next: VitalsSnapshot = { ...current };
      for (const [key, value] of Object.entries(patch) as [keyof VitalsSnapshot, VitalsSnapshot[keyof VitalsSnapshot]][]) {
        const currentValue = current[key];
        // Apply if the store field is at its default value (0 or 'idle')
        if (currentValue === 0 || currentValue === 'idle') {
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          (next as any)[key] = value;
        }
      }
      return { vitals: next };
    });
  },

  replaceSnapshot: (snapshot) => {
    // Guard: ignore null/non-object snapshots from gap recovery payloads.
    if (!snapshot || typeof snapshot !== 'object') return;
    set({
      plans: snapshot.plans ?? {},
      tasks: snapshot.tasks ?? {},
      agents: snapshot.agents ?? {},
      recentGates: snapshot.recentGates ?? [],
      recentEpisodes: snapshot.recentEpisodes ?? [],
      recentErrors: snapshot.recentErrors ?? [],
      inboxItems: snapshot.inboxItems ?? [],
      vitals: snapshot.vitals ?? DEFAULT_VITALS,
      affect: snapshot.affect ?? null,
      providers: snapshot.providers ?? {},
      learning: snapshot.learning ?? DEFAULT_LEARNING,
      // Activity feed is ephemeral — clear it on a full snapshot replacement
      // so stale events don't survive gap recovery.
      activityFeed: [],
    });
  },

  applyEvent: (event) => {
    try {
    // Safety: ignore unknown event types from roko-serve rather than crashing.
    // The server may send event types (e.g. snapshot_rebased, event_log_entry,
    // task_output_appended, episode_recorded) that the portal doesn't handle yet.
    if (!event || typeof event !== 'object' || !('type' in event)) return;

    // Cast to a loose record for safe field access — the server may use
    // snake_case field names while our TypeScript types use camelCase.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const raw = event as any;

    const state = get();
    const now = new Date().toISOString();
    const ts: string = (typeof raw.timestamp === 'string' && raw.timestamp) ? raw.timestamp : now;

    switch (event.type) {
      // -----------------------------------------------------------------------
      // Plan lifecycle
      // -----------------------------------------------------------------------

      case "plan_started": {
        const e = raw;
        const planId = (e.plan_id ?? e.planId ?? '') as string;
        if (!planId) return;
        const tasksTotal = (e.tasks_total ?? e.totalTasks ?? 0) as number;
        const name = (e.name ?? planId) as string;
        const existing = state.plans[planId];
        const plan: PlanState = existing ?? {
          id: planId,
          name,
          status: "running",
          progress: { total: tasksTotal, completed: 0, failed: 0 },
          tasks: [],
          costUsd: 0,
          budgetUsd: (e.budgetUsd as number) ?? null,
          startedAt: ts,
          completedAt: null,
        };
        set({
          plans: upsert(state.plans, planId, plan, {
            name,
            status: "running",
            progress: {
              total: tasksTotal,
              completed: existing?.progress.completed ?? 0,
              failed: existing?.progress.failed ?? 0,
            },
            startedAt: ts,
            completedAt: null,
          } satisfies Partial<PlanState>),
          activityFeed: pushActivity(state.activityFeed, "plan_started", `Plan started: ${name}`, ts),
        });
        return;
      }

      case "plan_completed": {
        const e = raw;
        const planId = (e.plan_id ?? e.planId ?? '') as string;
        if (!planId) return;
        const existing = state.plans[planId];
        if (!existing) return;
        const costUsd = (e.cost_usd ?? e.costUsd ?? existing.costUsd ?? 0) as number;
        set({
          plans: upsert(state.plans, planId, existing, {
            status: "completed",
            costUsd,
            completedAt: ts,
          } satisfies Partial<PlanState>),
          activityFeed: pushActivity(state.activityFeed, "plan_completed", `Plan completed: ${existing.name}`, ts),
        });
        return;
      }

      case "plan_failed": {
        const e = raw;
        const planId = (e.plan_id ?? e.planId ?? '') as string;
        if (!planId) return;
        const existing = state.plans[planId];
        if (!existing) return;
        const reason = (e.reason ?? 'unknown') as string;
        set({
          plans: upsert(state.plans, planId, existing, {
            status: "failed",
            completedAt: ts,
          } satisfies Partial<PlanState>),
          activityFeed: pushActivity(state.activityFeed, "plan_failed", `Plan failed: ${existing.name} — ${reason}`, ts),
        });
        return;
      }

      case "plan_paused": {
        const e = raw;
        const planId = (e.plan_id ?? e.planId ?? '') as string;
        const existing = state.plans[planId];
        if (!existing) return;
        set({ plans: upsert(state.plans, planId, existing, { status: "paused" }) });
        return;
      }

      case "plan_resumed": {
        const e = raw;
        const planId = (e.plan_id ?? e.planId ?? '') as string;
        const existing = state.plans[planId];
        if (!existing) return;
        set({ plans: upsert(state.plans, planId, existing, { status: "running" }) });
        return;
      }

      // -----------------------------------------------------------------------
      // Task lifecycle
      // -----------------------------------------------------------------------

      case "task_started": {
        const e = raw;
        const planId = (e.plan_id ?? e.planId ?? '') as string;
        const taskId = (e.task_id ?? e.taskId ?? '') as string;
        if (!taskId) return;
        const taskKey = `${planId}/${taskId}`;
        const name = (e.title ?? e.name ?? taskId) as string;
        const existing = state.tasks[taskKey];
        const task: TaskState = existing ?? {
          id: taskId, planId, name, status: "running",
          wave: (e.wave as number) ?? 0,
          agentName: (e.agent_name ?? e.agentName ?? null) as string | null,
          model: (e.model ?? null) as string | null,
          costUsd: 0, startedAt: ts, completedAt: null,
          dependsOn: [], gateResults: [],
        };
        const updatedTasks = upsert(state.tasks, taskKey, task, {
          status: "running", name, startedAt: ts, completedAt: null,
        } satisfies Partial<TaskState>);

        const plan = state.plans[planId];
        const updatedPlans = plan
          ? upsert(state.plans, planId, plan, {
              tasks: plan.tasks.includes(taskId) ? plan.tasks : [...plan.tasks, taskId],
              progress: { ...plan.progress, total: Math.max(plan.progress.total, plan.tasks.length + 1) },
            } satisfies Partial<PlanState>)
          : state.plans;

        set({ tasks: updatedTasks, plans: updatedPlans });
        return;
      }

      case "task_completed": {
        const e = raw;
        const planId = (e.plan_id ?? e.planId ?? '') as string;
        const taskId = (e.task_id ?? e.taskId ?? '') as string;
        const taskKey = `${planId}/${taskId}`;
        const existing = state.tasks[taskKey];
        if (!existing) return;
        const updatedTasks = upsert(state.tasks, taskKey, existing, {
          status: "completed", completedAt: ts,
        } satisfies Partial<TaskState>);

        const plan = state.plans[planId];
        const updatedPlans = plan
          ? upsert(state.plans, planId, plan, {
              progress: { ...plan.progress, completed: plan.progress.completed + 1 },
            } satisfies Partial<PlanState>)
          : state.plans;

        set({
          tasks: updatedTasks, plans: updatedPlans,
          activityFeed: pushActivity(state.activityFeed, "task_completed", `Task completed: ${existing.name}`, ts),
        });
        return;
      }

      case "task_failed": {
        const e = raw;
        const planId = (e.plan_id ?? e.planId ?? '') as string;
        const taskId = (e.task_id ?? e.taskId ?? '') as string;
        const taskKey = `${planId}/${taskId}`;
        const existing = state.tasks[taskKey];
        if (!existing) return;
        const reason = (e.reason ?? 'unknown') as string;
        const updatedTasks = upsert(state.tasks, taskKey, existing, {
          status: "failed", completedAt: ts,
        } satisfies Partial<TaskState>);

        const plan = state.plans[planId];
        const updatedPlans = plan
          ? upsert(state.plans, planId, plan, {
              progress: { ...plan.progress, failed: plan.progress.failed + 1 },
            } satisfies Partial<PlanState>)
          : state.plans;

        set({
          tasks: updatedTasks, plans: updatedPlans,
          activityFeed: pushActivity(state.activityFeed, "task_failed", `Task failed: ${existing.name} — ${reason}`, ts),
        });
        return;
      }

      case "task_restarted": {
        const e = raw;
        const planId = (e.plan_id ?? e.planId ?? '') as string;
        const taskId = (e.task_id ?? e.taskId ?? '') as string;
        const taskKey = `${planId}/${taskId}`;
        const existing = state.tasks[taskKey];
        if (!existing) return;
        set({ tasks: upsert(state.tasks, taskKey, existing, { status: "running", completedAt: null }) });
        return;
      }

      // -----------------------------------------------------------------------
      // Gate pipeline
      // -----------------------------------------------------------------------

      case "gate_started": {
        const e = raw;
        const planId = (e.plan_id ?? e.planId ?? '') as string;
        const taskId = (e.task_id ?? e.taskId ?? '') as string;
        const taskKey = `${planId}/${taskId}`;
        const existing = state.tasks[taskKey];
        if (!existing) return;
        set({ tasks: upsert(state.tasks, taskKey, existing, { status: "gating" }) });
        return;
      }

      case "gate_result": {
        const e = raw;
        const planId = (e.plan_id ?? e.planId ?? '') as string;
        const taskId = (e.task_id ?? e.taskId ?? '') as string;
        const passed = (e.passed ?? true) as boolean;
        const gateName = (e.gate ?? e.gateName ?? 'gate') as string;
        const output = (e.output_text ?? e.output ?? '') as string;
        const result: GateResult = {
          id: `gate-${planId}-${taskId}-${Date.now()}`,
          taskId, planId, gateName,
          rung: ((e.rung as number) ?? 0) as GateResult['rung'],
          passed,
          summary: output.slice(0, 200),
          output,
          durationMs: (e.duration_ms ?? e.durationMs ?? 0) as number,
          timestamp: ts,
        };
        const nextGates = prepend(state.recentGates, result, MAX_GATES);
        const taskKey = `${planId}/${taskId}`;
        const existingTask = state.tasks[taskKey];
        const updatedTasks = existingTask
          ? upsert(state.tasks, taskKey, existingTask, {
              gateResults: [...existingTask.gateResults, result],
            } satisfies Partial<TaskState>)
          : state.tasks;

        set({
          recentGates: nextGates,
          tasks: updatedTasks,
          activityFeed: !passed
            ? pushActivity(state.activityFeed, "gate_result", `Gate failed: ${gateName}`, ts)
            : state.activityFeed,
        });
        return;
      }

      // -----------------------------------------------------------------------
      // Agent lifecycle
      // -----------------------------------------------------------------------

      case "agent_spawned": {
        const e = raw;
        const agentId = (e.agent_id ?? e.agentId ?? '') as string;
        if (!agentId) return;
        const role = (e.role ?? 'agent') as string;
        const model = (e.model ?? '') as string;
        const provider = (e.provider ?? '') as string;
        const name = (e.name ?? agentId) as string;
        const taskId = (e.task_id ?? e.taskId ?? null) as string | null;
        const existing = state.agents[agentId];
        const agent: AgentState = existing ?? {
          id: agentId, name, role, model, provider,
          status: "active", currentTask: taskId,
          tokensIn: 0, tokensOut: 0, costUsd: 0, contextPct: 0, startedAt: ts,
        };
        set({
          agents: upsert(state.agents, agentId, agent, {
            name, role, model, provider, status: "active", currentTask: taskId, startedAt: ts,
          } satisfies Partial<AgentState>),
          activityFeed: pushActivity(state.activityFeed, "agent_spawned", `Agent spawned: ${name} (${role})`, ts),
        });
        return;
      }

      case "agent_completed": {
        const e = raw;
        const agentId = (e.agent_id ?? e.agentId ?? '') as string;
        if (!agentId) return;
        const existing = state.agents[agentId];
        if (!existing) return;
        const tokensIn = (e.tokens_in ?? e.tokensIn ?? existing.tokensIn) as number;
        const tokensOut = (e.tokens_out ?? e.tokensOut ?? existing.tokensOut) as number;
        const costUsd = (e.cost_usd ?? e.costUsd ?? existing.costUsd) as number;
        set({
          agents: upsert(state.agents, agentId, existing, {
            status: "completed", tokensIn, tokensOut, costUsd, currentTask: null,
          } satisfies Partial<AgentState>),
          activityFeed: pushActivity(state.activityFeed, "agent_completed", `Agent completed: ${existing.name}`, ts),
        });
        return;
      }

      case "agent_output": {
        const e = raw;
        const agentId = (e.agent_id ?? e.agentId ?? '') as string;
        // The server sends `content`, not `chunk`
        const content = (e.content ?? e.chunk ?? '') as string;
        if (!agentId || !content) return;
        const lines = content.split("\n");
        const prevLines = state.agentOutput[agentId] ?? [];
        const combined = [...prevLines, ...lines];
        const capped =
          combined.length > MAX_AGENT_OUTPUT_LINES
            ? combined.slice(combined.length - MAX_AGENT_OUTPUT_LINES)
            : combined;
        set({
          agentOutput: { ...state.agentOutput, [agentId]: capped },
        });
        return;
      }

      case "agent_route_metrics": {
        const e = raw;
        const agentId = (e.agent_id ?? e.agentId ?? '') as string;
        const existing = state.agents[agentId];
        if (!existing) return;
        set({
          agents: upsert(state.agents, agentId, existing, {
            model: (e.model as string) ?? existing.model,
            provider: (e.provider as string) ?? existing.provider,
            tokensIn: existing.tokensIn + ((e.tokens_in ?? e.tokensIn ?? 0) as number),
            tokensOut: existing.tokensOut + ((e.tokens_out ?? e.tokensOut ?? 0) as number),
            costUsd: existing.costUsd + ((e.cost_usd ?? e.costUsd ?? 0) as number),
          } satisfies Partial<AgentState>),
        });
        return;
      }

      // -----------------------------------------------------------------------
      // Cost and budget
      // -----------------------------------------------------------------------

      case "cost_updated": {
        const e = raw;
        const planId = (e.plan_id ?? e.planId ?? '') as string;
        const agentId = (e.agent_id ?? e.agentId ?? '') as string;
        const totalUsd = (e.total_usd ?? e.totalUsd ?? 0) as number;
        const delta = (e.delta ?? 0) as number;
        if (planId) {
          const plan = state.plans[planId];
          if (plan) {
            set({ plans: upsert(state.plans, planId, plan, { costUsd: totalUsd }) });
          }
        }
        if (agentId) {
          const agent = state.agents[agentId];
          if (agent) {
            set({ agents: upsert(state.agents, agentId, agent, { costUsd: agent.costUsd + delta }) });
          }
        }
        return;
      }

      case "budget_warning": {
        const e = raw;
        const planId = (e.plan_id ?? e.planId ?? '') as string;
        const fraction = (e.fraction ?? 0) as number;
        const spentUsd = (e.spent_usd ?? e.spentUsd ?? 0) as number;
        const budgetUsd = (e.budget_usd ?? e.budgetUsd ?? 0) as number;
        const item: InboxItem = {
          id: `budget-${planId}-${ts}`,
          kind: "budget_warning",
          severity: fraction >= 1 ? "error" : "warning",
          title: `Budget warning: ${planId}`,
          description: `Spent $${spentUsd.toFixed(2)} of $${budgetUsd.toFixed(2)} (${Math.round(fraction * 100)}%)`,
          dismissedAt: null,
          timestamp: ts,
        };
        set({
          inboxItems: prepend(state.inboxItems, item, MAX_EVENT_LOG),
          activityFeed: pushActivity(state.activityFeed, "budget_warning", `Budget warning: ${planId} at ${Math.round(fraction * 100)}%`, ts),
        });
        return;
      }

      // -----------------------------------------------------------------------
      // Affect
      // -----------------------------------------------------------------------

      case "affect_updated": {
        const e = raw;
        const affect = (e.affect ?? null) as AffectState | null;
        if (!affect) return;
        set({
          affect,
          vitals: { ...state.vitals, affectWord: affect.word ?? 'idle' },
          activityFeed: pushActivity(state.activityFeed, "affect_updated", `Affect: ${affect.word}`, ts),
        });
        return;
      }

      case "knowledge_entry_created": {
        const e = raw;
        const entry = e.entry as Record<string, unknown> | undefined;
        const tier = (entry?.tier ?? 'transient') as string;
        const prev = state.learning.knowledgeTierCounts[tier] ?? 0;
        set({ learning: { ...state.learning, knowledgeTierCounts: { ...state.learning.knowledgeTierCounts, [tier]: prev + 1 } } });
        return;
      }

      case "knowledge_tier_changed": {
        const e = raw;
        const previousTier = (e.previous_tier ?? e.previousTier ?? 'transient') as string;
        const newTier = (e.new_tier ?? e.newTier ?? 'working') as string;
        const counts = state.learning.knowledgeTierCounts;
        set({
          learning: { ...state.learning, knowledgeTierCounts: { ...counts, [previousTier]: Math.max(0, (counts[previousTier] ?? 0) - 1), [newTier]: (counts[newTier] ?? 0) + 1 } },
          activityFeed: pushActivity(state.activityFeed, "knowledge_tier_changed", `Knowledge promoted: ${previousTier} → ${newTier}`, ts),
        });
        return;
      }

      case "cfactor_updated": {
        const e = raw;
        const cfactor = (e.cfactor ?? e.value ?? 0) as number;
        const delta = (e.delta ?? 0) as number;
        const trend = (e.trend ?? 'flat') as 'up' | 'down' | 'flat';
        set({
          learning: { ...state.learning, cfactor, cfactorDelta: delta, cfactorTrend: trend },
          vitals: { ...state.vitals, cfactor },
          activityFeed: pushActivity(state.activityFeed, "cfactor_updated", `C-factor: ${cfactor.toFixed(3)}`, ts),
        });
        return;
      }

      case "learning_stage_changed": {
        const e = raw;
        set({
          learning: { ...state.learning, learningStage: (e.new_stage ?? e.newStage ?? 'static') as LearningState['learningStage'], observationCount: (e.observation_count ?? e.observationCount ?? 0) as number },
        });
        return;
      }

      case "provider_health_updated": {
        const e = raw;
        const provider = (e.provider ?? e) as ProviderState;
        if (!provider?.id) return;
        const prevProvider = state.providers[provider.id];
        const updatedProviders = upsert(state.providers, provider.id, provider, provider);
        const providerList = Object.values(updatedProviders);
        const healthyProviders = providerList.filter((p) => p.status === "healthy").length;
        const statusChanged = prevProvider && prevProvider.status !== provider.status;
        set({
          providers: updatedProviders,
          vitals: { ...state.vitals, healthyProviders, totalProviders: providerList.length },
          activityFeed: statusChanged ? pushActivity(state.activityFeed, "provider_health_updated", `Provider ${provider.name}: ${provider.status}`, ts) : state.activityFeed,
        });
        return;
      }

      case "experiment_updated": {
        const e = raw;
        const experiment = (e.experiment ?? e) as Record<string, unknown>;
        const expStatus = (experiment.status ?? '') as string;
        const isActive = expStatus === "collecting" || expStatus === "trending";
        const wasActive = state.learning.activeExperiments > 0;
        const d = isActive && !wasActive ? 1 : !isActive && wasActive ? -1 : 0;
        if (d !== 0) {
          set({ learning: { ...state.learning, activeExperiments: Math.max(0, state.learning.activeExperiments + d) } });
        }
        return;
      }

      case "experiment_concluded": {
        set({ learning: { ...state.learning, activeExperiments: Math.max(0, state.learning.activeExperiments - 1) } });
        return;
      }

      // Handle episode_recorded from roko-serve (not in the original union)
      case "episode_recorded" as string:
        return;

      case "error_occurred": {
        const e = raw;
        const error = (e.error ?? e) as ErrorEntry;
        if (!error?.id && !error?.message) return;
        const errEntry: ErrorEntry = {
          id: error.id ?? `err-${Date.now()}`,
          source: error.source ?? 'unknown',
          message: error.message ?? 'Unknown error',
          severity: error.severity ?? 'medium',
          timestamp: error.timestamp ?? ts,
        };
        set({
          recentErrors: prepend(state.recentErrors, errEntry, MAX_ERRORS),
          activityFeed: pushActivity(state.activityFeed, "error_occurred", `Error: ${errEntry.message}`, ts),
        });
        return;
      }

      case "inbox_item_added": {
        const e = raw;
        const item = (e.item ?? e) as InboxItem;
        if (!item?.id) return;
        if (state.inboxItems.some((i) => i.id === item.id)) return;
        set({
          inboxItems: prepend(state.inboxItems, item, MAX_EVENT_LOG),
          activityFeed: pushActivity(state.activityFeed, "inbox_item_added", `Inbox: ${item.title}`, ts),
        });
        return;
      }

      case "snapshot": {
        const e = raw;
        if (e.snapshot) get().replaceSnapshot(e.snapshot as DashboardSnapshot);
        return;
      }

      // Events the server sends but the portal doesn't need to handle yet
      case "dream_phase_changed":
      case "dream_cycle_completed":
      case "snapshot_rebased" as string:
      case "event_log_entry" as string:
      case "task_output_appended" as string:
      case "efficiency_event" as string:
        return;

      default: {
        // Silently ignore unknown event types from roko-serve.
        // The server may send types like snapshot_rebased, event_log_entry,
        // task_output_appended, episode_recorded, efficiency_event, etc.
        // that the portal doesn't handle yet.
        if (process.env.NODE_ENV !== 'production') {
          console.debug('[DashboardStore] Unhandled event type:', (event as Record<string, unknown>).type);
        }
        return;
      }
    }
    } catch (err) {
      // Never let a malformed SSE event crash the store or the UI.
      if (process.env.NODE_ENV !== 'production') {
        console.error('[DashboardStore] Event handling error:', err, event);
      }
    }
  },
}));
