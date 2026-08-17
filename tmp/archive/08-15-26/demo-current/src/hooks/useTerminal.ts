import { useEffect, useRef, useCallback, useState } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { rosedustTheme } from '../lib/rosedust-theme';
import { WS_URL } from '../lib/config';
import '@xterm/xterm/css/xterm.css';

/** Regex to strip ANSI escape sequences for text analysis */
const ANSI_RE = /\x1b\[[0-9;]*[A-Za-z]/g;

/** Shell prompt detection */
const PROMPT_RE = /[❯$%>#]\s*$/;

/**
 * Gate patterns matching actual roko output:
 *   "compile: pass", "compile: FAIL", "compile: FAIL — error: could not compile"
 *   "test: pass", "test: FAIL — test result: 1 failed"
 *   Also matches emoji-style: "✓ compile", "✕ test"
 */
const GATE_PATTERNS: { name: string; pass: RegExp; fail: RegExp }[] = [
  { name: 'compile', pass: /compile:\s*pass|[✓✔]\s*compile/i,  fail: /compile:\s*FAIL|[✕✖]\s*compile/i },
  { name: 'test',    pass: /test:\s*pass|[✓✔]\s*test/i,         fail: /test:\s*FAIL|[✕✖]\s*test|test result:\s*FAILED/i },
  { name: 'clippy',  pass: /clippy:\s*pass|[✓✔]\s*clippy/i,     fail: /clippy:\s*FAIL|[✕✖]\s*clippy/i },
  { name: 'diff',    pass: /diff:\s*pass|[✓✔]\s*diff/i,         fail: /diff:\s*FAIL|[✕✖]\s*diff/i },
];

/**
 * Cost detection matching actual roko output:
 *   "$0.0042", "$1.2500", "Total: $1.42", "cost: $0.05"
 */
const COST_RE = /\$(\d+\.\d{2,4})/;

/**
 * Token detection matching actual roko output:
 *   "input=2500, output=1200"
 *   "2500 in / 1200 out"
 *   "3700tok", "3700 tokens"
 */
const TOKEN_RE = /(?:input=(\d[\d,]*),\s*output=(\d[\d,]*))|(?:(\d[\d,]*)\s*in\s*\/\s*(\d[\d,]*)\s*out)|(?:(\d[\d,]*)\s*(?:tokens?|tok))/i;

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected';

export interface GateEvent {
  name: string;
  status: 'pass' | 'fail';
}

export interface TerminalHandle {
  /** Execute a command and wait for prompt to return (default 120s timeout) */
  execCmd: (cmd: string, timeoutMs?: number) => Promise<boolean>;
  /** Type a command character by character, then execute */
  typeCmd: (cmd: string) => Promise<boolean>;
  /** Send raw text to the terminal */
  sendRaw: (text: string) => void;
  /** Get the current output buffer (ANSI-stripped) */
  getOutputBuffer: () => string;
  /** Clear the output buffer */
  clearBuffer: () => void;
  /** Clear the terminal screen */
  clearTerminal: () => void;
  /** Wait for shell prompt to appear */
  waitForPrompt: (timeoutMs?: number) => Promise<boolean>;
}

interface UseTerminalOptions {
  sessionId?: string;
  onGate?: (event: GateEvent) => void;
  onCost?: (cost: number) => void;
  onTokens?: (tokens: number) => void;
  onLine?: (line: string) => void;
  onStatusChange?: (status: ConnectionStatus) => void;
}

interface UseTerminalReturn {
  /** Ref to attach to a container div */
  containerRef: React.RefCallback<HTMLDivElement>;
  /** Terminal handle for commanding the terminal */
  handle: TerminalHandle | null;
  /** Current connection status */
  status: ConnectionStatus;
  /** Session ID */
  sessionId: string;
}

const MAX_BUFFER = 60_000;
const TRIM_TO = 40_000;

export function useTerminal(options: UseTerminalOptions = {}): UseTerminalReturn {
  const {
    sessionId: customSessionId,
    onGate, onCost, onTokens, onLine, onStatusChange,
  } = options;

  const sessionIdRef = useRef(
    customSessionId ?? `t${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 5)}`
  );
  const [status, setStatus] = useState<ConnectionStatus>('disconnected');

  // Stable refs for callbacks
  const onGateRef = useRef(onGate);
  const onCostRef = useRef(onCost);
  const onTokensRef = useRef(onTokens);
  const onLineRef = useRef(onLine);
  const onStatusChangeRef = useRef(onStatusChange);
  onGateRef.current = onGate;
  onCostRef.current = onCost;
  onTokensRef.current = onTokens;
  onLineRef.current = onLine;
  onStatusChangeRef.current = onStatusChange;

  // Core refs
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const outBufRef = useRef('');
  const mountedRef = useRef(true);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const resizeObserverRef = useRef<ResizeObserver | null>(null);
  const containerElRef = useRef<HTMLDivElement | null>(null);
  const handleRef = useRef<TerminalHandle | null>(null);
  const [handleReady, setHandleReady] = useState(false);
  const decoderRef = useRef(new TextDecoder());

  // Update status helper
  const updateStatus = useCallback((s: ConnectionStatus) => {
    if (!mountedRef.current) return;
    setStatus(s);
    onStatusChangeRef.current?.(s);
  }, []);

  // Analyze output for gate events, cost, tokens
  const analyzeOutput = useCallback((text: string) => {
    const clean = text.replace(ANSI_RE, '');
    const lines = clean.split('\n');
    for (const line of lines) {
      if (!line.trim()) continue;
      onLineRef.current?.(line);

      for (const pat of GATE_PATTERNS) {
        if (pat.pass.test(line)) {
          onGateRef.current?.({ name: pat.name, status: 'pass' });
        } else if (pat.fail.test(line)) {
          onGateRef.current?.({ name: pat.name, status: 'fail' });
        }
      }

      const costMatch = line.match(COST_RE);
      if (costMatch) onCostRef.current?.(parseFloat(costMatch[1]));

      const tokenMatch = line.match(TOKEN_RE);
      if (tokenMatch) {
        let total = 0;
        if (tokenMatch[1] && tokenMatch[2]) {
          total = parseInt(tokenMatch[1].replace(/,/g, ''), 10) + parseInt(tokenMatch[2].replace(/,/g, ''), 10);
        } else if (tokenMatch[3] && tokenMatch[4]) {
          total = parseInt(tokenMatch[3].replace(/,/g, ''), 10) + parseInt(tokenMatch[4].replace(/,/g, ''), 10);
        } else if (tokenMatch[5]) {
          total = parseInt(tokenMatch[5].replace(/,/g, ''), 10);
        }
        if (total > 0) onTokensRef.current?.(total);
      }
    }
  }, []);

  // Connect WebSocket
  const connect = useCallback(() => {
    if (!mountedRef.current || !termRef.current) return;

    const id = sessionIdRef.current;
    const url = `${WS_URL}/ws/terminal/${id}`;
    updateStatus('connecting');

    const ws = new WebSocket(url);
    ws.binaryType = 'arraybuffer';
    wsRef.current = ws;

    ws.onopen = () => {
      if (!mountedRef.current) { ws.close(); return; }
      updateStatus('connected');

      // Send initial resize
      const term = termRef.current;
      if (term) {
        ws.send(JSON.stringify({ type: 'resize', cols: term.cols, rows: term.rows }));
      }
    };

    ws.onmessage = (ev) => {
      if (!mountedRef.current) return;
      const term = termRef.current;
      if (!term) return;

      if (ev.data instanceof ArrayBuffer) {
        const text = decoderRef.current.decode(ev.data);
        term.write(new Uint8Array(ev.data));

        // Append to output buffer
        outBufRef.current += text;
        if (outBufRef.current.length > MAX_BUFFER) {
          outBufRef.current = outBufRef.current.slice(-TRIM_TO);
        }

        analyzeOutput(text);
      } else if (typeof ev.data === 'string') {
        term.write(ev.data);
        outBufRef.current += ev.data;
        if (outBufRef.current.length > MAX_BUFFER) {
          outBufRef.current = outBufRef.current.slice(-TRIM_TO);
        }
        analyzeOutput(ev.data);
      }
    };

    ws.onclose = () => {
      if (!mountedRef.current) return;
      updateStatus('disconnected');
      // Auto-reconnect after 2s
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = setTimeout(() => {
        if (mountedRef.current) connect();
      }, 2000);
    };

    ws.onerror = () => {
      // Will trigger onclose
    };
  }, [updateStatus, analyzeOutput]);

  // Container ref callback — sets up xterm when container mounts
  const containerRef = useCallback((el: HTMLDivElement | null) => {
    // Cleanup previous
    if (containerElRef.current && !el) {
      resizeObserverRef.current?.disconnect();
      wsRef.current?.close();
      clearTimeout(reconnectTimerRef.current);
      termRef.current?.dispose();
      termRef.current = null;
      fitRef.current = null;
      containerElRef.current = null;
      handleRef.current = null;
      setHandleReady(false);
      return;
    }

    if (!el || containerElRef.current === el) return;
    containerElRef.current = el;

    // Create terminal
    const term = new Terminal({
      theme: rosedustTheme,
      fontFamily: "'JetBrains Mono', 'SF Mono', monospace",
      fontSize: 13,
      cursorBlink: true,
      allowProposedApi: true,
      scrollback: 5000,
    });

    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(el);

    // Initial fit
    requestAnimationFrame(() => {
      try { fit.fit(); } catch { /* container not ready */ }
    });

    termRef.current = term;
    fitRef.current = fit;

    // Handle user input → send to WebSocket
    term.onData((data) => {
      const ws = wsRef.current;
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(data);
      }
    });

    // Handle resize → send to WebSocket
    term.onResize(({ cols, rows }) => {
      const ws = wsRef.current;
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: 'resize', cols, rows }));
      }
    });

    // ResizeObserver for responsive fit
    const ro = new ResizeObserver(() => {
      try { fit.fit(); } catch { /* ignore */ }
    });
    ro.observe(el);
    resizeObserverRef.current = ro;

    // Build handle
    const handle: TerminalHandle = {
      execCmd: async (cmd: string, timeoutMs = 120_000): Promise<boolean> => {
        const ws = wsRef.current;
        if (!ws || ws.readyState !== WebSocket.OPEN) return false;

        outBufRef.current = '';
        ws.send(`${cmd}\r`);

        // Wait for shell prompt to return
        return handle.waitForPrompt(timeoutMs);
      },

      typeCmd: async (cmd: string): Promise<boolean> => {
        const ws = wsRef.current;
        if (!ws || ws.readyState !== WebSocket.OPEN) return false;

        for (const ch of cmd) {
          ws.send(ch);
          await new Promise(r => setTimeout(r, 12 + Math.random() * 6));
        }
        ws.send('\r');

        return handle.waitForPrompt(30_000);
      },

      sendRaw: (text: string) => {
        const ws = wsRef.current;
        if (ws && ws.readyState === WebSocket.OPEN) {
          ws.send(text);
        }
      },

      getOutputBuffer: () => outBufRef.current.replace(ANSI_RE, ''),

      clearBuffer: () => { outBufRef.current = ''; },

      clearTerminal: () => {
        termRef.current?.clear();
        outBufRef.current = '';
      },

      waitForPrompt: async (timeoutMs = 30_000): Promise<boolean> => {
        const deadline = Date.now() + timeoutMs;
        // Wait 500ms settling time
        await new Promise(r => setTimeout(r, 500));

        while (Date.now() < deadline) {
          const tail = outBufRef.current.replace(ANSI_RE, '').slice(-300);
          if (PROMPT_RE.test(tail)) {
            // Confirm stability
            await new Promise(r => setTimeout(r, 400));
            const tail2 = outBufRef.current.replace(ANSI_RE, '').slice(-300);
            if (PROMPT_RE.test(tail2)) return true;
          }
          await new Promise(r => setTimeout(r, 100));
        }
        return false;
      },
    };

    handleRef.current = handle;
    setHandleReady(true);

    // Connect WebSocket
    connect();
  }, [connect]);

  // Cleanup on unmount
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      clearTimeout(reconnectTimerRef.current);
      resizeObserverRef.current?.disconnect();
      wsRef.current?.close();
      termRef.current?.dispose();
    };
  }, []);

  return {
    containerRef,
    handle: handleReady ? handleRef.current : null,
    status,
    sessionId: sessionIdRef.current,
  };
}
