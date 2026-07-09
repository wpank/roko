# A4: Observatory pages -- live agents, plans, learning, conductor, costs

## Context

**Repo:** `/Users/will/dev/nunchi/nunchi-dashboard`
**Branch:** `demo-rewrite`
**Tech stack:** React 19 + Vite 8 + TypeScript + Tailwind CSS v4
**Backend:** `roko-serve` runs at `http://localhost:6677` with ~85 REST routes + WebSocket at `ws://localhost:6677/ws`
**Auth:** Privy (env var `VITE_PRIVY_APP_ID`) with password fallback
**Design:** ROSEDUST dark palette -- bg_void `#060608`, rose `#AA7088`, bone `#C8B890`, rose_bright `#CC90A8`

### Before starting
1. `cd /Users/will/dev/nunchi/nunchi-dashboard`
2. `git checkout -b demo-rewrite 2>/dev/null || git checkout demo-rewrite`
3. `npm install`
4. Verify: `npm run dev` starts without errors

### After every task
1. `npm run typecheck` passes
2. `npm run dev` -- page renders without console errors
3. All existing tests pass: `npm test` (if test runner is configured)

---

## What this task produces

Five pages under `/app/observatory/*`. Each page fetches real data from `roko-serve` via the TanStack Query hooks created in Task A2, handles loading/error/empty states using design system components from Task A1, and updates the router placeholders.

**Depends on:** Task A1 (design system, router), Task A2 (API hooks).

---

## Checklist

### 1. Create page directory

```bash
mkdir -p /Users/will/dev/nunchi/nunchi-dashboard/src/pages/observatory
```

### 2. LiveAgents page

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/observatory/LiveAgents.tsx`:

```tsx
import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useAgents } from "../../services/api";
import { Card, Badge, StatusDot, Skeleton, EmptyState, ErrorState } from "../../design-system/components";
import { useWsStore } from "../../stores/wsStore";

// Cognitive tier color scheme: T0 = hot path (rose), T1 = warm (gold), T2 = cold (muted)
const TIER_COLOR: Record<string, string> = {
  T0: "var(--rd-rose-bright)",
  T1: "var(--rd-accent-gold)",
  T2: "var(--rd-fg-muted)",
};

const TIER_LABEL: Record<string, string> = {
  T0: "T0",
  T1: "T1",
  T2: "T2",
};

function TierDot({ tier }: { tier: string }) {
  return (
    <span
      title={`Cognitive tier ${tier}`}
      style={{ backgroundColor: TIER_COLOR[tier] ?? "var(--rd-fg-muted)" }}
      className="inline-block w-2 h-2 rounded-full shrink-0"
    />
  );
}

export default function LiveAgents() {
  const { data: agents, isLoading, error, refetch } = useAgents();
  const queryClient = useQueryClient();
  const { lastEvent } = useWsStore();

  // Invalidate agents query on any WS event that touches agent state
  useEffect(() => {
    if (!lastEvent) return;
    const { type } = lastEvent;
    if (
      type === "agent_started" ||
      type === "agent_stopped" ||
      type === "agent_status_changed"
    ) {
      queryClient.invalidateQueries({ queryKey: ["agents"] });
    }
  }, [lastEvent, queryClient]);

  if (isLoading) {
    return (
      <section className="p-6 space-y-3">
        {Array.from({ length: 5 }).map((_, i) => (
          <Skeleton key={i} height="72px" />
        ))}
      </section>
    );
  }

  if (error) {
    return (
      <section className="p-6">
        <ErrorState error={String(error)} onRetry={() => refetch()} />
      </section>
    );
  }

  if (!agents || agents.length === 0) {
    return (
      <section className="p-6">
        <EmptyState
          title="No agents running"
          description="Start a plan execution or launch an agent via CLI to see them here."
        />
      </section>
    );
  }

  return (
    <section className="p-6">
      <header className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)]">
            Live agents
          </h1>
          <p className="text-xs text-[var(--rd-fg-muted)] mt-0.5">
            {agents.length} managed process{agents.length !== 1 ? "es" : ""}
          </p>
        </div>
        <Badge label={`${agents.length} active`} variant="success" />
      </header>

      <div className="space-y-2">
        {agents.map((agent) => {
          const tier = agent.tier ?? "T1";
          const statusVariant =
            agent.status === "running"
              ? "online"
              : agent.status === "idle"
                ? "idle"
                : "offline";

          return (
            <Card key={agent.id} padding="sm" className="flex items-center gap-3">
              <StatusDot status={statusVariant} />
              <TierDot tier={tier} />
              <div className="flex-1 min-w-0">
                <div className="text-sm font-medium text-[var(--rd-fg-primary)] truncate">
                  {agent.label || `Agent ${agent.id}`}
                </div>
                <div className="text-[10px] font-mono text-[var(--rd-fg-muted)]">
                  PID {agent.id} &middot; {tier}
                </div>
              </div>
              <div className="flex items-center gap-2 shrink-0">
                {agent.current_task && (
                  <span className="text-[10px] text-[var(--rd-fg-muted)] truncate max-w-[140px]">
                    {agent.current_task}
                  </span>
                )}
                <Badge
                  label={agent.status ?? "unknown"}
                  variant={
                    agent.status === "running"
                      ? "success"
                      : agent.status === "idle"
                        ? "default"
                        : "warning"
                  }
                />
              </div>
            </Card>
          );
        })}
      </div>
    </section>
  );
}
```

### 3. Plans page

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/observatory/Plans.tsx`:

