import { DotMatrix } from '@/components/kit/DotMatrix';
import { Meter } from '@/components/kit/Meter';
import { Pips } from '@/components/kit/Pips';
import type { Card, Reading } from '@/lib/runCard';

/** The glass: a run's four readings, always in this order, in dot-matrix whose tone is the state. */
export function Readout({ readings }: { readings: Card['readings'] }) {
  return (
    <div className="rk-glass rk-cells pt-readout" role="group" aria-label="Readings">
      <Cell name="tasks" reading={readings.tasks} />
      <Cell name="time" reading={readings.time} />
      <Cell name="spent" reading={readings.spent} />
      <Cell name="checks" reading={readings.checks} />
    </div>
  );
}

function Cell({ name, reading }: { name: string; reading: Reading }) {
  return (
    <div className="rk-cell" data-reading={name}>
      <span className="rk-cell-label">{name}</span>
      <DotMatrix text={reading.text} label={`${name}: ${reading.label}`} size="L" tone={reading.tone} />
      {reading.pips && reading.pips.length > 0 && <Pips signals={reading.pips} />}
      {reading.meter && <Meter lit={reading.meter.lit} tone={reading.meter.tone} />}
      {reading.note && <span className="rk-cell-note">{reading.note}</span>}
    </div>
  );
}
