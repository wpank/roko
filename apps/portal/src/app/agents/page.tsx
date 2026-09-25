'use client';

import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import Link from 'next/link';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { clsx } from 'clsx';
import { Plus, ArrowRight } from 'lucide-react';

import type {
  AgentState,
  AgentStatus,
  GateResult,
  EpisodeEntry,
} from '@/api/types';
import { useAgents as useAgentsQuery } from '@/api/hooks';
import { api } from '@/api/client';
import { showToast } from '@/components/atoms/Toast';
import { useDashboardStore } from '@/stores/dashboard';
import { useAgentsStore } from '@/stores/agents';
import type { AgentOutputSubTab } from '@/stores/agents';
import {
  Badge,
  Button,
  Pill,
  ProgressBar,
  Sparkline,
  Spinner,
} from '@/components/atoms';
import { AgentCard } from '@/components/molecules/AgentCard';
import { GateResultRow } from '@/components/molecules/GateResultRow';
import DrawerOverlay from '@/components/layout/DrawerOverlay';

// ---------------------------------------------------------------------------
// API shapes
// ---------------------------------------------------------------------------

interface CreateAgentPayload {
  name: string;
  domain: string;
  role: string;
  model: string;
  provider: string;
}

// ---------------------------------------------------------------------------
// Role filter tabs
// ---------------------------------------------------------------------------

const ROLE_TABS: { label: string; value: string }[] = [
  { label: 'All',   value: 'all' },
  { label: 'Impl',  value: 'implementer' },
  { label: 'Strat', value: 'strategist' },
  { label: 'Arch',  value: 'architect' },
  { label: 'Audit', value: 'auditor' },
  { label: 'Crit',  value: 'critic' },
  { label: 'Cond',  value: 'conductor' },
  { label: 'Res',   value: 'researcher' },
];

// ---------------------------------------------------------------------------
// Status sort order
// ---------------------------------------------------------------------------

const STATUS_ORDER: Record<AgentStatus, number> = {
  active:    0,
  idle:      1,
  completed: 2,
  failed:    3,
  stopped:   4,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatTokensK(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000)     return `${(n / 1_000).toFixed(0)}k`;
  return String(n);
}

function formatCost(usd: number): string {
  const v = usd ?? 0;
  if (v < 0.001) return '$0.00';
  if (v < 0.01)  return `$${v.toFixed(4)}`;
  if (v < 1)     return `$${v.toFixed(3)}`;
  return `$${v.toFixed(2)}`;
}

function formatTimestamp(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleTimeString(undefined, { hour12: false });
  } catch {
    return iso;
  }
}

/** Context window fill colour, matching ProgressBar variant="cost" thresholds. */
function ctxFillColor(pct: number): string {
  if (pct >= 80) return 'var(--accent-error)';
  if (pct >= 50) return 'var(--warning)';
  return 'var(--sage)';
}

// ---------------------------------------------------------------------------
// useCreateAgent — mutation
// ---------------------------------------------------------------------------

function useCreateAgent() {
  const queryClient = useQueryClient();
  return useMutation<AgentState, Error, CreateAgentPayload>({
    mutationFn: (payload) =>
      // Map the form payload to the roko-serve create endpoint shape.
      api.post<AgentState>('/api/agents/create', {
        name: payload.name,
        domain: payload.domain,
        role: payload.role,
        model: payload.model,
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['agents'] });
    },
  });
}

// ---------------------------------------------------------------------------
// TokenSparkline — pseudo-trend from ordered agent token accumulation
// ---------------------------------------------------------------------------

function TokenSparkline({ agents }: { agents: AgentState[] }) {
  const data = useMemo(() => {
    const sorted = [...(Array.isArray(agents) ? agents : [])]
      .filter((a) => a.startedAt !== null && a.startedAt !== undefined)
      .sort(
        (a, b) =>
          new Date(a.startedAt!).getTime() - new Date(b.startedAt!).getTime(),
      );
    let running = 0;
    return sorted.map((a) => {
      running += (a.tokensIn ?? 0) + (a.tokensOut ?? 0);
      return running;
    });
  }, [agents]);

  if (data.length < 2) return null;

  return (
    <Sparkline
      data={data}
      height={18}
      color="var(--rose)"
      fill
      className="opacity-60"
    />
  );
}