```tsx
import { useState, useCallback, useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { usePlans, usePlan, useExecutePlan } from "../../services/api";
import { Card, Badge, Button, Gauge, Skeleton, EmptyState, ErrorState, Modal } from "../../design-system/components";
import { useToast } from "../../design-system/components";
import { useWsStore } from "../../stores/wsStore";

const TASK_STATUS_COLOR: Record<string, string> = {
  completed: "var(--rd-success)",
  running: "var(--rd-rose-bright)",
  failed: "var(--rd-error)",
  pending: "var(--rd-bg-surface-3)",
  skipped: "var(--rd-fg-muted)",
};

export default function Plans() {
  const { data: plans, isLoading, error, refetch } = usePlans();
  const [selectedPlanId, setSelectedPlanId] = useState<string | null>(null);
  const { data: planDetail, isLoading: detailLoading } = usePlan(selectedPlanId ?? "");
  const executePlan = useExecutePlan();
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const { lastEvent } = useWsStore();

  // Invalidate plans on task or plan state changes
  useEffect(() => {
    if (!lastEvent) return;
    const { type } = lastEvent;
    if (
      type === "task_started" ||
      type === "task_completed" ||
      type === "task_failed" ||
      type === "plan_started" ||
      type === "plan_completed"
    ) {
      queryClient.invalidateQueries({ queryKey: ["plans"] });
      if (selectedPlanId) {
        queryClient.invalidateQueries({ queryKey: ["plan", selectedPlanId] });
      }
    }
  }, [lastEvent, queryClient, selectedPlanId]);

  const handleExecute = useCallback(
    (planId: string) => {
      executePlan.mutate(planId, {
        onSuccess: () => toast("Plan execution started", "success"),
        onError: (err) => toast(`Failed: ${err}`, "error"),
      });
    },
    [executePlan, toast],
  );

  if (isLoading) {
    return (
      <section className="p-6 space-y-3">
        {Array.from({ length: 4 }).map((_, i) => (
          <Skeleton key={i} height="88px" />
        ))}
      </section>
    );
  }

  if (error) {
    return (
      <section className="p-6">
        <ErrorState error={String(error)} onRetry={() => refetch()} />
      </section>
    );
  }

  if (!plans || plans.length === 0) {
    return (
      <section className="p-6">
        <EmptyState
          title="No plans found"
          description="Create a plan via the CLI or the Atelier page."
        />
      </section>
    );
  }

  return (
    <section className="p-6">
      <header className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)]">
            Plans
          </h1>
          <p className="text-xs text-[var(--rd-fg-muted)] mt-0.5">
            {plans.length} plan{plans.length !== 1 ? "s" : ""} discovered
          </p>
        </div>
        <Badge
          label={`${plans.filter((p) => !p.completed).length} pending`}
          variant="warning"
        />
      </header>

      <div className="space-y-2">
        {plans.map((plan) => {
          const completedCount = plan.completed_task_count ?? (plan.completed ? plan.task_count : 0);
          const progress = plan.task_count > 0 ? completedCount / plan.task_count : 0;

          return (
            <Card key={plan.id} padding="md">
              <div className="flex items-start justify-between mb-3">
                <button
                  type="button"
                  className="text-left flex-1 min-w-0 hover:opacity-80 transition-opacity"
                  onClick={() => setSelectedPlanId(plan.id)}
                >
                  <div className="text-sm font-medium text-[var(--rd-fg-primary)]">
                    {plan.title || plan.id}
                  </div>
                  <div className="text-[10px] font-mono text-[var(--rd-fg-muted)] mt-0.5">
                    {plan.id}
                  </div>
                </button>
                <div className="flex items-center gap-2 ml-3 shrink-0">
                  <Badge
                    label={plan.completed ? "completed" : "pending"}
                    variant={plan.completed ? "success" : "warning"}
                  />
                  {!plan.completed && (
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => handleExecute(plan.id)}
                      loading={executePlan.isPending}
                    >
                      Execute
                    </Button>
                  )}
                </div>
              </div>
              <Gauge
                value={progress}
                label={`${completedCount}/${plan.task_count} tasks`}
                size="sm"
              />
            </Card>
          );
        })}
      </div>

      {/* Plan detail modal */}
      <Modal
        isOpen={!!selectedPlanId}
        onClose={() => setSelectedPlanId(null)}
        title={planDetail?.title ?? "Plan detail"}
        size="lg"
      >
        {detailLoading ? (
          <div className="space-y-2">
            {Array.from({ length: 4 }).map((_, i) => (
              <Skeleton key={i} height="52px" />
            ))}
          </div>
        ) : planDetail ? (
          <div>
            {planDetail.description && (
              <p className="text-sm text-[var(--rd-fg-secondary)] mb-4">
                {planDetail.description}
              </p>
            )}
            <div className="space-y-1.5">
              {planDetail.tasks.map((task) => {
                const isCompleted = task.completed || task.status === "completed";
                const dotColor = TASK_STATUS_COLOR[task.status ?? (isCompleted ? "completed" : "pending")] ?? TASK_STATUS_COLOR.pending;

                return (
                  <article
                    key={task.id}
                    className="flex items-start gap-3 px-3 py-2.5 rounded-md bg-[var(--rd-bg-surface-0)]"
                  >
                    {/* Status indicator */}
                    <span
                      className="mt-0.5 w-2.5 h-2.5 rounded-sm border shrink-0 flex items-center justify-center"
                      style={{
                        backgroundColor: isCompleted ? dotColor : "transparent",
                        borderColor: dotColor,
                      }}
                    >
                      {isCompleted && (
                        <svg viewBox="0 0 8 8" className="w-1.5 h-1.5 text-white fill-current">
                          <path d="M1.5 4L3 5.5L6.5 2" stroke="white" strokeWidth="1.2" fill="none" strokeLinecap="round" />
                        </svg>
                      )}
                    </span>
                    <div className="flex-1 min-w-0">
                      <div className="text-xs font-medium text-[var(--rd-fg-primary)]">
                        {task.id}
                      </div>
                      {task.description && (
                        <div className="text-[10px] text-[var(--rd-fg-muted)] mt-0.5">
                          {task.description}
                        </div>
                      )}
                      {task.depends_on.length > 0 && (
                        <div className="text-[10px] text-[var(--rd-fg-muted)] mt-0.5 font-mono">
                          deps: {task.depends_on.join(", ")}
                        </div>
                      )}
                    </div>
                    {task.status && task.status !== "pending" && (
                      <Badge
                        label={task.status}
                        variant={
                          task.status === "completed"
                            ? "success"
                            : task.status === "failed"
                              ? "error"
                              : task.status === "running"
                                ? "info"
                                : "default"
                        }
                      />
                    )}
                  </article>
                );
              })}
            </div>
          </div>
        ) : null}
      </Modal>
    </section>
  );
}
```

