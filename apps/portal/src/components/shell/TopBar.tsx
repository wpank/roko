'use client';

/**
 * TopBar — roko's wordmark, the project and its branch, the status chip, and the day and night switch.
 *
 * The status chip says the one most urgent thing (lib/statusChip.ts); a click opens the plan it is about. Sets
 * document.title to "roko · <project>".
 */

import { useEffect } from 'react';
import { useWorkspace } from '@/api/queries';
import { DotMatrix } from '@/components/kit/DotMatrix';
import { Led } from '@/components/kit/Led';
import { ThemeSwitch } from '@/components/kit/ThemeSwitch';
import { Wordmark } from '@/components/kit/Wordmark';
import type { RunStarts } from '@/lib/runReadings';
import { statusChip } from '@/lib/statusChip';
import type { SeenFailures } from '@/lib/statusChip';
import { useNow } from '@/lib/useNow';
import { useDashboardStore } from '@/stores/dashboard';

export function TopBar({
  seenFailures,
  runStarts,
  onOpenPlan,
}: {
  seenFailures: SeenFailures;
  /** When roko accepted each run, which the chip's clock counts from, as the run's TIME does. */
  runStarts?: RunStarts;
  onOpenPlan(planId: string): void;
}) {
  const { data: workspace } = useWorkspace();
  const run = useDashboardStore((s) => s.run);
  const connection = useDashboardStore((s) => s.connection);
  const running = Object.values(run.plans).some((p) => p.phase === 'running');
  const nowMs = useNow(running);
  const chip = statusChip(run, connection, { nowMs, seenFailures, runStarts });

  const name = workspace?.name;
  useEffect(() => {
    if (name) document.title = `roko · ${name}`;
  }, [name]);

  return (
    <header data-region="header" className="pt-top">
      <Wordmark />
      <span className="pt-project">
        <span data-slot="workspace" className="pt-project-name">
          {name}
        </span>
        {workspace?.branch && <span className="pt-project-branch pt-hide-narrow">{workspace.branch}</span>}
      </span>
      <span className="pt-spacer" />
      <button
        type="button"
        className="rk-chip"
        data-slot={chip.running ? 'run' : 'status'}
        data-signal={chip.signal}
        disabled={chip.planId === null}
        title={chip.planId !== null ? 'Open this plan' : undefined}
        onClick={() => {
          if (chip.planId !== null) onOpenPlan(chip.planId);
        }}
      >
        <span className="rk-chip-glass">
          <Led signal={chip.signal} />
          {chip.reading && (
            <DotMatrix text={chip.reading.text} label={chip.reading.label} size="XS" tone="live" />
          )}
        </span>
        <span className="rk-chip-word">{chip.word}</span>
      </button>
      <ThemeSwitch />
    </header>
  );
}