// ---------------------------------------------------------------------------
// OutputTab — streaming text, auto-scroll tail, tool-call highlighting
// ---------------------------------------------------------------------------

function OutputTab({ agentId }: { agentId: string }) {
  const lines = useDashboardStore((s) => s.agentOutput[agentId] ?? []);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'auto' });
  }, [lines.length]);

  if (lines.length === 0) {
    return (
      <div className="flex items-center justify-center h-32">
        <span className="font-mono text-xs text-[var(--text-ghost)]">
          no output yet
        </span>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto min-h-0 p-3">
      <pre className="font-mono text-xs leading-relaxed whitespace-pre-wrap break-words">
        {lines.map((line, i) => {
          const isToolCall = line.startsWith('Tool:');
          return (
            <span
              key={i}
              className={clsx(
                'block',
                isToolCall
                  ? 'text-[var(--dream-bright)]'
                  : 'text-[var(--text-muted)]',
              )}
            >
              {line || '\u200b'}
            </span>
          );
        })}
      </pre>
      <div ref={bottomRef} />
    </div>
  );
}

// ---------------------------------------------------------------------------
// GatesTab — recent gate results filtered to this agent's current task
// ---------------------------------------------------------------------------

function GatesTab({ agentId }: { agentId: string }) {
  const allGates = useDashboardStore((s) => s.recentGates);
  const agent    = useDashboardStore((s) => s.agents[agentId]);
  const taskId   = agent?.currentTask ?? null;

  const gates = useMemo<GateResult[]>(() => {
    if (!taskId) return [];
    return allGates.filter((g) => g.taskId === taskId).slice(0, 6);
  }, [allGates, taskId]);

  if (gates.length === 0) {
    return (
      <div className="flex items-center justify-center h-32">
        <span className="font-mono text-xs text-[var(--text-ghost)]">
          no gate results
        </span>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto min-h-0">
      {gates.map((gate) => (
        <GateResultRow key={gate.id} gate={gate} expandable />
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// MetricsTab
// ---------------------------------------------------------------------------

function MetricsTab({ agent }: { agent: AgentState }) {
  const total  = (agent.tokensIn ?? 0) + (agent.tokensOut ?? 0);
  const ctxPct = Math.min(100, Math.max(0, agent.contextPct ?? 0));

  return (
    <div className="flex-1 overflow-y-auto min-h-0 p-4 flex flex-col gap-4">
      {/* Token summary */}
      <div className="flex flex-col gap-2">
        <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
          Tokens
        </span>
        <div className="grid grid-cols-3 gap-3">
          {(
            [
              ['Input',  agent.tokensIn],
              ['Output', agent.tokensOut],
              ['Total',  total],
            ] as [string, number][]
          ).map(([label, val]) => (
            <div key={label} className="flex flex-col gap-1">
              <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
                {label}
              </span>
              <span className="font-mono text-sm tabular-nums text-[var(--text-strong)] num">
                {val.toLocaleString()}
              </span>
            </div>
          ))}
        </div>
      </div>

      {/* Cost */}
      <div className="flex flex-col gap-1">
        <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
          Cost
        </span>
        <span className="font-mono text-sm tabular-nums text-[var(--text-strong)] num">
          {formatCost(agent.costUsd)}
        </span>
      </div>

      {/* Context window gauge */}
      <div className="flex flex-col gap-1.5">
        <div className="flex items-center justify-between">
          <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
            Context window
          </span>
          <span
            className="font-mono text-xs tabular-nums num"
            style={{ color: ctxFillColor(ctxPct) }}
          >
            {ctxPct.toFixed(0)}%
          </span>
        </div>
        <ProgressBar value={ctxPct} variant="cost" height={4} />
      </div>

      {/* Provider / model chips */}
      <div className="flex items-center gap-2 pt-2 border-t border-t-[var(--text-ghost)]">
        <span className="font-mono text-xs text-[var(--text-ghost)] border border-[var(--text-ghost)] px-1.5 py-0.5">
          {agent.provider}
        </span>
        <span className="font-mono text-xs text-[var(--text-ghost)] border border-[var(--text-ghost)] px-1.5 py-0.5">
          {agent.model}
        </span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// EpisodesTab
// ---------------------------------------------------------------------------

function EpisodesTab({ agentId }: { agentId: string }) {
  const allEpisodes = useDashboardStore((s) => s.recentEpisodes);
  const episodes    = useMemo<EpisodeEntry[]>(
    () => allEpisodes.filter((e) => e.agentId === agentId),
    [allEpisodes, agentId],
  );

  if (episodes.length === 0) {
    return (
      <div className="flex items-center justify-center h-32">
        <span className="font-mono text-xs text-[var(--text-ghost)]">
          no episodes recorded
        </span>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto min-h-0">
      {episodes.map((ep) => {
        const gateVariant =
          ep.gateResult === 'pass'
            ? 'success'
            : ep.gateResult === 'fail'
            ? 'error'
            : 'default';
        const fp = ep.hdcFingerprint ? ep.hdcFingerprint.slice(0, 8) : null;

        return (
          <div
            key={ep.id}
            className="flex items-center gap-2 px-3 py-2 border-b border-b-[var(--text-ghost)]"
          >
            <span className="font-mono text-xs text-[var(--text-ghost)] num tabular-nums shrink-0 w-16">
              {formatTimestamp(ep.timestamp)}
            </span>
            <span className="font-mono text-xs text-[var(--text-muted)] truncate flex-1">
              {ep.taskId}
            </span>
            <span className="font-mono text-xs text-[var(--text-faint)] num tabular-nums shrink-0">
              {formatTokensK((ep.tokensIn ?? 0) + (ep.tokensOut ?? 0))}t
            </span>
            <Badge variant={gateVariant}>{ep.gateResult}</Badge>
            {fp && (
              <span
                className="font-mono text-[10px] text-[var(--dream)] num tabular-nums shrink-0"
                title={ep.hdcFingerprint ?? undefined}
              >
                {fp}
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// QuickView — right 68% panel
// ---------------------------------------------------------------------------

const OUTPUT_SUB_TABS: { label: string; value: AgentOutputSubTab }[] = [
  { label: 'Output',   value: 'output' },
  { label: 'Gates',    value: 'gates' },
  { label: 'Metrics',  value: 'metrics' },
  { label: 'Episodes', value: 'episodes' },
];

function QuickView({
  agentId,
  agents,
}: {
  agentId: string | null;
  agents: Record<string, AgentState>;
}) {
  const subTab    = useAgentsStore((s) => s.outputSubTab);
  const setSubTab = useAgentsStore((s) => s.setOutputSubTab);

  if (!agentId) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <span className="font-mono text-sm text-[var(--text-ghost)]">
          Select an agent
        </span>
      </div>
    );
  }

  const agent = agents[agentId];
  if (!agent) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <span className="font-mono text-sm text-[var(--text-ghost)]">
          Agent not found
        </span>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full min-h-0 overflow-hidden">
      {/* Sub-tab bar */}
      <div
        className={clsx(
          'flex items-center justify-between shrink-0',
          'h-10 px-4',
          'border-b border-b-[var(--text-ghost)]',
        )}
      >
        <div className="flex items-center gap-1.5">
          {OUTPUT_SUB_TABS.map((t) => (
            <Pill
              key={t.value}
              active={subTab === t.value}
              onClick={() => setSubTab(t.value)}
            >
              {t.label}
            </Pill>
          ))}
        </div>

        <Link
          href={`/agents/${agent.id}`}
          className={clsx(
            'flex items-center gap-1',
            'font-mono text-xs text-[var(--text-faint)]',
            'hover:text-[var(--text-muted)]',
            'transition-colors duration-[80ms]',
          )}
        >
          View full detail
          <ArrowRight size={11} strokeWidth={1.5} aria-hidden />
        </Link>
      </div>

      {/* Tab content — each tab is responsible for its own overflow */}
      <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
        {subTab === 'output'   && <OutputTab   agentId={agentId} />}
        {subTab === 'gates'    && <GatesTab    agentId={agentId} />}
        {subTab === 'metrics'  && <MetricsTab  agent={agent} />}
        {subTab === 'episodes' && <EpisodesTab agentId={agentId} />}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// CreateAgentDrawer
// ---------------------------------------------------------------------------

const ROLE_OPTIONS = [
  'implementer',
  'strategist',
  'architect',
  'auditor',
  'critic',
  'conductor',
  'researcher',
];

const MODEL_OPTIONS = [
  'claude-sonnet-4-5',
  'claude-opus-4-5',
  'claude-haiku-3-5',
  'gpt-4o',
  'gpt-4o-mini',
  'gemini-1-5-pro',
  'gemini-1-5-flash',
];

const PROVIDER_OPTIONS = [
  'anthropic',
  'openai',
  'gemini',
  'cerebras',
  'perplexity',
  'local',
];

// Valid domains accepted by roko-serve POST /api/agents/create
const DOMAIN_OPTIONS = [
  'coding',
  'research',
  'chain',
  'general',
] as const;

const FIELD_INPUT_CLS = clsx(
  'w-full',
  'bg-[var(--bg-secondary)] border border-[var(--text-ghost)]',
  'font-mono text-sm text-[var(--text-strong)]',
  'px-2 py-1.5',
  'outline-none',
  'focus:border-[var(--rose)]',
  'transition-colors duration-[80ms]',
  'placeholder:text-[var(--text-ghost)]',
);

const FIELD_LABEL_CLS =
  'font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest mb-1 block';

function CreateAgentDrawer({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const createAgent = useCreateAgent();
  const [form, setForm] = useState<CreateAgentPayload>({
    name:     '',
    domain:   DOMAIN_OPTIONS[0],
    role:     ROLE_OPTIONS[0],
    model:    MODEL_OPTIONS[0],
    provider: PROVIDER_OPTIONS[0],
  });

  function patch(field: keyof CreateAgentPayload, value: string) {
    setForm((prev) => ({ ...prev, [field]: value }));
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!form.name.trim()) return;
    try {
      await createAgent.mutateAsync(form);
      setForm({
        name:     '',
        domain:   DOMAIN_OPTIONS[0],
        role:     ROLE_OPTIONS[0],
        model:    MODEL_OPTIONS[0],
        provider: PROVIDER_OPTIONS[0],
      });
      onClose();
      showToast(`Agent "${form.name}" created`, 'success');
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to create agent';
      showToast(msg, 'error');
    }
  }

  return (
    <DrawerOverlay open={open} onClose={onClose} title="Create agent" width={440}>
      <form onSubmit={handleSubmit} className="flex flex-col gap-5 p-4">
        {/* Name */}
        <div>
          <label className={FIELD_LABEL_CLS} htmlFor="ca-name">
            Name
          </label>
          <input
            id="ca-name"
            type="text"
            value={form.name}
            onChange={(e) => patch('name', e.target.value)}
            placeholder="e.g. alice"
            required
            className={FIELD_INPUT_CLS}
          />
        </div>

        {/* Domain */}
        <div>
          <label className={FIELD_LABEL_CLS} htmlFor="ca-domain">
            Domain
          </label>
          <select
            id="ca-domain"
            value={form.domain}
            onChange={(e) => patch('domain', e.target.value)}
            className={FIELD_INPUT_CLS}
          >
            {DOMAIN_OPTIONS.map((d) => (
              <option key={d} value={d}>{d}</option>
            ))}
          </select>
        </div>

        {/* Role */}
        <div>
          <label className={FIELD_LABEL_CLS} htmlFor="ca-role">
            Role
          </label>
          <select
            id="ca-role"
            value={form.role}
            onChange={(e) => patch('role', e.target.value)}
            className={FIELD_INPUT_CLS}
          >
            {ROLE_OPTIONS.map((r) => (
              <option key={r} value={r}>{r}</option>
            ))}
          </select>
        </div>

        {/* Model */}
        <div>
          <label className={FIELD_LABEL_CLS} htmlFor="ca-model">
            Model
          </label>
          <select
            id="ca-model"
            value={form.model}
            onChange={(e) => patch('model', e.target.value)}
            className={FIELD_INPUT_CLS}
          >
            {MODEL_OPTIONS.map((m) => (
              <option key={m} value={m}>{m}</option>
            ))}
          </select>
        </div>

        {/* Provider */}
        <div>
          <label className={FIELD_LABEL_CLS} htmlFor="ca-provider">
            Provider
          </label>
          <select
            id="ca-provider"
            value={form.provider}
            onChange={(e) => patch('provider', e.target.value)}
            className={FIELD_INPUT_CLS}
          >
            {PROVIDER_OPTIONS.map((p) => (
              <option key={p} value={p}>{p}</option>
            ))}
          </select>
        </div>

        {/* Error */}
        {createAgent.isError && (
          <div className="font-mono text-xs text-[var(--accent-error)] border border-[var(--accent-error)] px-2 py-1.5">
            {createAgent.error?.message ?? 'Failed to create agent'}
          </div>
        )}

        {/* Actions */}
        <div className="flex items-center justify-end gap-2 pt-2 border-t border-t-[var(--text-ghost)]">
          <Button type="button" variant="ghost" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button
            type="submit"
            variant="primary"
            size="sm"
            loading={createAgent.isPending}
            disabled={!form.name.trim()}
          >
            Create
          </Button>
        </div>
      </form>
    </DrawerOverlay>
  );
}

// ---------------------------------------------------------------------------
// AgentsPage
// ---------------------------------------------------------------------------

export default function AgentsPage() {
  // Store
  const liveAgents      = useDashboardStore((s) => s.agents);
  const selectedAgentId = useAgentsStore((s) => s.selectedAgentId);
  const roleFilter      = useAgentsStore((s) => s.roleFilter);
  const selectAgent     = useAgentsStore((s) => s.selectAgent);
  const setRoleFilter   = useAgentsStore((s) => s.setRoleFilter);

  // React Query initial hydration from /api/managed-agents
  const { data: queriedAgents, isLoading: agentsLoading, isError: agentsError } = useAgentsQuery();

  // Drawer
  const [drawerOpen, setDrawerOpen] = useState(false);

  // Merge HTTP snapshot into live Zustand map; SSE wins on conflict
  const agents = useMemo<Record<string, AgentState>>(() => {
    const seed: Record<string, AgentState> = {};
    if (queriedAgents) {
      for (const a of queriedAgents) seed[a.id] = a;
    }
    return { ...seed, ...liveAgents };
  }, [liveAgents, queriedAgents]);

  const agentList = useMemo<AgentState[]>(() => Object.values(agents), [agents]);

  // Derived counters
  const activeCount = useMemo(
    () => agentList.filter((a) => a.status === 'active').length,
    [agentList],
  );
  const totalTokens = useMemo(
    () => agentList.reduce((sum, a) => sum + (a.tokensIn ?? 0) + (a.tokensOut ?? 0), 0),
    [agentList],
  );
  const totalCost = useMemo(
    () => agentList.reduce((sum, a) => sum + (a.costUsd ?? 0), 0),
    [agentList],
  );

  // Filtered and sorted roster
  const filtered = useMemo<AgentState[]>(() => {
    const base =
      roleFilter === 'all'
        ? agentList
        : agentList.filter(
            (a) => (a.role ?? '').toLowerCase() === roleFilter.toLowerCase(),
          );
    return [...base].sort((a, b) => {
      const so =
        (STATUS_ORDER[a.status] ?? 99) - (STATUS_ORDER[b.status] ?? 99);
      if (so !== 0) return so;
      const ro = (a.role ?? '').localeCompare(b.role ?? '');
      if (ro !== 0) return ro;
      return (a.id ?? '').localeCompare(b.id ?? '');
    });
  }, [agentList, roleFilter]);

  // Keyboard navigation on the roster panel
  const handleRosterKey = useCallback(
    (e: React.KeyboardEvent) => {
      if (filtered.length === 0) return;
      const idx = filtered.findIndex((a) => a.id === selectedAgentId);
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        selectAgent(filtered[(idx + 1) % filtered.length].id);
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        selectAgent(
          filtered[(idx - 1 + filtered.length) % filtered.length].id,
        );
      } else if (e.key === 'Escape') {
        selectAgent(null);
      }
    },
    [filtered, selectedAgentId, selectAgent],
  );

  return (
    // The agents layout wraps children in overflow-y-auto; we break out of
    // that scroll by making this container fill the parent and clip itself.
    <div className="absolute inset-0 flex overflow-hidden">
      {/* ================================================================
          Left panel — Agent Roster (32%, min 280px)
          ================================================================ */}
      <div
        className={clsx(
          'flex flex-col shrink-0 h-full',
          'bg-[var(--bg-raised)]',
          'border-r border-r-[var(--text-ghost)]',
          'overflow-hidden',
        )}
        style={{ width: '32%', minWidth: 280 }}
        // eslint-disable-next-line jsx-a11y/no-noninteractive-element-to-interactive-role
        role="region"
        aria-label="Agent roster"
        onKeyDown={handleRosterKey}
      >
        {/* Header row: title + active count badge */}
        <div
          className={clsx(
            'flex items-center gap-2 shrink-0',
            'h-10 px-4',
            'border-b border-b-[var(--text-ghost)]',
          )}
        >
          <span className="flex-1 font-mono text-sm text-[var(--text-strong)] leading-none font-medium tracking-wide">
            Agents
          </span>
          <span
            className={clsx(
              'font-mono text-xs num tabular-nums leading-none',
              'border px-1.5 py-0.5',
              activeCount > 0
                ? 'text-[var(--sage)] border-[var(--sage)]'
                : 'text-[var(--text-ghost)] border-[var(--text-ghost)]',
            )}
          >
            {activeCount}/{agentList.length} active
          </span>
        </div>

        {/* Role filter pills */}
        <div
          className={clsx(
            'flex items-center gap-1 flex-wrap shrink-0',
            'px-3 py-2',
            'border-b border-b-[var(--text-ghost)]',
          )}
        >
          {ROLE_TABS.map((tab) => (
            <Pill
              key={tab.value}
              active={roleFilter === tab.value}
              onClick={() => setRoleFilter(tab.value)}
            >
              {tab.label}
            </Pill>
          ))}
        </div>

        {/* Scrollable agent list */}
        <div
          className="flex-1 overflow-y-auto overflow-x-hidden"
          role="listbox"
          aria-label="Agents"
        >
          {agentsLoading && agentList.length === 0 ? (
            <div className="flex items-center justify-center gap-2 py-10">
              <Spinner size="sm" />
              <span className="font-mono text-xs text-[var(--text-ghost)]">
                Loading agents…
              </span>
            </div>
          ) : agentsError && agentList.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-10 px-4 gap-1.5 text-center">
              <span className="font-mono text-xs text-[var(--accent-error)]">
                Failed to load agents.
              </span>
              <span className="font-mono text-[10px] text-[var(--text-faint)]">
                Is roko serve running on :6677?
              </span>
            </div>
          ) : filtered.length === 0 ? (
            <div className="flex items-center justify-center py-10">
              <span className="font-mono text-xs text-[var(--text-ghost)]">
                {roleFilter === 'all'
                  ? 'No agents registered yet'
                  : `No ${roleFilter} agents`}
              </span>
            </div>
          ) : (
            filtered.map((agent) => (
              <AgentCard
                key={agent.id}
                agent={agent}
                selected={selectedAgentId === agent.id}
                onClick={() =>
                  selectAgent(
                    selectedAgentId === agent.id ? null : agent.id,
                  )
                }
              />
            ))
          )}
        </div>

        {/* Footer: sparkline + summary line */}
        <div
          className={clsx(
            'shrink-0',
            'border-t border-t-[var(--text-ghost)]',
            'px-3 pt-2 pb-1',
            'flex flex-col gap-1.5',
          )}
        >
          <TokenSparkline agents={agentList} />

          <span className="font-mono text-[10px] text-[var(--text-faint)] num tabular-nums leading-none">
            agents:&thinsp;
            <span className="text-[var(--text-muted)]">{activeCount}</span>
            <span>/{agentList.length}</span>
            &emsp;tokens:&thinsp;
            <span className="text-[var(--text-muted)]">
              {formatTokensK(totalTokens)}
            </span>
            &emsp;cost:&thinsp;
            <span className="text-[var(--text-muted)]">
              {formatCost(totalCost)}
            </span>
          </span>
        </div>

        {/* Floating action button — Create Agent */}
        <div className="shrink-0 px-3 pb-3">
          <Button
            variant="secondary"
            size="sm"
            className="w-full gap-1.5"
            onClick={() => setDrawerOpen(true)}
          >
            <Plus size={12} strokeWidth={1.5} aria-hidden />
            New agent
          </Button>
        </div>
      </div>

      {/* ================================================================
          Right panel — Quick-View (remaining width, ~68%)
          ================================================================ */}
      <div
        className={clsx(
          'flex-1 flex flex-col min-w-0 h-full',
          'bg-[var(--bg-secondary)]',
          'overflow-hidden',
        )}
      >
        <QuickView agentId={selectedAgentId} agents={agents} />
      </div>

      {/* ================================================================
          Create Agent Drawer
          ================================================================ */}
      <CreateAgentDrawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
      />
    </div>
  );
}
