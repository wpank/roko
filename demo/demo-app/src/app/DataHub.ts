/**
 * DataHub — centralised Zustand store for the demo-app.
 *
 * Canonical owner for event dispatch, config, health, and workspace state.
 * Components consume focused hooks from `data/selectors.ts`.
 *
 * Implementation tasks: T1.9 (core store) + T1.10 (workspace slice).
 */

import { create } from 'zustand';
import type {
  DashboardEvent,
  DashboardGapPayload,
  DashboardSnapshot,
} from '../transport/types';
import { api } from '../transport/api';
import { parseBenchRunsListResponse } from '../lib/bench-types';
import type {
  BenchModel,
  BenchModelsResponse,
  BenchRunListEntry,
  BenchSSEEvent,
  BenchSuiteListEntry,
  BenchSuitesResponse,
} from '../lib/bench-types';
import type {
  ConnectedAgent as RelayConnectedAgent,
  ConnectedWorkspace as RelayConnectedWorkspace,
  FeedDescriptor as RelayFeedDescriptor,
  TopicInfo as RelayTopicInfo,
  RelayEvent,
} from '../lib/relay-api';
import {
  fetchRelayAgents,
  fetchRelayWorkspaces,
  fetchRelayFeeds,
  fetchRelayTopics,
} from '../lib/relay-api';
import {
  providerForModelKey,
  rawModelsToOptions,
  resolveModelKey,
  type RawConfigModels,
} from '../lib/config-models';

// ── Public types ────────────────────────────────────────────────

export type ServerStatus = 'connected' | 'checking' | 'disconnected';
export type StreamStatus =
  | 'idle'
  | 'connecting'
  | 'connected'
  | 'reconnecting'
  | 'failed';

export interface WorkspaceInfo {
  id: string;
  path: string;
  ready: boolean;
}

export interface AgentInfo {
  agentId: string;
  role: string;
  model: string;
  status: 'running' | 'stopped';
}

export interface EpisodeInfo {
  episodeId: string;
  agentId: string;
  role: string;
  passed: boolean;
  timestamp: number;
}

export interface InferenceRecord {
  requestId: string;
  model: string;
  agentId: string;
  inputTokens: number;
  outputTokens: number;
  costUsd: number;
  durationMs: number;
}

export interface SequencedBenchEvent {
  sequence: number;
  event: BenchSSEEvent;
}


// ── Feed types ──────────────────────────────────────────────────

export interface RelayFeed {
  feedId: string;
  topic: string;
  name: string;
  description: string;
  kind: 'raw' | 'derived' | 'composite' | 'meta';
  rate: string;
  agentId: string;
  agentName: string;
  lastValue: unknown | null;
  lastUpdateMs: number | null;
  messageCount: number;
  sparkline: number[];
  status: 'live' | 'stale' | 'offline';
}

export interface RelayAgentEntry {
  agentId: string;
  name: string;
  capabilities: string[];
  feedCount: number;
  connectedAtMs: number;
  online: boolean;
}

export interface FeedLogEntry {
  ts: number;
  agentId: string;
  topic: string;
  preview: string;
}

// ── Chain types ──────────────────────────────────────────────────

export interface ChainBlockEntry {
  number: number;
  hash: string;
  parentHash: string;
  timestamp: number;
  gasUsed: number;
  gasLimit: number;
  txCount: number;
  baseFeePerGas: number | null;
}

export interface ChainTxEntry {
  blockNumber: number;
  txHash: string;
  from: string;
  to: string | null;
  valueWei: string;
  gasUsed: number;
  methodSig: string | null;
  success: boolean;
}

export interface ChainEventEntry {
  blockNumber: number;
  txHash: string;
  logIndex: number;
  contract: string;
  eventName: string;
  decoded: Record<string, unknown>;
}

// ── Relay dashboard types ────────────────────────────────────

export interface RelayEventLogEntry {
  ts: number;
  type: string;
  message: string;
}

export interface RelayFeedGroup {
  agent_id: string;
  feeds: RelayFeedDescriptor[];
}

// ── Store interface ─────────────────────────────────────────────