### 4. Learning page

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/observatory/Learning.tsx`:

```tsx
import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useCFactor, useExperiments, useEfficiency, useCascade, useAdaptiveThresholds } from "../../services/api";
import { Card, Badge, Gauge, Sparkline, Skeleton, ErrorState } from "../../design-system/components";
import { useWsStore } from "../../stores/wsStore";

export default function Learning() {
  const { data: cfactor, isLoading: cfLoading, error: cfError, refetch: refetchCf } = useCFactor();
  const { data: experiments, isLoading: expLoading } = useExperiments();
  const { data: efficiency } = useEfficiency();
  const { data: cascade } = useCascade();
  const { data: thresholds } = useAdaptiveThresholds();
  const queryClient = useQueryClient();
  const { lastEvent } = useWsStore();

  // Invalidate learning data on efficiency/episode events
  useEffect(() => {
    if (!lastEvent) return;
    const { type } = lastEvent;
    if (
      type === "efficiency_event" ||
      type === "episode_recorded" ||
      type === "experiment_updated" ||
      type === "cascade_updated"
    ) {
      queryClient.invalidateQueries({ queryKey: ["cfactor"] });
      queryClient.invalidateQueries({ queryKey: ["experiments"] });
      queryClient.invalidateQueries({ queryKey: ["efficiency"] });
      queryClient.invalidateQueries({ queryKey: ["cascade"] });
    }
  }, [lastEvent, queryClient]);

  const isLoading = cfLoading || expLoading;

  if (isLoading) {
    return (
      <section className="p-6 grid grid-cols-2 gap-4">
        {Array.from({ length: 4 }).map((_, i) => (
          <Skeleton key={i} height="160px" />
        ))}
      </section>
    );
  }

  if (cfError) {
    return (
      <section className="p-6">
        <ErrorState error={String(cfError)} onRetry={() => refetchCf()} />
      </section>
    );
  }

  // C-Factor as a 0–1 gauge value: map percent to fraction (cap at 100%)
  const cfactorGaugeValue =
    typeof cfactor?.cfactor_pct === "number"
      ? Math.min(1, Math.max(0, cfactor.cfactor_pct / 100))
      : 0;

  return (
    <section className="p-6">
      <header className="mb-6">
        <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)]">
          Learning
        </h1>
        <p className="text-xs text-[var(--rd-fg-muted)] mt-0.5">
          Feedback loops, prompt experiments, model routing, and gate thresholds.
        </p>
      </header>

      {/* Top row: C-Factor + Cascade summary */}
      <div className="grid grid-cols-2 gap-4 mb-4">
        {/* C-Factor card */}
        <Card>
          <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-3">
            C-Factor (collective intelligence)
          </div>
          {cfactor ? (
            <div className="flex items-start gap-4">
              <div className="shrink-0">
                <Gauge
                  value={cfactorGaugeValue}
                  size="md"
                  color="var(--rd-rose-bright)"
                  label={`${typeof cfactor.cfactor_pct === "number" ? cfactor.cfactor_pct.toFixed(1) : "0"}%`}
                />
              </div>
              <div className="grid grid-cols-1 gap-1.5 mt-1 text-xs text-[var(--rd-fg-muted)]">
                <div>
                  Fleet:{" "}
                  <span className="text-[var(--rd-fg-secondary)] font-mono">
                    {cfactor.fleet_cfactor?.toFixed(3) ?? "--"}
                  </span>
                </div>
                <div>
                  Solo avg:{" "}
                  <span className="text-[var(--rd-fg-secondary)] font-mono">
                    {cfactor.solo_avg?.toFixed(3) ?? "--"}
                  </span>
                </div>
                <div>
                  Agents:{" "}
                  <span className="text-[var(--rd-fg-secondary)] font-mono">
                    {cfactor.agent_count ?? 0}
                  </span>
                </div>
              </div>
            </div>
          ) : (
            <div className="text-sm text-[var(--rd-fg-muted)]">No data yet</div>
          )}
        </Card>

        {/* Cascade router card */}
        <Card>
          <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-3">
            Cascade router
          </div>
          {cascade ? (
            <div className="space-y-2 text-xs text-[var(--rd-fg-secondary)]">
              <div>
                Model routing decisions persist to{" "}
                <code className="text-[var(--rd-rose)] bg-[var(--rd-bg-surface-2)] px-1 rounded text-[10px]">
                  .roko/learn/cascade-router.json
                </code>
              </div>
              <div className="text-[10px] text-[var(--rd-fg-muted)]">
                {Object.keys(cascade).length > 0
                  ? `${Object.keys(cascade).length} routing entries`
                  : "Waiting for routing data"}
              </div>
            </div>
          ) : (
            <div className="text-sm text-[var(--rd-fg-muted)]">No routing data</div>
          )}
        </Card>
      </div>

      {/* Experiments */}
      <Card className="mb-4">
        <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-3">
          Prompt experiments
        </div>
        {experiments && experiments.length > 0 ? (
          <div className="space-y-2">
            {experiments.map((exp) => {
              const isConcluded = exp.status === "concluded";
              const winner = isConcluded ? exp.winner_variant : null;

              return (
                <article
                  key={exp.id}
                  className="flex items-center justify-between px-3 py-2 rounded-md bg-[var(--rd-bg-surface-0)]"
                >
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-medium text-[var(--rd-fg-primary)] truncate">
                        {exp.name}
                      </span>
                      {winner && (
                        <Badge label={`winner: ${winner}`} variant="success" />
                      )}
                    </div>
                    <div className="text-[10px] text-[var(--rd-fg-muted)] mt-0.5">
                      {exp.variants?.length ?? 0} variant{(exp.variants?.length ?? 0) !== 1 ? "s" : ""}
                    </div>
                  </div>
                  <Badge
                    label={exp.status}
                    variant={
                      exp.status === "concluded"
                        ? "success"
                        : exp.status === "running"
                          ? "info"
                          : "default"
                    }
                  />
                </article>
              );
            })}
          </div>
        ) : (
          <div className="text-xs text-[var(--rd-fg-muted)]">
            No experiments running. Start one via CLI:{" "}
            <code className="text-[var(--rd-rose)]">roko prd plan --experiment</code>
          </div>
        )}
      </Card>

      {/* Bottom row: Efficiency + Adaptive thresholds */}
      <div className="grid grid-cols-2 gap-4">
        <Card>
          <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-2">
            Efficiency events
          </div>
          {efficiency && Array.isArray(efficiency) && efficiency.length > 0 ? (
            <>
              <div className="text-xs text-[var(--rd-fg-secondary)] mb-2">
                {efficiency.length} event{efficiency.length !== 1 ? "s" : ""} recorded
              </div>
              <Sparkline
                data={efficiency.slice(-20).map((e) => e.cost_usd ?? 0)}
                height={32}
                color="var(--rd-accent-gold)"
              />
            </>
          ) : (
            <div className="text-xs text-[var(--rd-fg-muted)]">No efficiency data</div>
          )}
        </Card>

        <Card>
          <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-2">
            Adaptive gate thresholds
          </div>
          {thresholds && Object.keys(thresholds).length > 0 ? (
            <>
              <div className="text-xs text-[var(--rd-fg-secondary)] mb-2">
                {Object.keys(thresholds).length} threshold{Object.keys(thresholds).length !== 1 ? "s" : ""} configured
              </div>
              <div className="text-[10px] text-[var(--rd-fg-muted)]">
                EMA-adjusted per rung, persisted to{" "}
                <code className="text-[var(--rd-rose)] bg-[var(--rd-bg-surface-2)] px-1 rounded">
                  .roko/learn/gate-thresholds.json
                </code>
              </div>
            </>
          ) : (
            <div className="text-xs text-[var(--rd-fg-muted)]">No threshold data</div>
          )}
        </Card>
      </div>
    </section>
  );
}
```

### 5. Conductor page

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/observatory/Conductor.tsx`:

