import type { Alert, AlertAction } from '@/lib/alerts';

// Label map for each action kind.
const ACTION_LABELS: Record<AlertAction['kind'], string> = {
  'select-task': 'Show',
  retry: 'Retry',
  reconnect: 'Reconnect',
};

// Glyph for each severity level.
const SEVERITY_GLYPHS: Record<Alert['severity'], string> = {
  error: '✗',
  warning: '⚠',
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

  return (
    <div
      data-region="alert"
      data-severity={alert.severity}
      role="status"
      className="rd-alert"
    >
      <span className="rd-alert__glyph" aria-hidden="true">
        {SEVERITY_GLYPHS[alert.severity]}
      </span>

      <span className="rd-alert__text" title={alert.text}>
        {alert.text}
      </span>

      {alert.actions.map((action, i) => (
        <button
          key={i}
          type="button"
          className="rd-alert__action"
          onClick={() => onAction(action)}
        >
          {ACTION_LABELS[action.kind]}
        </button>
      ))}

      <button
        type="button"
        className="rd-alert__dismiss"
        aria-label="Dismiss alert"
        onClick={() => onDismiss(alert.key)}
      >
        ×
      </button>
    </div>
  );
}
