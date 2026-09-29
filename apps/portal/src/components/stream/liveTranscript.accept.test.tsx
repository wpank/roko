// @vitest-environment jsdom
/**
 * Acceptance: the transcript shows each live tool step as it happens — glyph,
 * tool, target and a dim `live` marker — frames unscreened content with a
 * label, and gives way to the screened transcript at the end of the turn.
 * Copied verbatim from plans/portal-programme/08c-portal-live-steps/accept/.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render } from '@testing-library/react';
import type { WireDashboardEvent } from '@/api/contracts';
import { Transcript } from '@/components/stream/Transcript';
import { taskKey } from '@/lib/runState';
import { STREAM_RECORD_PREFIX } from '@/lib/streamRecord';
import { foldEvents, hasMissingValue, textOf } from '@/test/dom';

afterEach(() => cleanup());

const KEY = taskKey('hello', 'T01');
const frame = (o: Record<string, unknown>) => STREAM_RECORD_PREFIX + JSON.stringify(o);
const ENV = { agent_id: 'hello/T01', plan_id: 'hello', task_id: 'T01', attempt: 0 };
const out = (content: string): WireDashboardEvent => ({
  type: 'agent_output',
  agent_id: 'a1',
  plan_id: 'hello',
  task_id: 'T01',
  content,
});
const step = (toolId: string, tool: string, target: string) =>
  out(frame({ kind: 'tool_start', live: true, ...ENV, payload: { tool_id: toolId, tool, target } }));
const unscreened = (kind: string, payload: Record<string, unknown>) =>
  out(frame({ kind, live: true, screened: false, ...ENV, payload }));
const screened = (kind: string, payload: Record<string, unknown>) => out(frame({ kind, ...ENV, payload }));

const START: WireDashboardEvent[] = [
  { type: 'plan_started', plan_id: 'hello', tasks_total: 1 },
  { type: 'task_started', plan_id: 'hello', task_id: 'T01', phase: 'implement' },
  { type: 'agent_spawned', agent_id: 'a1', plan_id: 'hello', task_id: 'T01', role: 'implementer' },
];

function renderTranscript(events: WireDashboardEvent[], working: boolean) {
  const run = foldEvents([...START, ...events]);
  return render(
    <Transcript transcript={run.transcripts[KEY]} working={working ? { sinceMs: Date.now() - 5_000 } : null} />,
  );
}

const region = () => document.querySelector('[data-region="transcript"]');

describe('live tool steps', () => {
  it('show glyph, tool, target and a live marker while the agent works', () => {
    renderTranscript([step('toolu_1', 'Write', 'apps/x.ts')], true);
    const row = document.querySelector('[data-step="toolu_1"]');
    expect(row).not.toBeNull();
    expect(textOf(row)).toContain('✎');
    expect(textOf(row)).toContain('Write');
    expect(textOf(row?.querySelector('[data-target]') ?? null)).toBe('apps/x.ts');
    expect(textOf(row?.querySelector('[data-live]') ?? null)).toBe('live');
  });

  it('stay as history without the live marker once the agent stops', () => {
    renderTranscript([step('toolu_1', 'Write', 'apps/x.ts'), step('toolu_2', 'Bash', 'cargo test')], false);
    expect(document.querySelectorAll('[data-step]')).toHaveLength(2);
    expect(document.querySelector('[data-live]')).toBeNull();
    expect(textOf(document.querySelector('[data-step="toolu_2"]'))).toContain('⚙');
  });

  it('show no target when the step has none, and no missing value', () => {
    renderTranscript([out(frame({ kind: 'tool_start', live: true, ...ENV, payload: { tool_id: 't', tool: 'Bash', target: '' } }))], true);
    const row = document.querySelector('[data-step="t"]');
    expect(row?.querySelector('[data-target]')).toBeNull();
    expect(hasMissingValue(textOf(region()))).toBe(false);
  });
});

describe('unscreened content', () => {
  it('is framed and labelled while it shows', () => {
    renderTranscript([unscreened('text', { text: 'Writing the file now' })], true);
    const framed = document.querySelector('[data-unscreened]');
    expect(framed).not.toBeNull();
    expect(textOf(framed)).toContain('Writing the file now');
    expect(textOf(framed?.querySelector('[data-unscreened-label]') ?? null)).toContain('unscreened');
  });

  it('shows an unscreened tool call’s input', () => {
    renderTranscript(
      [
        step('toolu_1', 'Write', 'apps/x.ts'),
        unscreened('tool_start', { tool_id: 'toolu_1', tool: 'Write', input: '{"file_path":"apps/x.ts"}' }),
      ],
      true,
    );
    const tool = document.querySelector('[data-unscreened] [data-tool="toolu_1"]');
    expect(tool).not.toBeNull();
    expect(textOf(tool?.querySelector('[data-input]') ?? null)).toContain('"file_path":"apps/x.ts"');
    expect(document.querySelector('[data-step]')).toBeNull();
  });

  it('gives way to the screened transcript, which keeps the step’s target', () => {
    renderTranscript(
      [
        step('toolu_1', 'Write', 'apps/x.ts'),
        unscreened('text', { text: 'draft words' }),
        screened('text', { text: 'I wrote the file.' }),
        screened('tool_start', { tool_id: 'toolu_1', tool: 'Write' }),
        screened('tool_result', { tool_id: 'toolu_1', output: 'ok' }),
      ],
      true,
    );
    expect(document.querySelector('[data-unscreened]')).toBeNull();
    expect(textOf(region())).not.toContain('draft words');
    expect(document.querySelector('[data-step]')).toBeNull();
    const tool = document.querySelector('[data-tool="toolu_1"]');
    expect(textOf(tool?.querySelector('[data-target]') ?? null)).toContain('apps/x.ts');
    expect(textOf(region())).toContain('I wrote the file.');
  });
});
