export const SERVE_URL = import.meta.env.VITE_SERVE_URL ?? 'http://localhost:6677';
export const WS_URL = SERVE_URL.replace(/^http/, 'ws');
