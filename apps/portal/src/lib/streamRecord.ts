/**
 * streamRecord.ts — decode and assemble framed agent-output records.
 *
 * `agent_output.content` is not raw text.  `publish_stream_record` in
 * `crates/roko-cli/src/runner/tui_bridge.rs` writes STREAM_RECORD_PREFIX
 * (record-separator U+001E + "roko.stream.v1 ") followed by a JSON object
 * with shape `{kind, agent_id, plan_id, task_id, attempt, payload}`.
 *
 * Oversized tool output is stored tail-first: the tail begins with the
 * literal string TRUNCATED_PREFIX = '[...truncated]\n' so callers know that
 * leading bytes were dropped.
 */

// ── Constants ────────────────────────────────────────────────────────────────

/** Wire prefix for every framed stream record (U+001E + "roko.stream.v1 "). */
export const STREAM_RECORD_PREFIX = '\u001eroko.stream.v1 ';

/**
 * Prefix that marks truncated tool output.
 * `forward_dispatch_events_to_tui` in `graph_task_dispatch.rs` prepends this
 * when the output exceeds the size limit so the portal can detect and flag it.
 */
export const TRUNCATED_PREFIX = '[...truncated]\n';

// ── Types ─────────────────────────────────────────────────────────────────────

/** A glyph that indicates the category of a tool call. */
export type ToolGlyph = '✎' | '◇' | '⚙';

/**
 * A decoded stream record extracted from a single `agent_output.content` line.
 *
 * - `text` / `reasoning`   — incremental text deltas; accumulate in the UI.
 * - `tool_start`           — the agent called a tool; no input arguments.
 * - `tool_result`          — the tool returned; `truncated` is true when the
 *                            output was clipped and TRUNCATED_PREFIX stripped.
 * - `raw`                  — unframed legacy text or a record that failed to
 *                            parse.  `malformed` is false for unframed content
 *                            and true when a frame prefix was present but the
 *                            JSON/kind was invalid.
 */
export type StreamRecord =
  | { kind: 'text'; text: string }
  | { kind: 'reasoning'; text: string }
  | { kind: 'tool_start'; toolId: string; tool: string }
  | { kind: 'tool_result'; toolId: string; output: string; truncated: boolean }
  | { kind: 'raw'; text: string; malformed: boolean };

/** A synthetic marker inserted between retried task attempts. */
export interface AttemptDivider {
  kind: 'divider';
  attempt: number;
}

/** Everything that can appear in a raw transcript sequence. */
export type TranscriptEntry = StreamRecord | AttemptDivider;

/**
 * A coalesced, display-ready block produced by `toBlocks`.
 *
 * - `text` / `reasoning`  — merged from consecutive same-kind deltas.
 * - `tool`                — a tool_start + optional tool_result pair; `output`
 *                           is null when no result was received yet.
 * - `raw`                 — passed through unchanged.
 * - `AttemptDivider`      — passed through, resets merge state.
 */
export type TranscriptBlock =
  | { kind: 'text'; text: string }
  | { kind: 'reasoning'; text: string }
  | {
      kind: 'tool';
      toolId: string;
      tool: string;
      glyph: ToolGlyph;
      output: string | null;
      truncated: boolean;
    }
  | { kind: 'raw'; text: string; malformed: boolean }
  | AttemptDivider;

// ── Wire envelope (internal) ──────────────────────────────────────────────────

interface WireEnvelope {
  kind: string;
  agent_id?: string;
  plan_id?: string;
  task_id?: string;
  attempt?: number;
  payload?: Record<string, unknown>;
}

// ── decodeStreamRecord ────────────────────────────────────────────────────────

/**
 * Decode a single `agent_output.content` string into a `StreamRecord`.
 *
 * - Content that does not start with `STREAM_RECORD_PREFIX` is returned as
 *   `{kind:'raw', text: content, malformed: false}` (legacy / unframed output).
 * - A framed record whose JSON is unparsable, or whose `kind` is unknown, is
 *   returned as `{kind:'raw', text: content, malformed: true}`.
 * - This function never throws.
 */
export function decodeStreamRecord(content: string): StreamRecord {
  if (!content.startsWith(STREAM_RECORD_PREFIX)) {
    return { kind: 'raw', text: content, malformed: false };
  }

  const jsonStr = content.slice(STREAM_RECORD_PREFIX.length);
  let envelope: WireEnvelope;
  try {
    envelope = JSON.parse(jsonStr) as WireEnvelope;
  } catch {
    return { kind: 'raw', text: content, malformed: true };
  }

  const { kind, payload = {} } = envelope;

  try {
    switch (kind) {
      case 'text':
        return { kind: 'text', text: String(payload.text ?? '') };

      case 'reasoning':
        return { kind: 'reasoning', text: String(payload.text ?? '') };

      case 'tool_start':
        return {
          kind: 'tool_start',
          toolId: String(payload.tool_id ?? ''),
          tool: String(payload.tool ?? ''),
        };

      case 'tool_result': {
        const rawOutput = String(payload.output ?? '');
        const truncated = rawOutput.startsWith(TRUNCATED_PREFIX);
        const output = truncated ? rawOutput.slice(TRUNCATED_PREFIX.length) : rawOutput;
        return { kind: 'tool_result', toolId: String(payload.tool_id ?? ''), output, truncated };
      }

      default:
        return { kind: 'raw', text: content, malformed: true };
    }
  } catch {
    return { kind: 'raw', text: content, malformed: true };
  }
}

