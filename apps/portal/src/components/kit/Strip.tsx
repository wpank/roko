import type { Signal } from './Led';

/** The most squares a strip draws; the rest are counted beside it. */
export const STRIP_MAX = 16;

/**
 * A plan at list width: one square per task, coloured by its state (passed, running, failed, your turn, waiting).
 */
export function Strip({ squares, label }: { squares: readonly Signal[]; label: string }) {
  const shown = squares.slice(0, STRIP_MAX);
  const more = squares.length - shown.length;
  return (
    <span className="pt-strip" role="img" aria-label={label}>
      <span className="rk-strip">
        <span className="rk-strip-step">
          {shown.map((s, i) => (
            <i key={i} data-state={s === 'waiting' || s === 'off' || s === 'still' ? undefined : s} />
          ))}
        </span>
      </span>
      {more > 0 && <span className="pt-plan-more">+{more}</span>}
    </span>
  );
}
