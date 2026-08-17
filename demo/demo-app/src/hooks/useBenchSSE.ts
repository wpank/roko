import { useCallback, useEffect, useMemo, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { useDataHub } from '../app/DataHub';

interface UseBenchSSEOptions {
  benchId?: string;
  enabled?: boolean;
}

/**
 * Selects bench events from the app-wide `/api/bench/events` connection.
 * The transport is owned by bootstrap; mounting this hook never opens another
 * EventSource. Each caller keeps its own clear cursor, so clearing one bench
 * view does not discard events needed by another view.
 */
export function useBenchSSE({ benchId, enabled = true }: UseBenchSSEOptions = {}) {
  const { status, sequence, entries } = useDataHub(
    useShallow((state) => ({
      status: state.benchSseStatus,
      sequence: state.benchEventSequence,
      entries: state.benchEvents,
    })),
  );
  const [afterSequence, setAfterSequence] = useState(
    () => useDataHub.getState().benchEventSequence,
  );

  // A disabled hook mirrors the old unmounted connection: events received
  // while disabled are not replayed when the caller enables it later.
  useEffect(() => {
    if (!enabled) setAfterSequence(sequence);
  }, [enabled, sequence]);

  const events = useMemo(() => {
    if (!enabled) return [];
    return entries
      .filter(({ sequence: eventSequence, event }) => {
        if (eventSequence <= afterSequence) return false;
        const eventBenchId = 'bench_id' in event ? event.bench_id : undefined;
        return !benchId || !eventBenchId || eventBenchId === benchId;
      })
      .map(({ event }) => event);
  }, [afterSequence, benchId, enabled, entries]);

  const clear = useCallback(() => {
    setAfterSequence(sequence);
  }, [sequence]);

  return {
    connected: enabled && status === 'connected',
    lastEvent: events[events.length - 1] ?? null,
    events,
    clear,
  };
}