// ── toolGlyph ─────────────────────────────────────────────────────────────────

const EDIT_WRITE_TOOLS = new Set([
  'edit',
  'write',
  'multiedit',
  'notebookedit',
  'edit_file',
  'write_file',
  'multi_edit',
  'apply_patch',
]);

const READ_SEARCH_TOOLS = new Set([
  'read',
  'grep',
  'glob',
  'ls',
  'read_file',
  'list_dir',
  'search',
]);

/**
 * Return the display glyph for a given tool name (case-insensitive).
 *
 * - Edit/write tools → `✎`
 * - Read/search tools → `◇`
 * - Everything else → `⚙`
 */
export function toolGlyph(tool: string): ToolGlyph {
  const lower = tool.toLowerCase();
  if (EDIT_WRITE_TOOLS.has(lower)) return '✎';
  if (READ_SEARCH_TOOLS.has(lower)) return '◇';
  return '⚙';
}

// ── toBlocks ──────────────────────────────────────────────────────────────────

type MutableToolBlock = {
  kind: 'tool';
  toolId: string;
  tool: string;
  glyph: ToolGlyph;
  output: string | null;
  truncated: boolean;
};

/**
 * Coalesce a flat list of `TranscriptEntry` values into display-ready
 * `TranscriptBlock` values.
 *
 * Rules:
 * - Consecutive `text` entries are concatenated into a single `text` block.
 * - Consecutive `reasoning` entries are concatenated into a single `reasoning`
 *   block.
 * - A `tool_result` entry is attached to the open `tool` block whose `toolId`
 *   matches; if no such block exists it becomes a new `tool` block with
 *   `tool: ''`.
 * - A `divider` is passed through and resets any in-progress text/reasoning
 *   merge.
 */
export function toBlocks(entries: readonly TranscriptEntry[]): TranscriptBlock[] {
  const blocks: TranscriptBlock[] = [];
  // Map from toolId → index in `blocks` for open tool_start blocks.
  const openTools = new Map<string, number>();

  let currentText: { kind: 'text'; text: string } | null = null;
  let currentReasoning: { kind: 'reasoning'; text: string } | null = null;

  function flushMerge(): void {
    if (currentText !== null) {
      blocks.push(currentText);
      currentText = null;
    }
    if (currentReasoning !== null) {
      blocks.push(currentReasoning);
      currentReasoning = null;
    }
  }

  for (const entry of entries) {
    switch (entry.kind) {
      case 'text':
        // Flush any in-progress reasoning merge before continuing text merge.
        if (currentReasoning !== null) {
          blocks.push(currentReasoning);
          currentReasoning = null;
        }
        if (currentText === null) {
          currentText = { kind: 'text', text: entry.text };
        } else {
          currentText.text += entry.text;
        }
        break;

      case 'reasoning':
        // Flush any in-progress text merge before continuing reasoning merge.
        if (currentText !== null) {
          blocks.push(currentText);
          currentText = null;
        }
        if (currentReasoning === null) {
          currentReasoning = { kind: 'reasoning', text: entry.text };
        } else {
          currentReasoning.text += entry.text;
        }
        break;

      case 'tool_start': {
        flushMerge();
        const block: MutableToolBlock = {
          kind: 'tool',
          toolId: entry.toolId,
          tool: entry.tool,
          glyph: toolGlyph(entry.tool),
          output: null,
          truncated: false,
        };
        openTools.set(entry.toolId, blocks.length);
        blocks.push(block);
        break;
      }

      case 'tool_result': {
        flushMerge();
        const idx = openTools.get(entry.toolId);
        if (idx !== undefined) {
          const block = blocks[idx] as MutableToolBlock;
          block.output = entry.output;
          block.truncated = entry.truncated;
          openTools.delete(entry.toolId);
        } else {
          // Orphan tool_result — create a standalone tool block.
          blocks.push({
            kind: 'tool',
            toolId: entry.toolId,
            tool: '',
            glyph: toolGlyph(''),
            output: entry.output,
            truncated: entry.truncated,
          });
        }
        break;
      }

      case 'raw':
        flushMerge();
        blocks.push({ kind: 'raw', text: entry.text, malformed: entry.malformed });
        break;

      case 'divider':
        flushMerge();
        blocks.push({ kind: 'divider', attempt: entry.attempt });
        break;
    }
  }

  flushMerge();
  return blocks;
}
