import { type CSSProperties } from 'react';
import { useTerminal, type ConnectionStatus, type GateEvent } from '../hooks/useTerminal';

interface TerminalPaneProps {
  sessionId?: string;
  label?: string;
  onGate?: (event: GateEvent) => void;
  onCost?: (cost: number) => void;
  onTokens?: (tokens: number) => void;
  onLine?: (line: string) => void;
  style?: CSSProperties;
}

const containerStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  background: '#060608',
  border: '1px solid var(--border)',
  boxShadow: 'var(--shadow-sm)',
  overflow: 'hidden',
  minHeight: 200,
};

const headerStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 'var(--gap-sm)',
  padding: '8px 12px',
  borderBottom: '1px solid var(--border-soft)',
  background: 'rgba(10, 8, 16, 0.9)',
  fontFamily: 'var(--mono)',
  fontSize: '11px',
};

const dotStyle = (status: ConnectionStatus): CSSProperties => ({
  width: 6,
  height: 6,
  borderRadius: '50%',
  background: status === 'connected' ? 'var(--success)' : status === 'connecting' ? 'var(--warning)' : 'var(--rose-dim)',
  boxShadow: status === 'connected' ? '0 0 6px rgba(122, 138, 120, 0.5)' : 'none',
  animation: status === 'connecting' ? 'pulse 1.5s ease-in-out infinite' : 'none',
  flexShrink: 0,
});

const bodyStyle: CSSProperties = {
  flex: 1,
  padding: 4,
  overflow: 'hidden',
};

export function TerminalPane({
  sessionId,
  label,
  onGate,
  onCost,
  onTokens,
  onLine,
  style,
}: TerminalPaneProps) {
  const { containerRef, status, sessionId: sid } = useTerminal({
    sessionId,
    onGate,
    onCost,
    onTokens,
    onLine,
  });

  const statusLabel = status === 'connected' ? 'connected' : status === 'connecting' ? 'connecting...' : 'disconnected';

  return (
    <div style={{ ...containerStyle, ...style }}>
      <div style={headerStyle}>
        <span style={dotStyle(status)} />
        <span style={{ color: 'var(--text-soft)' }}>{label ?? sid}</span>
        <span style={{ marginLeft: 'auto', color: 'var(--text-ghost)', fontSize: 10 }}>
          {statusLabel}
        </span>
      </div>
      <div style={bodyStyle} ref={containerRef} />
    </div>
  );
}
