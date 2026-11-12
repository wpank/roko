'use client';

import { useEffect, useRef } from 'react';
import { dotReading } from './dots';
import type { DotSize } from './dots';

/** A reading's tone is the state of what it reads: a running run's time is live, a passed run's checks pass. */
export type Tone = 'on' | 'live' | 'pass' | 'you' | 'fail' | 'dim';

/**
 * A reading on the glass, in roko's dot-matrix face. A glyph that changed since the last render rolls in; the first
 * render is still. Screen readers get `label`, the plain value ("4 minutes 5 seconds"), never the dots.
 */
export function DotMatrix({
  text,
  label,
  size = 'M',
  tone = 'on',
}: {
  text: string;
  label: string;
  size?: DotSize;
  tone?: Tone;
}) {
  const shown = useRef<string | null>(null);
  const reading = dotReading(text, size, shown.current);
  useEffect(() => {
    shown.current = text;
  }, [text]);

  return (
    <span className="pt-dots" data-tone={tone === 'on' ? undefined : tone}>
      <svg
        className="rk-dots"
        role="img"
        aria-label={label}
        width={reading.width}
        height={reading.height}
        viewBox={`0 0 ${reading.width} ${reading.height}`}
      >
        <path className="rk-dots-off" strokeWidth={reading.dot} d={reading.grid} />
        {reading.lit.map((g, i) => (
          <path
            key={`${i}:${text}`}
            className="rk-dots-on"
            strokeWidth={reading.dot}
            data-roll={g.roll ? 'true' : undefined}
            d={g.d}
          />
        ))}
      </svg>
    </span>
  );
}
