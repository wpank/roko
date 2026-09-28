import type { Alert, AlertAction } from '@/lib/alerts';

// Label map for each action kind.
const ACTION_LABELS: Record<AlertAction['kind'], string> = {
  'select-task': 'Show',
  retry: 'Retry',
  reconnect: 'Reconnect',
};

// CSS custom-property tokens for severity levels.
const SEVERITY_COLOR: Record<Alert['severity'], string> = {
  error: 'var(--state-failed)',
  warning: 'var(--state-accepted)',
};

export function AlertBand({
  alert,
  onAction,
  onDismiss,
}: {
  alert: Alert | null;
  onAction(action: AlertAction): void;
  onDismiss(key: string): void;
}) {
  // Render nothing when there is no active alert — keeps the row height at zero.
  if (alert === null) return null;

  const color = SEVERITY_COLOR[alert.severity];

  return (
    <div
      data-region="alert"
      role="status"
      style={{
        backgroundColor: color,
        color: 'var(--fg-on-state, #fff)',
        display: 'flex',
        alignItems: 'center',
        gap: '0.5rem',
        padding: '0.375rem 0.75rem',
        fontSize: '0.875rem',
        lineHeight: '1.25rem',
      }}
    >
      {/* Alert text */}
      <span style={{ flex: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
        {alert.text}
      </span>

      {/* One button per action */}
      {alert.actions.map((action, i) => (
        <button
          key={i}
          type="button"
          onClick={() => onAction(action)}
          style={{
            background: 'rgba(0,0,0,0.15)',
            border: '1px solid rgba(255,255,255,0.35)',
            borderRadius: '0.25rem',
            color: 'inherit',
            cursor: 'pointer',
            fontSize: 'inherit',
            padding: '0.125rem 0.5rem',
            whiteSpace: 'nowrap',
          }}
        >
          {ACTION_LABELS[action.kind]}
        </button>
      ))}

      {/* Dismiss control */}
      <button
        type="button"
        aria-label="Dismiss alert"
        onClick={() => onDismiss(alert.key)}
        style={{
          background: 'transparent',
          border: 'none',
          color: 'inherit',
          cursor: 'pointer',
          fontSize: '1rem',
          lineHeight: 1,
          padding: '0.125rem 0.25rem',
          opacity: 0.8,
        }}
      >
        ×
      </button>
    </div>
  );
}
