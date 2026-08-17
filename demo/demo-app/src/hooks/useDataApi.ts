import { useCallback, useMemo } from 'react';
import { useServerConnected } from '../data/selectors';
import { api, type ApiResult } from '../transport/api';

function unwrap<T>(result: ApiResult<T>): T {
  if (result.ok) return result.data;
  const { status, statusText } = result.error;
  throw new Error(`${status} ${statusText}`);
}

/**
 * React adapter for the canonical transport client and DataHub connectivity.
 * It preserves the throwing data-return API expected by page-level loaders.
 */
export function useDataApi() {
  const isLive = useServerConnected();
  const get = useCallback(
    async <T = unknown>(path: string): Promise<T> => unwrap(await api.get<T>(path)),
    [],
  );
  const post = useCallback(
    async <T = unknown>(path: string, body?: unknown): Promise<T> =>
      unwrap(await api.post<T>(path, body)),
    [],
  );
  const put = useCallback(
    async <T = unknown>(path: string, body?: unknown): Promise<T> =>
      unwrap(await api.put<T>(path, body)),
    [],
  );

  return useMemo(
    () => ({ get, post, put, baseUrl: api.baseUrl, isLive }),
    [get, post, put, isLive],
  );
}
