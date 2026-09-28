/**
 * Acceptance: toBlocks shows live tool steps as rows until a tool block for the
 * same call absorbs them (target included), and keeps unscreened content
 * flagged and apart from screened content.
 * Copied verbatim from plans/portal-programme/08c-portal-live-steps/accept/.
 */
import { describe, expect, it } from 'vitest';
import { toBlocks } from '@/lib/streamRecord';
import type { StreamRecord, TranscriptEntry } from '@/lib/streamRecord';

const step = (toolId: string, tool: string, target: string | null): TranscriptEntry => ({
  kind: 'step',
  toolId,
  tool,
  target,
  agentId: 'a1',
  attempt: 0,
});
const unscreened = (record: StreamRecord, input: string | null = null): TranscriptEntry => ({
  kind: 'unscreened',
  record,
  input,
  agentId: 'a1',
  attempt: 0,
});

describe('toBlocks with live steps', () => {
  it('renders a step as a row with its glyph and target', () => {
    expect(toBlocks([step('toolu_1', 'Write', 'apps/x.ts')])).toEqual([
      { kind: 'step', toolId: 'toolu_1', tool: 'Write', glyph: '✎', target: 'apps/x.ts' },
    ]);
  });

  it('picks the glyph from the tool name', () => {
    const blocks = toBlocks([step('a', 'Read', 'src/lib.rs'), step('b', 'Bash', 'cargo test')]);
    expect(blocks.map((b) => (b.kind === 'step' ? b.glyph : null))).toEqual(['◇', '⚙']);
  });

  it('keeps a step without a target', () => {
    expect(toBlocks([step('t', 'Bash', null)])).toEqual([
      { kind: 'step', toolId: 't', tool: 'Bash', glyph: '⚙', target: null },
    ]);
  });

  it('lets the screened tool block absorb its step, target included', () => {
    const blocks = toBlocks([
      step('toolu_1', 'Write', 'apps/x.ts'),
      { kind: 'text', text: 'I wrote the file.' },
      { kind: 'tool_start', toolId: 'toolu_1', tool: 'Write' },
      { kind: 'tool_result', toolId: 'toolu_1', output: 'ok', truncated: false },
    ]);
    expect(blocks).toEqual([
      { kind: 'text', text: 'I wrote the file.' },
      { kind: 'tool', toolId: 'toolu_1', tool: 'Write', glyph: '✎', output: 'ok', truncated: false, target: 'apps/x.ts' },
    ]);
  });

  it('adds no target to a tool block whose step had none', () => {
    const blocks = toBlocks([step('t', 'Bash', null), { kind: 'tool_start', toolId: 't', tool: 'Bash' }]);
    expect(blocks).toEqual([
      { kind: 'tool', toolId: 't', tool: 'Bash', glyph: '⚙', output: null, truncated: false },
    ]);
  });

  it('ends a text merge at a step row', () => {
    const blocks = toBlocks([
      { kind: 'text', text: 'a' },
      step('t', 'Bash', 'ls'),
      { kind: 'text', text: 'b' },
    ]);
    expect(blocks.map((b) => b.kind)).toEqual(['text', 'step', 'text']);
  });

  it('leaves screened blocks exactly as before', () => {
    expect(
      toBlocks([
        { kind: 'tool_start', toolId: 'x', tool: 'Read' },
        { kind: 'tool_result', toolId: 'x', output: 'content', truncated: false },
      ]),
    ).toEqual([{ kind: 'tool', toolId: 'x', tool: 'Read', glyph: '◇', output: 'content', truncated: false }]);
  });
});

describe('toBlocks with unscreened content', () => {
  it('flags unscreened text and keeps it apart from screened text', () => {
    expect(
      toBlocks([
        { kind: 'text', text: 'screened ' },
        unscreened({ kind: 'text', text: 'live ' }),
        unscreened({ kind: 'text', text: 'draft' }),
        { kind: 'text', text: 'screened again' },
      ]),
    ).toEqual([
      { kind: 'text', text: 'screened ' },
      { kind: 'text', text: 'live draft', unscreened: true },
      { kind: 'text', text: 'screened again' },
    ]);
  });

  it('flags unscreened reasoning', () => {
    expect(toBlocks([unscreened({ kind: 'reasoning', text: 'thinking' })])).toEqual([
      { kind: 'reasoning', text: 'thinking', unscreened: true },
    ]);
  });

  it('keeps an unscreened tool call’s input and result, and absorbs its step', () => {
    const blocks = toBlocks([
      step('toolu_1', 'Write', 'apps/x.ts'),
      unscreened({ kind: 'tool_start', toolId: 'toolu_1', tool: 'Write' }, '{"file_path":"apps/x.ts"}'),
      unscreened({ kind: 'tool_result', toolId: 'toolu_1', output: 'wrote', truncated: false }),
    ]);
    expect(blocks).toEqual([
      {
        kind: 'tool',
        toolId: 'toolu_1',
        tool: 'Write',
        glyph: '✎',
        output: 'wrote',
        truncated: false,
        target: 'apps/x.ts',
        input: '{"file_path":"apps/x.ts"}',
        unscreened: true,
      },
    ]);
  });
});
