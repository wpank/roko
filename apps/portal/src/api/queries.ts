/**
 * Roko Portal — React Query hooks
 *
 * All network I/O that the portal views use lives here.  Every hook delegates
 * to `api` from `@/api/client` and is typed with the wire contracts from
 * `@/api/contracts`.
 *
 * Conventions:
 *  - IDs are encoded with `encodeURIComponent` before interpolation.
 *  - Mutations that change a plan (save, generate, revise) invalidate the
 *    plan list, plan tasks, plan source and validation caches.
 *  - Run / cancel invalidate the plan list only.
 *  - No polling intervals; no optimistic updates.
 *  - Errors propagate as `ApiError`; callers read `.body` for detail.
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api, ApiError } from '@/api/client';
import type {
  WirePlanSummary,
  WirePlanTasks,
  WirePlanSource,
  WireValidation,
  WireStatus,
  WireSourceSaved,
  WireAccepted,
  WireOperation,
} from '@/api/contracts';

// ---------------------------------------------------------------------------
// Query keys
// ---------------------------------------------------------------------------

export const queryKeys = {
  plans: ['plans'] as const,
  planTasks: (id: string) => ['plans', id, 'tasks'] as const,
  planSource: (id: string) => ['plans', id, 'source'] as const,
  validation: (id: string) => ['plans', id, 'validation'] as const,
  workspace: ['workspace'] as const,
};

// ---------------------------------------------------------------------------
// Workspace
// ---------------------------------------------------------------------------

export interface WorkspaceInfo {
  name: string;
  path: string;
  branch: string | null;
}

function basename(path: string): string {
  return path.replace(/\\/g, '/').split('/').filter(Boolean).pop() ?? path;
}

export function useWorkspace() {
  return useQuery<WorkspaceInfo>({
    queryKey: queryKeys.workspace,
    queryFn: async () => {
      const status = await api.get<WireStatus>('/api/status');
      return {
        name: basename(status.workdir),
        path: status.workdir,
        branch: status.git_branch ?? null,
      };
    },
  });
}

// ---------------------------------------------------------------------------
// Plans
// ---------------------------------------------------------------------------

export function usePlans() {
  return useQuery<WirePlanSummary[]>({
    queryKey: queryKeys.plans,
    queryFn: () => api.get<WirePlanSummary[]>('/api/plans'),
  });
}

export function usePlanTasks(id: string | undefined) {
  return useQuery<WirePlanTasks>({
    queryKey: queryKeys.planTasks(id ?? ''),
    queryFn: () =>
      api.get<WirePlanTasks>(`/api/plans/${encodeURIComponent(id!)}/tasks`),
    enabled: Boolean(id),
  });
}

export function usePlanSource(id: string | undefined, enabled = true) {
  return useQuery<WirePlanSource>({
    queryKey: queryKeys.planSource(id ?? ''),
    queryFn: () =>
      api.get<WirePlanSource>(`/api/plans/${encodeURIComponent(id!)}/source`),
    enabled: Boolean(id) && enabled,
  });
}

export function useValidation(id: string | undefined) {
  return useQuery<WireValidation>({
    queryKey: queryKeys.validation(id ?? ''),
    // No body: validate the saved file (ask P-3). Servers before the fix
    // answered `{}` with 400, since they required `toml` in any body.
    queryFn: () =>
      api.post<WireValidation>(`/api/plans/${encodeURIComponent(id!)}/validate`),
    enabled: Boolean(id),
  });
}

// ---------------------------------------------------------------------------
// Plan mutations — save, generate, revise
// ---------------------------------------------------------------------------

export function useSavePlanSource() {
  const queryClient = useQueryClient();
  return useMutation<WireSourceSaved, ApiError, { id: string; toml: string }>({
    mutationFn: ({ id, toml }) =>
      api.put<WireSourceSaved>(
        `/api/plans/${encodeURIComponent(id)}/source`,
        { toml },
      ),
    onSuccess: (_data, { id }) => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.plans });
      void queryClient.invalidateQueries({ queryKey: queryKeys.planTasks(id) });
      void queryClient.invalidateQueries({ queryKey: queryKeys.planSource(id) });
      void queryClient.invalidateQueries({
        queryKey: queryKeys.validation(id),
      });
    },
  });
}

export function useGeneratePlan() {
  const queryClient = useQueryClient();
  return useMutation<WireAccepted, ApiError, { prompt: string }>({
    mutationFn: ({ prompt }) =>
      api.post<WireAccepted>('/api/plans/generate', { prompt }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.plans });
    },
  });
}

export function useRevisePlan() {
  const queryClient = useQueryClient();
  return useMutation<WireAccepted, ApiError, { id: string; feedback: string }>({
    mutationFn: ({ id, feedback }) =>
      api.post<WireAccepted>(
        `/api/plans/${encodeURIComponent(id)}/revise`,
        { feedback },
      ),
    onSuccess: (_data, { id }) => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.plans });
      void queryClient.invalidateQueries({ queryKey: queryKeys.planTasks(id) });
      void queryClient.invalidateQueries({ queryKey: queryKeys.planSource(id) });
      void queryClient.invalidateQueries({
        queryKey: queryKeys.validation(id),
      });
    },
  });
}

// ---------------------------------------------------------------------------
// Run control mutations
// ---------------------------------------------------------------------------

export function useRunPlan() {
  const queryClient = useQueryClient();
  return useMutation<WireAccepted, ApiError, { id: string; resume?: boolean }>({
    mutationFn: ({ id, resume }) =>
      api.post<WireAccepted>(
        `/api/plans/${encodeURIComponent(id)}/execute`,
        resume ? { resume: true } : undefined,
      ),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.plans });
    },
  });
}

export function useRunPlans() {
  const queryClient = useQueryClient();
  return useMutation<WireAccepted, ApiError, { plans?: string[] }>({
    mutationFn: ({ plans }) =>
      api.post<WireAccepted>('/api/plans/execute', plans ? { plans } : {}),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.plans });
    },
  });
}

export function useCancelPlan() {
  const queryClient = useQueryClient();
  return useMutation<unknown, ApiError, { id: string }>({
    mutationFn: ({ id }) =>
      api.post<unknown>(`/api/plans/${encodeURIComponent(id)}/cancel`),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.plans });
    },
  });
}

// ---------------------------------------------------------------------------
// Plain async helpers (not React Query hooks)
// ---------------------------------------------------------------------------

/**
 * Returns `true` when a plan with the given id exists (200 OK), `false` on 404.
 * Used to poll until a generated plan lands (the accepted response names it).
 * Any error other than 404 propagates.
 */
export async function planExists(id: string): Promise<boolean> {
  try {
    await api.get<unknown>(`/api/plans/${encodeURIComponent(id)}`);
    return true;
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) return false;
    throw err;
  }
}

/**
 * Fetches an async operation by id.  Returns `null` on 404 (operation not yet
 * written or already GC'd).  Any other error propagates.
 */
export async function fetchOperation(id: string): Promise<WireOperation | null> {
  try {
    return await api.get<WireOperation>(
      `/api/operations/${encodeURIComponent(id)}`,
    );
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) return null;
    throw err;
  }
}