```tsx
import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useDiagnosis } from "../../services/api";
import { Card, Badge, Skeleton, EmptyState, ErrorState } from "../../design-system/components";
import { useWsStore } from "../../stores/wsStore";

const SEVERITY_VARIANT: Record<string, "success" | "warning" | "error" | "default"> = {
  low: "default",
  medium: "warning",
  high: "error",
  critical: "error",
};

function formatTimestamp(ts: string): string {
  const d = new Date(ts);
  if (isNaN(d.getTime())) return ts;
  return d.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export default function Conductor() {
  const { data: diagnoses, isLoading, error, refetch } = useDiagnosis();
  const queryClient = useQueryClient();
  const { lastEvent } = useWsStore();

  // Invalidate diagnoses on conductor events
  useEffect(() => {
    if (!lastEvent) return;
    const { type } = lastEvent;
    if (
      type === "diagnosis_added" ||
      type === "circuit_breaker_tripped" ||
      type === "watcher_alert"
    ) {
      queryClient.invalidateQueries({ queryKey: ["diagnoses"] });
    }
  }, [lastEvent, queryClient]);

  if (isLoading) {
    return (
      <section className="p-6 space-y-3">
        {Array.from({ length: 3 }).map((_, i) => (
          <Skeleton key={i} height="100px" />
        ))}
      </section>
    );
  }

  if (error) {
    return (
      <section className="p-6">
        <ErrorState error={String(error)} onRetry={() => refetch()} />
      </section>
    );
  }

  if (!diagnoses || diagnoses.length === 0) {
    return (
      <section className="p-6">
        <EmptyState
          title="No diagnoses"
          description="The conductor runs 10 watchers that detect anomalies. When something trips, diagnoses appear here."
        />
      </section>
    );
  }

  const hasCritical = diagnoses.some((d) => d.severity === "critical");

  return (
    <section className="p-6">
      <header className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)]">
            Conductor
          </h1>
          <p className="text-xs text-[var(--rd-fg-muted)] mt-0.5">
            Circuit breaker diagnoses and watcher alerts
          </p>
        </div>
        <Badge
          label={`${diagnoses.length} diagnosis${diagnoses.length !== 1 ? "es" : ""}`}
          variant={hasCritical ? "error" : "warning"}
        />
      </header>

      <div className="space-y-2">
        {diagnoses.map((diag, i) => (
          <Card key={`${diag.agent_id}-${diag.timestamp}-${i}`} padding="md">
            <div className="flex items-start justify-between mb-2">
              <div className="flex items-center gap-2">
                <code className="text-xs font-mono text-[var(--rd-fg-muted)]">
                  {diag.agent_id}
                </code>
                <Badge
                  label={diag.severity}
                  variant={SEVERITY_VARIANT[diag.severity] ?? "default"}
                />
              </div>
              <time
                dateTime={diag.timestamp}
                className="text-[10px] text-[var(--rd-fg-muted)] shrink-0"
              >
                {formatTimestamp(diag.timestamp)}
              </time>
            </div>
            <div className="text-sm font-medium text-[var(--rd-fg-primary)] mb-1">
              {diag.verdict}
            </div>
            <p className="text-xs text-[var(--rd-fg-secondary)]">
              {diag.details}
            </p>
          </Card>
        ))}
      </div>
    </section>
  );
}
```

