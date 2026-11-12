import { DotMatrix } from './DotMatrix';

/** roko's wordmark: ROKO in dot-matrix on a small glass. */
export function Wordmark() {
  return (
    <span className="rk-wordmark">
      <DotMatrix text="ROKO" label="roko" size="S" />
    </span>
  );
}