export interface DataHub {
  // -- Connection status -------------------------------------------
  serverStatus: ServerStatus;
  sseStatus: StreamStatus;
  wsStatus: StreamStatus;
  dashboardMissedEvents: number;
  dashboardLastMaterializedSeq: number | null;

  // -- Config slice ------------------------------------------------
  config: Record<string, unknown> | null;
  defaultModel: string;
  defaultBackend: string;
  lastConfigSavedAt: number | null;

  // -- Workspace slice ---------------------------------------------
  serverWorkdir: string | null;
  workspace: WorkspaceInfo | null;
  workspaceCache: Map<string, WorkspaceInfo>;

  // -- Plan execution slice ----------------------------------------
  activePlanId: string | null;
  activePhase: string | null;
  planCompleted: boolean;

  // -- Agent slice -------------------------------------------------
  agents: AgentInfo[];

  // -- Episode / metrics slice -------------------------------------
  episodes: EpisodeInfo[];
  totalCost: number;
  totalTokens: number;
  recentInferences: InferenceRecord[]; // ring buffer, max 200

  // -- Bench slice -------------------------------------------------
  benchRuns: BenchRunListEntry[];
  benchSuites: BenchSuiteListEntry[];
  benchModels: BenchModel[];
  benchSseStatus: StreamStatus;
  benchEventSequence: number;
  benchEvents: SequencedBenchEvent[];

  // -- Actions: event handling -------------------------------------
  handleServerEvent: (event: DashboardEvent) => void;
  hydrateDashboardSnapshot: (
    snapshot: DashboardSnapshot,
    gap: Pick<DashboardGapPayload, 'missed_events' | 'last_materialized_seq'>,
  ) => void;
  handleBenchEvent: (event: BenchSSEEvent) => void;
  setServerStatus: (status: ServerStatus) => void;
  setSseStatus: (status: StreamStatus) => void;
  setBenchSseStatus: (status: StreamStatus) => void;
  setWsStatus: (status: StreamStatus) => void;

  // -- Actions: REST fetches ---------------------------------------
  fetchConfig: () => Promise<void>;
  updateConfig: (partial: Record<string, unknown>) => Promise<boolean>;
  updateModelConfig: (model: string, backend: string) => Promise<boolean>;
  fetchBenchRuns: () => Promise<void>;
  fetchBenchSuites: () => Promise<void>;
  fetchBenchModels: () => Promise<void>;
  fetchAgents: () => Promise<void>;
  fetchServerWorkdir: () => Promise<void>;
  ensureWorkspace: (
    prefix: string,
    opts?: { gitInit?: boolean },
  ) => Promise<WorkspaceInfo>;
  createWorkspace: (
    prefix: string,
    opts?: { gitInit?: boolean },
  ) => Promise<WorkspaceInfo>;
  destroyWorkspace: (id: string) => Promise<void>;

  // -- Chain slice ------------------------------------------------
  chainBlocks: ChainBlockEntry[];
  chainTxs: ChainTxEntry[];
  chainEvents: ChainEventEntry[];
  chainLatestBlock: ChainBlockEntry | null;
  chainWatcherRunning: boolean;
  chainGasHistory: number[];

  // -- Actions: Chain REST fetches --------------------------------
  fetchChainBlocks: () => Promise<void>;
  fetchChainTxs: () => Promise<void>;
  fetchChainEvents: () => Promise<void>;
  fetchChainStatus: () => Promise<void>;

  // -- Feed slice -------------------------------------------------
  relayFeeds: RelayFeed[];
  relayAgents: RelayAgentEntry[];
  feedLog: FeedLogEntry[];
  feedThroughput: number[];

  // -- Actions: Feed REST fetches ---------------------------------
  fetchFeedCatalog: () => Promise<void>;

  // -- Relay dashboard slice ------------------------------------
  relayDashAgents: RelayConnectedAgent[];
  relayDashWorkspaces: RelayConnectedWorkspace[];
  relayDashFeeds: RelayFeedGroup[];
  relayDashTopics: RelayTopicInfo[];
  relayDashEventLog: RelayEventLogEntry[];

  // -- Actions: Relay dashboard ---------------------------------
  fetchRelayDashboard: () => Promise<void>;
  handleRelayEvent: (event: RelayEvent) => void;
}

// ── Ring-buffer limits ──────────────────────────────────────────

