import type { Alert, AlertAction } from '@/lib/alerts';
import { Key } from '@/components/kit/Key';
import { Led } from '@/components/kit/Led';
import type { Signal } from '@/components/kit/Led';

/** The words on each action's key. `retry` resumes the run: only what is left runs. */
const ACTION_LABELS: Record<AlertAction['kind'], string> = {
  'select-task': 'show',
  retry: 'resume',
  reconnect: 'reconnect',
};

/** A failure is red, a warning orange (it is yours to act on), a notice a plain grey light. */
const SIGNALS: Record<Alert['severity'], Signal> = {
  error: 'fail',
  warning: 'you',
  info: 'still',
};

/** One sentence under the top bar, at most one at a time, with its keys. Nothing when there is no alert. */
export function AlertBand({
  alert,
  onAction,
  onDismiss,
}: {
  alert: Alert | null;
  onAction(action: AlertAction): void;
  onDismiss(key: string): void;
}) {
  if (alert === null) return null;

  return (
    <div data-region="alert" data-severity={alert.severity} role="status" className="pt-notice">
      <Led signal={SIGNALS[alert.severity]} label={alert.severity} />
      <span className="pt-notice-text" title={alert.text}>
        {alert.text}
      </span>
      {alert.actions.map((action, i) => (
        <Key key={i} size="s" data-alert-action={action.kind} onClick={() => onAction(action)}>
          {ACTION_LABELS[action.kind]}
        </Key>
      ))}
      <Key size="s" aria-label="Dismiss this notice" onClick={() => onDismiss(alert.key)}>
        close
      </Key>
    </div>
  );
}
