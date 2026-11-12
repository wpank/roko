/**
 * Twenty segments on the glass, for spend against the cap: lit in blue while under 80% of it, orange from 80%,
 * green once the run passed.
 */
export function Meter({ lit, tone }: { lit: number; tone: 'live' | 'pass' | 'you' }) {
  return (
    <span className="rk-meter" data-tone={tone === 'pass' ? 'pass' : undefined} aria-hidden="true">
      {Array.from({ length: 20 }, (_, i) => (
        <i key={i} data-on={i < lit ? (tone === 'you' ? 'you' : '') : undefined} />
      ))}
    </span>
  );
}
