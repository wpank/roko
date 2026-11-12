import { Led } from './Led';
import type { Signal } from './Led';

/** A state, named in a word, with its light first: `● RUNNING`. One per run or task, beside its title. */
export function Pill({ signal, word }: { signal: Signal; word: string }) {
  return (
    <span className="rk-pill" data-state={signal}>
      <Led signal={signal} />
      {word}
    </span>
  );
}
