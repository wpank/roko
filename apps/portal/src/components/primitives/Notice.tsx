import type { ReactNode } from 'react';

/**
 * Notice — an inline informational callout.
 *
 * Renders a `role="note"` box with a ⓘ glyph. Used wherever the server
 * cannot fulfil a request (kind='unsupported') or to surface any other
 * non-alert informational message (kind='info').
 *
 * T02 provides the .rd-notice / .rd-notice__glyph / .rd-notice__text styles.
 * T05 uses this component at the call sites that previously showed ad-hoc
 * "not supported" text.
 */
export function Notice({
  kind = 'unsupported',
  children,
}: {
  kind?: 'unsupported' | 'info';
  children: ReactNode;
}) {
  return (
    <div data-notice={kind} role="note" className="rd-notice">
      <span className="rd-notice__glyph" aria-hidden="true">
        ⓘ
      </span>
      <span className="rd-notice__text">{children}</span>
    </div>
  );
}
