import type { ReactNode } from 'react';

/**
 * Notice — an inline informational callout.
 *
 * Renders a `role="note"` box with a ⓘ glyph. Used wherever the server
 * cannot fulfil a request (kind='unsupported'), to surface a failed request
 * (kind='error'; T01 draws its rule and glyph in the failed red), or for any
 * other non-alert informational message (kind='info').
 *
 * Pass `onClose` to render a compact Close button inside the notice row.
 * Without it, no close affordance is rendered.
 *
 * T02 provides the .rd-notice / .rd-notice__glyph / .rd-notice__text styles.
 * T05 uses this component at the call sites that previously showed ad-hoc
 * "not supported" text.
 * T06 adds kind='error' and the inline Close button.
 */
export function Notice({
  kind = 'unsupported',
  onClose,
  children,
}: {
  kind?: 'unsupported' | 'info' | 'error';
  onClose?: () => void;
  children: ReactNode;
}) {
  return (
    <div data-notice={kind} role="note" className="rd-notice">
      <span className="rd-notice__glyph" aria-hidden="true">
        ⓘ
      </span>
      <span className="rd-notice__text">{children}</span>
      {onClose && (
        <button type="button" className="rd-notice__close" onClick={onClose}>
          Close
        </button>
      )}
    </div>
  );
}
