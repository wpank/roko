import type { Signal } from './Led';

/** One 7px square per check, task or try: passed, failed, running (blinking), yours, or waiting. */
export function Pips({ signals, label }: { signals: readonly Signal[]; label?: string }) {
  return (
    <span className="rk-pips" role={label ? 'img' : undefined} aria-label={label} aria-hidden={label ? undefined : true}>
      {signals.map((s, i) => (
        <i key={i} data-state={s === 'waiting' || s === 'off' || s === 'still' ? undefined : s} />
      ))}
    </span>
  );
}
