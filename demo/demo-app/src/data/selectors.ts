/**
 * DataHub selectors — thin hooks that read individual slices from the
 * centralised Zustand store.
 *
 * Components read shared state through these hooks rather than mounting
 * parallel context providers and duplicate REST/SSE owners.
 *
 * Implementation task: A1.6
 */

import { useDataHub } from '../app/DataHub';
import { useCallback, useMemo } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { api } from '../transport/api';
import {
  groupConfigModels,
  type RawConfigModels,
} from '../lib/config-models';

// ── Connection / health ─────────────────────────────────────────────

/** Server reachability status: 'connected' | 'checking' | 'disconnected'. */
export const useServerConnected = () =>
  useDataHub((s) => s.serverStatus === 'connected');

/** Raw server status enum. */
export const useServerStatus = () => useDataHub((s) => s.serverStatus);

/** DataHub health state plus an explicit singleton-client refresh action. */
export const useServerHealthController = () => {
  const { status, setStatus } = useDataHub(
    useShallow((s) => ({
      status: s.serverStatus,
      setStatus: s.setServerStatus,
    })),
  );
  const checkNow = useCallback(async () => {
    const snapshot = await api.probe(true);
    setStatus(snapshot.reachable ? 'connected' : 'disconnected');
  }, [setStatus]);
  return { status, checkNow };
};

/** SSE transport status. */
export const useSseStatus = () => useDataHub((s) => s.sseStatus);

/** WebSocket transport status. */
export const useWsStatus = () => useDataHub((s) => s.wsStatus);

/** All three transport statuses in one selector. */
export const useTransportStatus = () =>
  useDataHub(
    useShallow((s) => ({
      serverStatus: s.serverStatus,
      sseStatus: s.sseStatus,
      wsStatus: s.wsStatus,
    })),
  );

// ── Config ──────────────────────────────────────────────────────────

/** Full config blob (null until first fetch completes). */
export const useFullConfig = () => useDataHub((s) => s.config);

/** Default model string from config. */
export const useDefaultModel = () => useDataHub((s) => s.defaultModel);

/** Default backend string from config. */
export const useDefaultBackend = () => useDataHub((s) => s.defaultBackend);

/** Config slice: config + model + backend + actions. */
export const useConfigSlice = () =>
  useDataHub(
    useShallow((s) => ({
      config: s.config,
      defaultModel: s.defaultModel,
      defaultBackend: s.defaultBackend,
      fetchConfig: s.fetchConfig,
      updateConfig: s.updateConfig,
    })),
  );

/** Complete config read/write controller backed by the canonical DataHub. */
export const useConfigController = () => {
  const slice = useDataHub(
    useShallow((s) => ({
      config: s.config,
      defaultModel: s.defaultModel,
      defaultBackend: s.defaultBackend,
      isLive: s.serverStatus === 'connected',
      lastSaved: s.lastConfigSavedAt,
      updateModelConfig: s.updateModelConfig,
      updateConfig: s.updateConfig,
      refreshConfig: s.fetchConfig,
    })),
  );
  const providers = useMemo(
    () => groupConfigModels(
      slice.config?.models as RawConfigModels | undefined,
      slice.config?.providers as Record<string, { kind: string }> | undefined,
    ),
    [slice.config],
  );
  return {
    ...slice,
    fullConfig: slice.config ?? {},
    providers,
  };
};

// ── Workspace ───────────────────────────────────────────────────────

/** Server working directory (null until fetched). */
export const useServerWorkdir = () => useDataHub((s) => s.serverWorkdir);

/** Current workspace info (null until ensured). */
export const useWorkspaceInfo = () => useDataHub((s) => s.workspace);

/** Workspace actions: ensure, destroy, workdir. */
export const useWorkspaceSlice = () =>
  useDataHub(
    useShallow((s) => ({
      serverWorkdir: s.serverWorkdir,
      workspace: s.workspace,
      ensureWorkspace: s.ensureWorkspace,
      createWorkspace: s.createWorkspace,
      destroyWorkspace: s.destroyWorkspace,
      fetchServerWorkdir: s.fetchServerWorkdir,
    })),
  );

// ── Plan execution ──────────────────────────────────────────────────

/** Active plan ID (null when idle). */
export const useActivePlanId = () => useDataHub((s) => s.activePlanId);

/** Current phase label. */
export const useActivePhase = () => useDataHub((s) => s.activePhase);

/** Whether the active plan has completed. */
export const usePlanCompleted = () => useDataHub((s) => s.planCompleted);

/** Full plan execution slice. */
export const usePlanSlice = () =>
  useDataHub(
    useShallow((s) => ({
      activePlanId: s.activePlanId,
      activePhase: s.activePhase,
      planCompleted: s.planCompleted,
    })),
  );

// ── Agents ──────────────────────────────────────────────────────────

/** Agent list. */
export const useAgents = () => useDataHub((s) => s.agents);

/** Fetch agents action. */
export const useFetchAgents = () => useDataHub((s) => s.fetchAgents);

// ── Episodes / metrics ──────────────────────────────────────────────

/** Episode ring buffer. */
export const useEpisodes = () => useDataHub((s) => s.episodes);

/** Total cost across all inferences. */
export const useTotalCost = () => useDataHub((s) => s.totalCost);

/** Total tokens across all inferences. */
export const useTotalTokens = () => useDataHub((s) => s.totalTokens);

/** Recent inferences ring buffer. */
export const useRecentInferences = () =>
  useDataHub((s) => s.recentInferences);

/** Cost + token metrics bundle. */
export const useCostSlice = () =>
  useDataHub(
    useShallow((s) => ({
      totalCost: s.totalCost,
      totalTokens: s.totalTokens,
      recentInferences: s.recentInferences,
    })),
  );

// ── Bench ───────────────────────────────────────────────────────────

/** Bench run history. */
export const useBenchRuns = () => useDataHub((s) => s.benchRuns);

/** Bench suites. */
export const useBenchSuites = () => useDataHub((s) => s.benchSuites);

/** Bench models. */
export const useBenchModels = () => useDataHub((s) => s.benchModels);

/** Full bench slice with fetch actions. */
export const useBenchSlice = () =>
  useDataHub(
    useShallow((s) => ({
      benchRuns: s.benchRuns,
      benchSuites: s.benchSuites,
      benchModels: s.benchModels,
      fetchBenchRuns: s.fetchBenchRuns,
      fetchBenchSuites: s.fetchBenchSuites,
      fetchBenchModels: s.fetchBenchModels,
    })),
  );
