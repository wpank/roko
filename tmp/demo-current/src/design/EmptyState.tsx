import { type CSSProperties } from 'react';

interface EmptyStateProps {
  message: string;
  action?: string;
  hint?: string;
}

const containerStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  justifyContent: 'center',
  gap: 'var(--gap-sm)',
  padding: 'var(--gap-xl)',
  textAlign: 'center',
  fontFamily: 'var(--mono)',
  fontSize: '12px',
};

const messageStyle: CSSProperties = {
  color: 'var(--text-dim)',
};

const actionStyle: CSSProperties = {
  color: 'var(--text-soft)',
};

const hintStyle: CSSProperties = {
  color: 'var(--text-ghost)',
  fontSize: '11px',
  marginTop: 'var(--gap-xs)',
};

export function EmptyState({ message, action, hint }: EmptyStateProps) {
  return (
    <div style={containerStyle}>
      <div style={messageStyle}>{message}</div>
      {action && <div style={actionStyle}>{action}</div>}
      {hint && <div style={hintStyle}>{hint}</div>}
    </div>
  );
}
