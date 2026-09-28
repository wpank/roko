import { describe, it, expect } from 'vitest';
import {
  STREAM_RECORD_PREFIX,
  TRUNCATED_PREFIX,
  decodeStreamRecord,
  toolGlyph,
  toBlocks,
  type StreamRecord,
  type AttemptDivider,
  type TranscriptEntry,
} from './streamRecord';

// ── Helpers ───────────────────────────────────────────────────────────────────

function frame(kind: string, payload: Record<string, unknown>): string {
  return (
    STREAM_RECORD_PREFIX +
    JSON.stringify({ kind, agent_id: 'a1', plan_id: 'p1', task_id: 't1', attempt: 0, payload })
  );
}

// ── decodeStreamRecord ────────────────────────────────────────────────────────

describe('decodeStreamRecord – text kind', () => {
  it('decodes a framed text record', () => {
    const record = decodeStreamRecord(frame('text', { text: 'hello world' }));
    expect(record).toEqual({ kind: 'text', text: 'hello world' });
  });
});

describe('decodeStreamRecord – reasoning kind', () => {
  it('decodes a framed reasoning record', () => {
    const record = decodeStreamRecord(frame('reasoning', { text: 'I think…' }));
    expect(record).toEqual({ kind: 'reasoning', text: 'I think…' });
  });
});

describe('decodeStreamRecord – tool_start kind', () => {
  it('decodes a framed tool_start record (no arguments)', () => {
    const record = decodeStreamRecord(frame('tool_start', { tool_id: 'tid1', tool: 'Read' }));
    expect(record).toEqual({ kind: 'tool_start', toolId: 'tid1', tool: 'Read' });
  });
});

describe('decodeStreamRecord – tool_result kind', () => {
  it('decodes a framed tool_result (non-truncated)', () => {
    const record = decodeStreamRecord(
      frame('tool_result', { tool_id: 'tid1', output: 'file contents' }),
    );
    expect(record).toEqual({
      kind: 'tool_result',
      toolId: 'tid1',
      output: 'file contents',
      truncated: false,
    });
  });

  it('detects truncation and strips the TRUNCATED_PREFIX marker', () => {
    const truncatedOutput = TRUNCATED_PREFIX + 'tail of the file…';
    const record = decodeStreamRecord(
      frame('tool_result', { tool_id: 'tid2', output: truncatedOutput }),
    );
    expect(record).toEqual({
      kind: 'tool_result',
      toolId: 'tid2',
      output: 'tail of the file…',
      truncated: true,
    });
  });
});

describe('decodeStreamRecord – raw/unframed input', () => {
  it('treats unframed content as raw (malformed: false)', () => {
    const record = decodeStreamRecord('plain text output');
    expect(record).toEqual({ kind: 'raw', text: 'plain text output', malformed: false });
  });

  it('treats a frame prefix with unparsable JSON as malformed raw', () => {
    const bad = STREAM_RECORD_PREFIX + '{not valid json';
    const record = decodeStreamRecord(bad);
    expect(record).toEqual({ kind: 'raw', text: bad, malformed: true });
  });

  it('treats a frame prefix with an unknown kind as malformed raw', () => {
    const content = frame('unknown_kind', {});
    const record = decodeStreamRecord(content);
    expect(record).toEqual({ kind: 'raw', text: content, malformed: true });
  });

  it('never throws on any input', () => {
    expect(() => decodeStreamRecord('')).not.toThrow();
    expect(() => decodeStreamRecord(STREAM_RECORD_PREFIX)).not.toThrow();
    expect(() => decodeStreamRecord('💥')).not.toThrow();
  });
});

// ── toolGlyph ─────────────────────────────────────────────────────────────────

describe('toolGlyph – edit/write family', () => {
  it('returns ✎ for Edit (case-insensitive)', () => {
    expect(toolGlyph('Edit')).toBe('✎');
    expect(toolGlyph('WRITE')).toBe('✎');
    expect(toolGlyph('multiedit')).toBe('✎');
    expect(toolGlyph('NotebookEdit')).toBe('✎');
    expect(toolGlyph('edit_file')).toBe('✎');
    expect(toolGlyph('write_file')).toBe('✎');
    expect(toolGlyph('multi_edit')).toBe('✎');
    expect(toolGlyph('apply_patch')).toBe('✎');
  });
});

