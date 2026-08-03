/**
 * EventStreamContext — retained as a re-export shim.
 *
 * The SSE connection is now owned entirely by app/bootstrap.ts.
 * All consumers have been migrated to:
 *   - `useServerEventSubscription` (hooks/useEventStream.ts)
 *   - `useServerConnected`         (hooks/useEventStream.ts)
 *   - `subscribeServerEvents`      (app/bootstrap.ts)
 *
 * This file re-exports those hooks for any import path that still points
 * here during a transition window. The legacy manager/provider/context
 * constructs have been removed.
 */

export {
  useServerEventSubscription,
  useServerConnected,
  useEventStreamSubscription,
} from '../hooks/useEventStream';

export { subscribeServerEvents } from '../app/bootstrap';
