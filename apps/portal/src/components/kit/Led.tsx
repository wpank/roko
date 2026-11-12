/**
 * What an LED or a signal word shows: roko working (live), passed, your turn (you), failed, a settled state that is
 * no signal (still: stopped, not checked), not started (waiting), or nothing going on (off).
 */
export type Signal = 'live' | 'pass' | 'you' | 'fail' | 'still' | 'waiting' | 'off';

/** An 8px light. Live blinks slowly and you fast; still is a steady grey; waiting and off are hollow. */
export function Led({ signal, label }: { signal: Signal; label?: string }) {
  return (
    <span
      className="rk-led"
      data-state={signal}
      role={label ? 'img' : undefined}
      aria-label={label}
      aria-hidden={label ? undefined : true}
    />
  );
}