describe('toolGlyph – read/search family', () => {
  it('returns ◇ for Read, Grep, Glob, LS, read_file, list_dir, search', () => {
    expect(toolGlyph('Read')).toBe('◇');
    expect(toolGlyph('grep')).toBe('◇');
    expect(toolGlyph('Glob')).toBe('◇');
    expect(toolGlyph('LS')).toBe('◇');
    expect(toolGlyph('read_file')).toBe('◇');
    expect(toolGlyph('list_dir')).toBe('◇');
    expect(toolGlyph('search')).toBe('◇');
  });
});

describe('toolGlyph – default', () => {
  it('returns ⚙ for an unrecognised tool name', () => {
    expect(toolGlyph('Bash')).toBe('⚙');
    expect(toolGlyph('')).toBe('⚙');
    expect(toolGlyph('SomeCustomTool')).toBe('⚙');
  });
});

// ── toBlocks ──────────────────────────────────────────────────────────────────

describe('toBlocks – text merging', () => {
  it('merges consecutive text entries into a single block', () => {
    const entries: TranscriptEntry[] = [
      { kind: 'text', text: 'Hello, ' },
      { kind: 'text', text: 'world' },
      { kind: 'text', text: '!' },
    ];
    const blocks = toBlocks(entries);
    expect(blocks).toEqual([{ kind: 'text', text: 'Hello, world!' }]);
  });
});

describe('toBlocks – reasoning merging', () => {
  it('merges consecutive reasoning entries into a single block', () => {
    const entries: TranscriptEntry[] = [
      { kind: 'reasoning', text: 'Step one. ' },
      { kind: 'reasoning', text: 'Step two.' },
    ];
    const blocks = toBlocks(entries);
    expect(blocks).toEqual([{ kind: 'reasoning', text: 'Step one. Step two.' }]);
  });
});

describe('toBlocks – tool pairing', () => {
  it('attaches a tool_result to the matching tool_start block', () => {
    const entries: TranscriptEntry[] = [
      { kind: 'tool_start', toolId: 'tid1', tool: 'Read' },
      { kind: 'tool_result', toolId: 'tid1', output: 'file data', truncated: false },
    ];
    const blocks = toBlocks(entries);
    expect(blocks).toHaveLength(1);
    expect(blocks[0]).toMatchObject({
      kind: 'tool',
      toolId: 'tid1',
      tool: 'Read',
      glyph: '◇',
      output: 'file data',
      truncated: false,
    });
  });

  it('sets output to null when no result arrived yet', () => {
    const entries: TranscriptEntry[] = [
      { kind: 'tool_start', toolId: 'tid2', tool: 'Bash' },
    ];
    const blocks = toBlocks(entries);
    expect(blocks[0]).toMatchObject({ kind: 'tool', output: null });
  });
});

describe('toBlocks – orphan tool_result', () => {
  it('creates a standalone tool block with tool="" for an unmatched result', () => {
    const entries: TranscriptEntry[] = [
      { kind: 'tool_result', toolId: 'orphan', output: 'unexpected', truncated: false },
    ];
    const blocks = toBlocks(entries);
    expect(blocks).toHaveLength(1);
    expect(blocks[0]).toMatchObject({
      kind: 'tool',
      toolId: 'orphan',
      tool: '',
      output: 'unexpected',
    });
  });
});

describe('toBlocks – dividers', () => {
  it('passes dividers through and ends in-progress merges', () => {
    const divider: AttemptDivider = { kind: 'divider', attempt: 2 };
    const entries: TranscriptEntry[] = [
      { kind: 'text', text: 'before' },
      divider,
      { kind: 'text', text: 'after' },
    ];
    const blocks = toBlocks(entries);
    expect(blocks).toEqual([
      { kind: 'text', text: 'before' },
      { kind: 'divider', attempt: 2 },
      { kind: 'text', text: 'after' },
    ]);
  });
});

describe('toBlocks – mixed sequence', () => {
  it('handles a realistic mixed stream correctly', () => {
    const entries: TranscriptEntry[] = [
      { kind: 'text', text: 'Reading file…' },
      { kind: 'tool_start', toolId: 't1', tool: 'Read' },
      { kind: 'tool_result', toolId: 't1', output: 'content', truncated: false },
      { kind: 'text', text: 'Done.' },
      { kind: 'raw', text: 'legacy output', malformed: false },
    ];
    const blocks = toBlocks(entries);
    expect(blocks).toHaveLength(4);
    expect(blocks[0]).toMatchObject({ kind: 'text', text: 'Reading file…' });
    expect(blocks[1]).toMatchObject({ kind: 'tool', tool: 'Read', output: 'content' });
    expect(blocks[2]).toMatchObject({ kind: 'text', text: 'Done.' });
    expect(blocks[3]).toMatchObject({ kind: 'raw', malformed: false });
  });
});
