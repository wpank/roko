'use client';

/**
 * Sequencer — a plan drawn as a step sequencer (the design system's §4.7): columns are steps, tasks in a step run
 * at the same time, each task is a pad with its tries as pips, and lines run from a task to those that wait on it.
 * A click on a pad opens that task.
 */

import { useLayoutEffect, useRef, useState } from 'react';
import { Led } from '@/components/kit/Led';
import type { Signal } from '@/components/kit/Led';
import { Pips } from '@/components/kit/Pips';
import type { Edge, Pad, PadState, Step } from '@/lib/runCard';

const LED: Record<PadState, Signal> = {
  waiting: 'waiting',
  live: 'live',
  pass: 'pass',
  fail: 'fail',
  you: 'you',
  still: 'still',
  skipped: 'off',
};

interface Line {
  d: string;
  state: Edge['state'];
}

export function Sequencer({
  steps,
  edges,
  selectedId,
  onSelect,
}: {
  steps: readonly Step[];
  edges: readonly Edge[];
  selectedId: string | null;
  onSelect(id: string): void;
}) {
  const box = useRef<HTMLDivElement>(null);
  const [lines, setLines] = useState<{ width: number; height: number; paths: Line[] }>({ width: 0, height: 0, paths: [] });

  // Lines follow the pads: drawn after layout, and again whenever the sequencer changes size.
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const draw = () => {
      const frame = el.getBoundingClientRect();
      const at = (id: string) => el.querySelector(`[data-pad=${JSON.stringify(id)}]`)?.getBoundingClientRect();
      const paths: Line[] = [];
      for (const edge of edges) {
        const a = at(edge.from);
        const b = at(edge.to);
        if (!a || !b) continue;
        const x1 = a.right - frame.left + el.scrollLeft;
        const y1 = a.top + a.height / 2 - frame.top;
        const x2 = b.left - frame.left + el.scrollLeft;
        const y2 = b.top + b.height / 2 - frame.top;
        const mx = (x1 + x2) / 2;
        paths.push({ d: `M${x1} ${y1} C${mx} ${y1} ${mx} ${y2} ${x2} ${y2}`, state: edge.state });
      }
      setLines({ width: el.scrollWidth, height: el.scrollHeight, paths });
    };
    draw();
    if (typeof ResizeObserver === 'undefined') return;
    const watch = new ResizeObserver(draw);
    watch.observe(el);
    return () => watch.disconnect();
  }, [steps, edges]);

  return (
    <div className="rk-module pt-seq" data-region="sequencer">
      <div className="rk-seq" ref={box}>
        <div className="rk-seq-grid">
          {steps.map((step) => (
            <div key={step.n} className="rk-seq-col" data-step-col={step.n}>
              <div className="rk-seq-head">
                <span className="rk-step" data-state={step.state ?? undefined}>
                  {step.n}
                </span>
                <span className="rk-label">step</span>
              </div>
              {step.pads.map((pad) => (
                <PadKey key={pad.id} pad={pad} selected={pad.id === selectedId} onSelect={onSelect} />
              ))}
            </div>
          ))}
        </div>
        <svg className="rk-seq-edges" width={lines.width} height={lines.height} aria-hidden="true">
          {lines.paths.map((line, i) => (
            <path key={i} d={line.d} data-state={line.state ?? undefined} />
          ))}
        </svg>
      </div>
    </div>
  );
}

function PadKey({ pad, selected, onSelect }: { pad: Pad; selected: boolean; onSelect(id: string): void }) {
  return (
    <button
      type="button"
      className="rk-pad"
      data-pad={pad.id}
      data-task-row={pad.id}
      data-state={pad.state}
      data-problem={pad.problem ?? undefined}
      data-selected={selected ? '' : undefined}
      aria-pressed={selected}
      aria-label={`Task ${pad.n.replace(/^0/, '')}: ${pad.title}, ${pad.foot}`}
      onClick={() => onSelect(pad.id)}
    >
      <span className="rk-pad-top">
        <span className="rk-pad-n">{pad.n}</span>
        <Led signal={LED[pad.state]} />
      </span>
      <span className="rk-pad-title">{pad.title}</span>
      <span className="rk-pad-foot">
        {pad.tries.length > 0 ? <Pips signals={pad.tries} /> : <span />}
        <span>{pad.foot}</span>
      </span>
    </button>
  );
}
