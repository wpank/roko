export { RokoApi, api } from './api';
export type { ApiError, ApiResult, HealthSnapshot } from './api';

export { SseAdapter } from './sse';
export type { SseAdapterConfig, SseStatus } from './sse';

export { WsAdapter } from './ws';
export type {
  WsAdapterConfig,
  WsFrame,
  WsOutbound,
  WsPingMsg,
  WsResizeMsg,
  WsStatus,
  WsSubscribeMsg,
  WsUnsubscribeMsg,
} from './ws';

export {
  DASHBOARD_EVENT_TYPES,
  parseDashboardEvent,
  parseDashboardGap,
} from './types';
export type {
  DashboardEvent,
  DashboardGapPayload,
  DashboardSnapshot,
} from './types';
