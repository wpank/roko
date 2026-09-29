/**
 * Acceptance: decodeFrame reads the live-output envelope (contract §3.5) — a
 * live tool step with its target, and unscreened records — while
 * decodeStreamRecord keeps returning the plain record.
 * Copied verbatim from plans/portal-programme/08c-portal-live-steps/accept/.
 */
import { describe, expect, it } from 'vitest';
import { decodeFrame, decodeStreamRecord, STREAM_RECORD_PREFIX } from '@/lib/streamRecord';

const frame = (o: Record<string, unknown>) => STREAM_RECORD_PREFIX + JSON.stringify(o);
const BASE = { agent_id: 'hello/T01', plan_id: 'hello', task_id: 'T01', attempt: 0 };
const step = (payload: Record<string, unknown>) => frame({ kind: 'tool_start', live: true, ...BASE, payload });
const unscreened = (kind: string, payload: Record<string, unknown>) =>
  frame({ kind, live: true, screened: false, ...BASE, payload });

describe('decodeFrame', () => {
  it('reads a live tool step with its target', () => {
    expect(decodeFrame(step({ tool_id: 'toolu_1', tool: 'Write', target: 'apps/x.ts' }))).toEqual({
      record: { kind: 'tool_start', toolId: 'toolu_1', tool: 'Write' },
      live: true,
      unscreened: false,
      target: 'apps/x.ts',
      input: null,
      attempt: 0,
      agentId: 'hello/T01',
    });
  });

  it('treats an empty or missing target as none', () => {
    expect(decodeFrame(step({ tool_id: 't', tool: 'Bash', target: '' })).target).toBeNull();
    expect(decodeFrame(step({ tool_id: 't', tool: 'Bash' })).target).toBeNull();
  });

  it('reads unscreened text and reasoning', () => {
    const text = decodeFrame(unscreened('text', { text: 'Writing the file…' }));
    expect(text.record).toEqual({ kind: 'text', text: 'Writing the file…' });
    expect([text.live, text.unscreened]).toEqual([true, true]);
    expect(decodeFrame(unscreened('reasoning', { text: 'hmm' })).record).toEqual({ kind: 'reasoning', text: 'hmm' });
  });

  it('keeps an unscreened tool call’s input', () => {
    const f = decodeFrame(unscreened('tool_start', { tool_id: 'toolu_1', tool: 'Write', input: '{"file_path":"apps/x.ts"}' }));
    expect(f.record).toEqual({ kind: 'tool_start', toolId: 'toolu_1', tool: 'Write' });
    expect(f.input).toBe('{"file_path":"apps/x.ts"}');
    expect(f.unscreened).toBe(true);
  });

  it('reads an unscreened tool result', () => {
    const f = decodeFrame(unscreened('tool_result', { tool_id: 'toolu_1', output: 'wrote 12 lines' }));
    expect(f.record).toEqual({ kind: 'tool_result', toolId: 'toolu_1', output: 'wrote 12 lines', truncated: false });
    expect(f.unscreened).toBe(true);
  });

  it('marks a screened record as neither live nor unscreened', () => {
    const f = decodeFrame(frame({ kind: 'text', ...BASE, payload: { text: 'Done.' } }));
    expect([f.live, f.unscreened, f.target, f.input]).toEqual([false, false, null, null]);
    expect(f.record).toEqual({ kind: 'text', text: 'Done.' });
  });

  it('decodes unframed and malformed content as plain raw frames', () => {
    expect(decodeFrame('plain output')).toMatchObject({
      record: { kind: 'raw', text: 'plain output', malformed: false },
      live: false,
      unscreened: false,
    });
    for (const bad of [STREAM_RECORD_PREFIX + '{nope', STREAM_RECORD_PREFIX + 'null', STREAM_RECORD_PREFIX + '42']) {
      expect(decodeFrame(bad)).toMatchObject({ record: { kind: 'raw', malformed: true }, live: false });
    }
  });

  it('leaves decodeStreamRecord returning the plain record', () => {
    expect(decodeStreamRecord(step({ tool_id: 'toolu_1', tool: 'Write', target: 'apps/x.ts' }))).toEqual({
      kind: 'tool_start',
      toolId: 'toolu_1',
      tool: 'Write',
    });
  });
});