const MAX_EPISODES = 500;
const MAX_BENCH_EVENTS = 500;
const MAX_CHAIN_BLOCKS = 64;
const MAX_CHAIN_TXS = 128;
const MAX_CHAIN_EVENTS = 128;
const MAX_CHAIN_GAS_HISTORY = 64;
const MAX_FEED_LOG = 200;
const MAX_FEED_SPARKLINE = 30;
// const MAX_FEED_THROUGHPUT = 60; // reserved for future throughput sparkline
const MAX_RELAY_EVENT_LOG = 200;

function configState(config: Record<string, unknown>) {
  const agent = config.agent as Record<string, string> | undefined;
  const models = rawModelsToOptions(config.models as RawConfigModels | undefined);
  const defaultModel = agent?.default_model
    ? resolveModelKey(models, agent.default_model)
    : '';
  const defaultBackend =
    providerForModelKey(models, defaultModel) ?? agent?.default_backend ?? '';
  return { config, defaultModel, defaultBackend };
}

// ── Relay dashboard helpers ─────────────────────────────────

function relayEventMessage(event: RelayEvent): string {
  switch (event.type) {
    case 'agent_connected':
      return `Agent connected: ${event.agent.name ?? event.agent.agent_id}`;
    case 'agent_disconnected':
      return `Agent disconnected: ${event.agent_id}`;
    case 'workspace_connected':
      return `Workspace connected: ${event.workspace.name ?? event.workspace.workspace_id}`;
    case 'workspace_disconnected':
      return `Workspace disconnected: ${event.workspace_id}`;
    case 'workspace_heartbeat':
      return `Heartbeat: ${event.workspace_id} (${event.agents_count} agents)`;
    case 'feed_registered':
      return `Feed registered: ${event.feed.name} on ${event.feed.topic}`;
    case 'feed_unregistered':
      return `Feed unregistered: ${event.feed_id} from ${event.agent_id}`;
    case 'card_updated':
      return `Card updated: ${event.agent_id}`;
    case 'message_delivered':
      return `Message delivered to ${event.agent_id}`;
    case 'message_responded':
      return `Response from ${event.agent_id}`;
    case 'agent_error':
      return `Error from ${event.agent_id}: ${event.error}`;
  }
}

function upsertFeed(
  groups: { agent_id: string; feeds: RelayFeedDescriptor[] }[],
  agentId: string,
  feed: RelayFeedDescriptor,
): { agent_id: string; feeds: RelayFeedDescriptor[] }[] {
  const idx = groups.findIndex((g) => g.agent_id === agentId);
  if (idx >= 0) {
    const existing = groups[idx];
    const feeds = [...existing.feeds.filter((f) => f.feed_id !== feed.feed_id), feed];
    const next = [...groups];
    next[idx] = { agent_id: agentId, feeds };
    return next;
  }
  return [...groups, { agent_id: agentId, feeds: [feed] }];
}

function removeFeed(
  groups: { agent_id: string; feeds: RelayFeedDescriptor[] }[],
  agentId: string,
  feedId: string,
): { agent_id: string; feeds: RelayFeedDescriptor[] }[] {
  return groups
    .map((g) =>
      g.agent_id === agentId
        ? { ...g, feeds: g.feeds.filter((f) => f.feed_id !== feedId) }
        : g,
    )
    .filter((g) => g.feeds.length > 0);
}

// ── Store implementation ────────────────────────────────────────