### 6. Costs page

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/observatory/Costs.tsx`:

```tsx
import { useEfficiency, useCostTiers, useProviders } from "../../services/api";
import { Card, Sparkline, Skeleton, EmptyState, ErrorState } from "../../design-system/components";

// Format USD values with appropriate precision
function formatUsd(value: number): string {
  if (value >= 1) return `$${value.toFixed(2)}`;
  if (value >= 0.01) return `$${value.toFixed(4)}`;
  return `$${value.toFixed(6)}`;
}

function MetricCard({
  label,
  value,
  mono = true,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <Card padding="sm">
      <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-1.5">
        {label}
      </div>
      <div
        className={`text-xl text-[var(--rd-fg-primary)] ${mono ? "font-mono" : "font-semibold"}`}
      >
        {value}
      </div>
    </Card>
  );
}

export default function Costs() {
  const { data: efficiency, isLoading: effLoading, error: effError, refetch: refetchEff } = useEfficiency();
  const { data: providers } = useProviders();

  if (effLoading) {
    return (
      <section className="p-6 space-y-4">
        <div className="grid grid-cols-4 gap-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} height="80px" />
          ))}
        </div>
        <Skeleton height="120px" />
        <Skeleton height="160px" />
      </section>
    );
  }

  if (effError) {
    return (
      <section className="p-6">
        <ErrorState error={String(effError)} onRetry={() => refetchEff()} />
      </section>
    );
  }

  const hasData = (efficiency?.length ?? 0) > 0 || (providers?.length ?? 0) > 0;

  // Compute aggregates from efficiency events
  const totalCost = efficiency?.reduce((sum, e) => sum + (e.cost_usd ?? 0), 0) ?? 0;
  const totalTokensIn = efficiency?.reduce((sum, e) => sum + (e.tokens_in ?? 0), 0) ?? 0;
  const totalTokensOut = efficiency?.reduce((sum, e) => sum + (e.tokens_out ?? 0), 0) ?? 0;
  const avgLatency =
    efficiency && efficiency.length > 0
      ? efficiency.reduce((sum, e) => sum + (e.latency_ms ?? 0), 0) / efficiency.length
      : 0;

  return (
    <section className="p-6">
      <header className="mb-6">
        <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)]">
          Costs
        </h1>
        <p className="text-xs text-[var(--rd-fg-muted)] mt-0.5">
          Token usage, cost breakdown, and provider health.
        </p>
      </header>

      {/* Summary stat cards */}
      <div className="grid grid-cols-4 gap-4 mb-6">
        <MetricCard label="Total cost" value={formatUsd(totalCost)} />
        <MetricCard label="Tokens in" value={totalTokensIn.toLocaleString()} />
        <MetricCard label="Tokens out" value={totalTokensOut.toLocaleString()} />
        <MetricCard label="Avg latency" value={`${avgLatency.toFixed(0)}ms`} />
      </div>

      {/* Cost-per-event sparkline */}
      {efficiency && efficiency.length > 0 && (
        <Card className="mb-6">
          <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-3">
            Cost per event (last 50)
          </div>
          <Sparkline
            data={efficiency.slice(-50).map((e) => e.cost_usd ?? 0)}
            height={80}
            color="var(--rd-accent-gold)"
          />
        </Card>
      )}

      {/* Per-provider breakdown */}
      {providers && providers.length > 0 && (
        <Card>
          <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-3">
            Providers
          </div>
          <div className="space-y-2">
            {providers.map((p) => {
              const healthColor =
                p.health === "healthy"
                  ? "var(--rd-success)"
                  : p.health === "degraded"
                    ? "var(--rd-warning)"
                    : "var(--rd-error)";

              return (
                <div
                  key={p.id}
                  className="flex items-center justify-between px-3 py-2 rounded-md bg-[var(--rd-bg-surface-0)]"
                >
                  <div>
                    <div className="text-xs font-medium text-[var(--rd-fg-primary)]">
                      {p.name}
                    </div>
                    <div className="text-[10px] text-[var(--rd-fg-muted)]">
                      {p.model_count} model{p.model_count !== 1 ? "s" : ""}
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <span
                      className="w-1.5 h-1.5 rounded-full"
                      style={{ backgroundColor: healthColor }}
                    />
                    <span className="text-[10px] text-[var(--rd-fg-muted)]">
                      {p.health}
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        </Card>
      )}

      {!hasData && (
        <EmptyState
          title="No cost data"
          description="Run an agent or plan execution to generate efficiency events."
        />
      )}
    </section>
  );
}
```

### 7. Wire pages into the router

- [ ] Update `/Users/will/dev/nunchi/nunchi-dashboard/src/router.tsx` -- replace the five observatory placeholders with lazy imports:

Find these lines in the router children array:
```
{ path: "observatory/agents", element: <Placeholder name="Live agents" /> },
{ path: "observatory/plans", element: <Placeholder name="Plans" /> },
{ path: "observatory/learning", element: <Placeholder name="Learning" /> },
{ path: "observatory/conductor", element: <Placeholder name="Conductor" /> },
{ path: "observatory/costs", element: <Placeholder name="Costs" /> },
```

Replace them with:
```tsx
{ path: "observatory/agents", element: lazyPage(() => import("./pages/observatory/LiveAgents")) },
{ path: "observatory/plans", element: lazyPage(() => import("./pages/observatory/Plans")) },
{ path: "observatory/learning", element: lazyPage(() => import("./pages/observatory/Learning")) },
{ path: "observatory/conductor", element: lazyPage(() => import("./pages/observatory/Conductor")) },
{ path: "observatory/costs", element: lazyPage(() => import("./pages/observatory/Costs")) },
```

---

## Verification

Run from `/Users/will/dev/nunchi/nunchi-dashboard`:

- [ ] `npm run typecheck` -- exits 0
- [ ] `npm run dev` -- navigate to each route:
  - `/app/observatory/agents` -- shows empty state (or agent list with tier dots if roko-serve is running)
  - `/app/observatory/plans` -- shows empty state (or plans with Gauge progress bars)
  - `/app/observatory/learning` -- shows C-Factor Gauge card, experiments list with winner badge, efficiency sparkline
  - `/app/observatory/conductor` -- shows empty state (or diagnoses with formatted timestamps)
  - `/app/observatory/costs` -- shows summary cards with zeros (or real data), sparkline, providers
- [ ] Each page handles loading (Skeleton grid), error (ErrorState retry button), and empty states
- [ ] Plans page: clicking a plan title opens the detail modal with task list and status indicators
- [ ] Plans page: "Execute" button triggers the mutation
- [ ] LiveAgents page: each agent card shows a colored tier dot (T0/T1/T2) and correct status badge
- [ ] Learning page: C-Factor uses the Gauge component, concluded experiments show a winner badge
- [ ] WS events (agent_started, task_completed, etc.) trigger query invalidation on all pages
- [ ] No console errors