export const useDataHub = create<DataHub>()((set, get) => ({
  // -- Initial state -----------------------------------------------
  serverStatus: 'checking',
  sseStatus: 'idle',
  wsStatus: 'idle',
  dashboardMissedEvents: 0,
  dashboardLastMaterializedSeq: null,
  config: null,
  defaultModel: '',
  defaultBackend: '',
  lastConfigSavedAt: null,
  serverWorkdir: null,
  workspace: null,
  workspaceCache: new Map(),
  activePlanId: null,
  activePhase: null,
  planCompleted: false,
  agents: [],
  episodes: [],
  totalCost: 0,
  totalTokens: 0,
  recentInferences: [],
  benchRuns: [],
  benchSuites: [],
  benchModels: [],
  benchSseStatus: 'idle',
  benchEventSequence: 0,
  benchEvents: [],
  chainBlocks: [],
  chainTxs: [],
  chainEvents: [],
  chainLatestBlock: null,
  chainWatcherRunning: false,
  chainGasHistory: [],
  relayFeeds: [],
  relayAgents: [],
  feedLog: [],
  feedThroughput: [],
  relayDashAgents: [],
  relayDashWorkspaces: [],
  relayDashFeeds: [],
  relayDashTopics: [],
  relayDashEventLog: [],

  // -- Event handling -----------------------------------------------

  handleServerEvent: (event: DashboardEvent) => {
    // Wire events arrive with snake_case field names matching the server
    // serde contract. This handler is the single adapter boundary: it maps
    // snake_case wire fields to the camelCase shapes used by the UI store.
    switch (event.type) {
      case 'plan_started':
        set({
          activePlanId: event.plan_id,
          activePhase: 'started',
          planCompleted: false,
        });
        break;

      case 'plan_completed':
        set({ planCompleted: true, activePhase: 'completed' });
        break;

      case 'phase_transition':
        set({ activePhase: event.to });
        break;

      case 'agent_spawned':
        set((s) => ({
          agents: [
            ...s.agents,
            {
              agentId: event.agent_id,
              role: event.role,
              model: event.model,
              status: 'running' as const,
            },
          ],
        }));
        break;

      case 'agent_completed':
        set((s) => ({
          agents: s.agents.map((a) =>
            a.agentId === event.agent_id
              ? { ...a, status: 'stopped' as const }
              : a,
          ),
        }));
        break;

      case 'episode_recorded':
        set((s) => ({
          episodes: [
            ...s.episodes.slice(-(MAX_EPISODES - 1)),
            {
              episodeId: event.episode_id,
              agentId: event.agent_id,
              role: event.role,
              passed: event.passed,
              timestamp: Date.now(),
            },
          ],
        }));
        break;

      case 'gate_result':
        // Consumed by components via raw event subscriptions; no store update.
        break;

      case 'chain_block':
        set((s) => ({
          chainLatestBlock: {
            number: event.number,
            hash: event.hash,
            parentHash: event.parent_hash,
            timestamp: event.timestamp,
            gasUsed: event.gas_used,
            gasLimit: event.gas_limit,
            txCount: event.tx_count,
            baseFeePerGas: event.base_fee_per_gas,
          },
          chainBlocks: [
            {
              number: event.number,
              hash: event.hash,
              parentHash: event.parent_hash,
              timestamp: event.timestamp,
              gasUsed: event.gas_used,
              gasLimit: event.gas_limit,
              txCount: event.tx_count,
              baseFeePerGas: event.base_fee_per_gas,
            },
            ...s.chainBlocks.slice(0, MAX_CHAIN_BLOCKS - 1),
          ],
          chainGasHistory: [
            ...s.chainGasHistory.slice(-(MAX_CHAIN_GAS_HISTORY - 1)),
            event.gas_used,
          ],
        }));
        break;

      case 'chain_tx':
        set((s) => ({
          chainTxs: [
            {
              blockNumber: event.block_number,
              txHash: event.tx_hash,
              from: event.from,
              to: event.to,
              valueWei: event.value_wei,
              gasUsed: event.gas_used,
              methodSig: event.method_sig,
              success: event.success,
            },
            ...s.chainTxs.slice(0, MAX_CHAIN_TXS - 1),
          ],
        }));
        break;

      case 'chain_contract_event':
        set((s) => ({
          chainEvents: [
            {
              blockNumber: event.block_number,
              txHash: event.tx_hash,
              logIndex: event.log_index,
              contract: event.contract,
              eventName: event.event_name,
              decoded: event.decoded !== null
                && typeof event.decoded === 'object'
                && !Array.isArray(event.decoded)
                ? event.decoded as Record<string, unknown>
                : {},
            },
            ...s.chainEvents.slice(0, MAX_CHAIN_EVENTS - 1),
          ],
        }));
        break;

      case 'feed_tick':
        set((s) => {
          // Find and update the matching feed.
          const feedIdx = s.relayFeeds.findIndex((f) => f.feedId === event.feed_id);
          const now = Date.now();

          // Extract a numeric value for sparkline (try common keys).
          // Feed payloads arrive as raw server JSON — keys are snake_case.
          const numericValue = (() => {
            if (typeof event.payload === 'object' && event.payload !== null) {
              const p = event.payload as Record<string, unknown>;
              for (const key of ['composite_bps', 'ema_gwei', 'rate_bps', 'spread_bps', 'stddev_bps', 'confidence_pct', 'block_number', 'epoch', 'number', 'current_epoch', 'max_spread_bps', 'relay_agent_count', 'total_feeds']) {
                if (typeof p[key] === 'number') return p[key] as number;
              }
            }
            return 0;
          })();

          // Build a short preview string for the log.
          const preview = typeof event.payload === 'object' && event.payload !== null
            ? Object.entries(event.payload as Record<string, unknown>)
                .slice(0, 3)
                .map(([k, v]) => `${k}:${typeof v === 'number' ? (v as number).toFixed(0) : v}`)
                .join(' ')
            : String(event.payload);

          const updatedFeeds = [...s.relayFeeds];
          if (feedIdx >= 0) {
            const feed = { ...updatedFeeds[feedIdx] };
            feed.lastValue = event.payload;
            feed.lastUpdateMs = now;
            feed.messageCount += 1;
            feed.status = 'live';
            feed.sparkline = [...feed.sparkline.slice(-(MAX_FEED_SPARKLINE - 1)), numericValue];
            updatedFeeds[feedIdx] = feed;
          }

          return {
            relayFeeds: updatedFeeds,
            feedLog: [
              { ts: now, agentId: event.agent_id, topic: event.topic, preview },
              ...s.feedLog.slice(0, MAX_FEED_LOG - 1),
            ],
          };
        });
        break;

      case 'feed_agent_online':
        set((s) => {
          const exists = s.relayAgents.findIndex((a) => a.agentId === event.agent_id);
          const entry: RelayAgentEntry = {
            agentId: event.agent_id,
            name: event.name,
            capabilities: [],
            feedCount: event.feed_count,
            connectedAtMs: Date.now(),
            online: true,
          };

          const agents = [...s.relayAgents];
          if (exists >= 0) {
            agents[exists] = entry;
          } else {
            agents.push(entry);
          }
          return { relayAgents: agents };
        });
        break;

      case 'feed_agent_offline':
        set((s) => ({
          relayAgents: s.relayAgents.map((a) =>
            a.agentId === event.agent_id ? { ...a, online: false } : a,
          ),
          relayFeeds: s.relayFeeds.map((f) =>
            f.agentId === event.agent_id ? { ...f, status: 'offline' as const } : f,
          ),
        }));
        break;

      case 'error':
        console.warn('[DataHub] server error:', event.message);
        break;

      default:
        // Unknown events silently ignored.
        break;
    }
  },

  hydrateDashboardSnapshot: (snapshot, gap) => {
    const plans = Object.values(snapshot.plans);
    const selectedPlan = plans.find((plan) => plan.active) ?? null;
    const agents = Object.values(snapshot.agents).map((agent) => ({
      agentId: agent.agent_id,
      role: agent.role,
      model: agent.model,
      status: agent.active ? 'running' as const : 'stopped' as const,
    }));
    const episodes = snapshot.episodes.map((episode) => ({
      episodeId: episode.episode_id,
      agentId: episode.agent_id,
      role: episode.role,
      passed: episode.passed,
      timestamp: episode.ts_millis,
    }));
    const totalTokens = Object.values(snapshot.agents).reduce(
      (sum, agent) => sum + agent.input_tokens + agent.output_tokens,
      0,
    );

    // One set() call makes the validated materialized snapshot visible as a
    // single state transition; no replay subscriber can observe a half-hydrated hub.
    set({
      activePlanId: selectedPlan?.plan_id ?? null,
      activePhase: selectedPlan?.phase ?? null,
      planCompleted: selectedPlan === null && plans.length > 0,
      agents,
      episodes,
      totalCost: snapshot.stats.cost_usd_total,
      totalTokens,
      dashboardMissedEvents: gap.missed_events,
      dashboardLastMaterializedSeq: gap.last_materialized_seq,
    });
  },

  handleBenchEvent: (event) => {
    set((s) => {
      const sequence = s.benchEventSequence + 1;
      return {
        benchEventSequence: sequence,
        benchEvents: [
          ...s.benchEvents.slice(-(MAX_BENCH_EVENTS - 1)),
          { sequence, event },
        ],
      };
    });
    if (event.type === 'BenchRunCompleted') {
      void get().fetchBenchRuns();
    }
  },

  // -- Status setters -----------------------------------------------

  setServerStatus: (status) => set({ serverStatus: status }),
  setSseStatus: (status) => set({ sseStatus: status }),
  setBenchSseStatus: (status) => set({ benchSseStatus: status }),
  setWsStatus: (status) => set({ wsStatus: status }),

  // -- REST fetch actions -------------------------------------------

  fetchConfig: async () => {
    const res = await api.get<Record<string, unknown>>('/api/config');
    if (res.ok) {
      set(configState(res.data));
    }
  },

  updateConfig: async (partial) => {
    if (get().serverStatus !== 'connected') return false;
    const res = await api.put<Record<string, unknown>>(
      '/api/config',
      partial,
    );
    if (res.ok) {
      set({ ...configState(res.data), lastConfigSavedAt: Date.now() });
      return true;
    }
    return false;
  },

  updateModelConfig: async (model, backend) => {
    if (get().serverStatus !== 'connected') return false;
    const config = get().config;
    const models = rawModelsToOptions(config?.models as RawConfigModels | undefined);
    const modelKey = resolveModelKey(models, model);
    const modelBackend = providerForModelKey(models, modelKey) ?? backend;
    return get().updateConfig({
      agent: { default_model: modelKey, default_backend: modelBackend },
    });
  },

  fetchBenchRuns: async () => {
    const res = await api.get<unknown>('/api/bench/runs');
    if (res.ok) {
      const listing = parseBenchRunsListResponse(res.data);
      if (listing) set({ benchRuns: listing.runs });
    }
  },

  fetchBenchSuites: async () => {
    const res = await api.get<BenchSuitesResponse>('/api/bench/suites');
    if (res.ok) set({ benchSuites: res.data.suites });
  },

  fetchBenchModels: async () => {
    const res = await api.get<BenchModelsResponse>('/api/bench/models');
    if (res.ok) {
      set({
        benchModels: res.data.models.map((model) => ({
          id: model,
          name: model,
          provider: '',
          cost_per_1k_input: 0,
          cost_per_1k_output: 0,
          max_tokens: 0,
          context_window: 0,
        })),
      });
    }
  },

  fetchAgents: async () => {
    const res = await api.get<AgentInfo[]>('/api/managed-agents');
    if (res.ok) set({ agents: res.data });
  },

  // -- Workspace actions (T1.10) -----------------------------------

  fetchServerWorkdir: async () => {
    const res = await api.get<{ path: string }>(
      '/api/workspaces/default',
    );
    if (res.ok) set({ serverWorkdir: res.data.path });
  },

  createWorkspace: async (prefix, opts) => {
    const res = await api.post<WorkspaceInfo>('/api/workspaces', {
      prefix,
      git_init: opts?.gitInit ?? true,
    });
    if (!res.ok) {
      throw new Error(
        `Failed to create workspace: ${res.error.status} ${res.error.body ?? res.error.statusText}`,
      );
    }
    const ws = res.data;
    set({ workspace: ws });
    return ws;
  },

  ensureWorkspace: async (prefix, opts) => {
    const cached = get().workspaceCache.get(prefix);
    if (cached) return cached;

    const ws = await get().createWorkspace(prefix, opts);
    set((s) => {
      const next = new Map(s.workspaceCache);
      next.set(prefix, ws);
      return { workspace: ws, workspaceCache: next };
    });
    return ws;
  },

  destroyWorkspace: async (id) => {
    await api.delete(`/api/workspaces/${encodeURIComponent(id)}`);
    set((s) => {
      const next = new Map(s.workspaceCache);
      for (const [key, ws] of next.entries()) {
        if (ws.id === id) {
          next.delete(key);
          break;
        }
      }
      return {
        workspace: s.workspace?.id === id ? null : s.workspace,
        workspaceCache: next,
      };
    });
  },

  // -- Chain fetch actions -----------------------------------------

  fetchChainBlocks: async () => {
    try {
      const res = await api.get<{ blocks: Array<{
        number: number; hash: string; parent_hash: string; timestamp: number;
        gas_used: number; gas_limit: number; tx_count: number; base_fee_per_gas: number | null;
      }> }>('/api/chain/blocks?limit=64');
      if (res.ok) {
        const mapped = res.data.blocks.map((b) => ({
          number: b.number,
          hash: b.hash,
          parentHash: b.parent_hash,
          timestamp: b.timestamp,
          gasUsed: b.gas_used,
          gasLimit: b.gas_limit,
          txCount: b.tx_count,
          baseFeePerGas: b.base_fee_per_gas,
        }));
        set((s) => {
          // Only seed from REST when SSE hasn't already pushed data.
          if (s.chainBlocks.length > 0) return {};
          const seedGas =
            s.chainGasHistory.length === 0 && mapped.length > 0
              ? [...mapped].reverse().map((b) => b.gasUsed)
              : s.chainGasHistory;
          const latestBlock =
            s.chainLatestBlock ?? (mapped.length > 0 ? mapped[0] : null);
          return {
            chainBlocks: mapped,
            chainGasHistory: seedGas,
            chainLatestBlock: latestBlock,
          };
        });
      }
    } catch (err) {
      console.warn('[DataHub] fetchChainBlocks failed:', err);
    }
  },

  fetchChainTxs: async () => {
    try {
      const res = await api.get<{ transactions: Array<{
        block_number: number; tx_hash: string; from: string; to: string | null;
        value_wei: string; gas_used: number; method_sig: string | null; success: boolean;
      }> }>('/api/chain/transactions?limit=128');
      if (res.ok) {
        const mapped = res.data.transactions.map((t) => ({
          blockNumber: t.block_number,
          txHash: t.tx_hash,
          from: t.from,
          to: t.to,
          valueWei: t.value_wei,
          gasUsed: t.gas_used,
          methodSig: t.method_sig,
          success: t.success,
        }));
        // Only seed if SSE hasn't already pushed data (avoids clobbering live stream)
        set((s) => (s.chainTxs.length === 0 ? { chainTxs: mapped } : {}));
      }
    } catch (err) {
      console.warn('[DataHub] fetchChainTxs failed:', err);
    }
  },

  fetchChainEvents: async () => {
    try {
      const res = await api.get<{ events: Array<{
        block_number: number; tx_hash: string; log_index: number;
        contract: string; event_name: string; decoded: Record<string, unknown>;
      }> }>('/api/chain/events?limit=128');
      if (res.ok) {
        const mapped = res.data.events.map((e) => ({
          blockNumber: e.block_number,
          txHash: e.tx_hash,
          logIndex: e.log_index,
          contract: e.contract,
          eventName: e.event_name,
          decoded: e.decoded,
        }));
        // Only seed if SSE hasn't already pushed data (avoids clobbering live stream)
        set((s) => (s.chainEvents.length === 0 ? { chainEvents: mapped } : {}));
      }
    } catch (err) {
      console.warn('[DataHub] fetchChainEvents failed:', err);
    }
  },

  fetchChainStatus: async () => {
    try {
      const res = await api.get<{ watcher_running: boolean; latest_block: number | null }>('/api/chain/watcher');
      if (res.ok) {
        set({ chainWatcherRunning: res.data.watcher_running });
      }
    } catch (err) {
      console.warn('[DataHub] fetchChainStatus failed:', err);
    }
  },

  // -- Feed fetch actions -----------------------------------------

  // -- Relay dashboard actions -----------------------------------

  fetchRelayDashboard: async () => {
    const [agentsRes, workspacesRes, feedsRes, topicsRes] = await Promise.all([
      fetchRelayAgents(),
      fetchRelayWorkspaces(),
      fetchRelayFeeds(),
      fetchRelayTopics(),
    ]);
    set((s) => ({
      relayDashAgents: agentsRes.ok ? agentsRes.data : s.relayDashAgents,
      relayDashWorkspaces: workspacesRes.ok ? workspacesRes.data : s.relayDashWorkspaces,
      relayDashFeeds: feedsRes.ok
        ? Object.entries(feedsRes.data).map(([agent_id, feeds]) => ({ agent_id, feeds }))
        : s.relayDashFeeds,
      relayDashTopics: topicsRes.ok ? topicsRes.data : s.relayDashTopics,
    }));
  },

  handleRelayEvent: (event: RelayEvent) => {
    const now = Date.now();
    const logEntry = (msg: string): RelayEventLogEntry => ({
      ts: now,
      type: event.type,
      message: msg,
    });

    set((s) => {
      const nextLog = [
        logEntry(relayEventMessage(event)),
        ...s.relayDashEventLog.slice(0, MAX_RELAY_EVENT_LOG - 1),
      ];

      switch (event.type) {
        case 'agent_connected':
          return {
            relayDashAgents: [
              ...s.relayDashAgents.filter((a) => a.agent_id !== event.agent.agent_id),
              event.agent,
            ],
            relayDashEventLog: nextLog,
          };

        case 'agent_disconnected':
          return {
            relayDashAgents: s.relayDashAgents.filter((a) => a.agent_id !== event.agent_id),
            relayDashEventLog: nextLog,
          };

        case 'workspace_connected':
          return {
            relayDashWorkspaces: [
              ...s.relayDashWorkspaces.filter((w) => w.workspace_id !== event.workspace.workspace_id),
              event.workspace,
            ],
            relayDashEventLog: nextLog,
          };

        case 'workspace_disconnected':
          return {
            relayDashWorkspaces: s.relayDashWorkspaces.filter((w) => w.workspace_id !== event.workspace_id),
            relayDashEventLog: nextLog,
          };

        case 'workspace_heartbeat':
          return {
            relayDashWorkspaces: s.relayDashWorkspaces.map((w) =>
              w.workspace_id === event.workspace_id
                ? { ...w, last_heartbeat_ms: now, agents_count: event.agents_count }
                : w,
            ),
            relayDashEventLog: nextLog,
          };

        case 'feed_registered':
          return {
            relayDashFeeds: upsertFeed(s.relayDashFeeds, event.agent_id, event.feed),
            relayDashEventLog: nextLog,
          };

        case 'feed_unregistered':
          return {
            relayDashFeeds: removeFeed(s.relayDashFeeds, event.agent_id, event.feed_id),
            relayDashEventLog: nextLog,
          };

        default:
          return { relayDashEventLog: nextLog };
      }
    });
  },

  fetchFeedCatalog: async () => {
    try {
      const res = await api.get<{
        agents: Array<{
          agent_id: string; name: string; capabilities: string[];
          feed_count: number; online: boolean;
        }>;
        feeds: Array<{
          feed_id: string; topic: string; name: string; description: string;
          kind: string; rate: string; agent_id: string;
        }>;
        stats: { total_agents: number; total_feeds: number; messages_per_sec: number };
      }>('/api/feeds/catalog');
      if (res.ok) {
        const { agents, feeds } = res.data;
        set((s) => ({
          relayAgents: agents.map((a) => ({
            agentId: a.agent_id,
            name: a.name,
            capabilities: a.capabilities,
            feedCount: a.feed_count,
            connectedAtMs: Date.now(),
            online: a.online,
          })),
          relayFeeds: feeds.map((f) => {
            // Preserve sparkline + messageCount from existing state.
            const existing = s.relayFeeds.find((ef) => ef.feedId === f.feed_id);
            const agentName = agents.find((a) => a.agent_id === f.agent_id)?.name ?? f.agent_id;
            return {
              feedId: f.feed_id,
              topic: f.topic,
              name: f.name,
              description: f.description,
              kind: f.kind as RelayFeed['kind'],
              rate: f.rate,
              agentId: f.agent_id,
              agentName,
              lastValue: existing?.lastValue ?? null,
              lastUpdateMs: existing?.lastUpdateMs ?? null,
              messageCount: existing?.messageCount ?? 0,
              sparkline: existing?.sparkline ?? [],
              status: existing?.status ?? 'live',
            };
          }),
        }));
      }
    } catch (err) {
      console.warn('[DataHub] fetchFeedCatalog failed:', err);
    }
  },
}));
