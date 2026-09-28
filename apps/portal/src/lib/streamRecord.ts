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
 *
 * Live-output envelope (contract §3.5):
 * During an active agent run, `graph_task_dispatch.rs` may emit records
 * before they have been screened by the gate pipeline.  These carry two
 * additional top-level fields:
 *
 * - `live: true`      — record was streamed directly from the provider; the
 *                       task has not yet finished.
 * - `screened: false` — record is explicitly flagged as not yet gate-screened
 *                       (unscreened text, reasoning, and tool calls/results).
 *                       When `screened` is absent the record was screened
 *                       normally (`unscreened = false`).
 *
 * For a live `tool_start` the payload may carry:
 * - `target`  — the primary file path the tool will act on (non-empty string),
 *               used by the portal to preview edits before they land.
 * - `input`   — the raw tool-call input JSON string, present only for
 *               unscreened calls (`screened: false`).
 *
 * Use `decodeFrame` to read the full envelope.  `decodeStreamRecord` is a
 * thin wrapper that returns only the inner `StreamRecord`.
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

/**
 * A live tool-start step emitted before the task finishes.
 * Produced by the portal when it processes a live `tool_start` envelope
 * (`live: true`, `screened` absent or `true`).
 */
export interface ToolStep {
  kind: 'step';
  toolId: string;
  tool: string;
  target: string | null;
  agentId: string;
  attempt: number;
}

/**
 * A live, not-yet-screened record (text, reasoning, tool call, or result)
 * that arrived before the gate pipeline finished.
 */
export interface UnscreenedEntry {
  kind: 'unscreened';
  record: StreamRecord;
  input: string | null;
  agentId: string;
  attempt: number;
}

/** Everything that can appear in a raw transcript sequence. */
export type TranscriptEntry = StreamRecord | AttemptDivider | ToolStep | UnscreenedEntry;

/**
 * A coalesced, display-ready block produced by `toBlocks`.
 *
 * - `text` / `reasoning`  — merged from consecutive same-kind deltas.
 *                           `unscreened: true` when the content has not yet
 *                           been gate-screened; screened and unscreened runs
 *                           are never merged together.
 * - `tool`                — a tool_start + optional tool_result pair; `output`
 *                           is null when no result was received yet.
 *                           `target` is set when a matching `step` entry carried
 *                           a non-null target.  `input` is set for unscreened
 *                           calls.  `unscreened: true` when the tool call was
 *                           not yet gate-screened.
 * - `step`                — a live tool-start marker with no matching
 *                           `tool_start` record in the transcript yet.
 * - `raw`                 — passed through unchanged.
 * - `AttemptDivider`      — passed through, resets merge state.
 */
export type TranscriptBlock =
  | { kind: 'text'; text: string; unscreened?: true }
  | { kind: 'reasoning'; text: string; unscreened?: true }
  | {
      kind: 'tool';
      toolId: string;
      tool: string;
      glyph: ToolGlyph;
      output: string | null;
      truncated: boolean;
      target?: string;
      input?: string;
      unscreened?: true;
    }
  | { kind: 'step'; toolId: string; tool: string; glyph: ToolGlyph; target: string | null }
  | { kind: 'raw'; text: string; malformed: boolean }
  | AttemptDivider;

// ── Wire envelope (internal) ──────────────────────────────────────────────────

interface WireEnvelope {
  kind: string;
  agent_id?: string;
  plan_id?: string;
  task_id?: string;
  attempt?: number;
  /** Present on live records emitted before gate screening completes. */
  live?: boolean;
  /**
   * Absent on normally-screened records; `false` when the record is
   * explicitly flagged as not yet screened (live-output path).
   */
  screened?: boolean;
  payload?: Record<string, unknown>;
}

// ── Frame ─────────────────────────────────────────────────────────────────────

/**
 * A fully-decoded live-output envelope.
 *
 * `decodeFrame` always returns one of these; `decodeStreamRecord` returns only
 * the inner `record` field for backward compatibility.
 */
export interface Frame {
  /** The decoded stream record (same value `decodeStreamRecord` returns). */
  record: StreamRecord;
  /** `true` when the envelope carried `live: true`. */
  live: boolean;
  /**
   * `true` when the envelope carried `screened: false` — i.e. the record has
   * not yet been gate-screened.
   */
  unscreened: boolean;
  /**
   * For a `tool_start`, the non-empty value of `payload.target`; `null`
   * otherwise (or when the target is absent or an empty string).
   */
  target: string | null;
  /**
   * For an unscreened `tool_start`, the raw tool-call input JSON string
   * (`payload.input`); `null` otherwise.
   */
  input: string | null;
  /** `envelope.attempt` when it is a number; `null` otherwise. */
  attempt: number | null;
  /** `envelope.agent_id` when it is a string; `null` otherwise. */
  agentId: string | null;
}

// ── decodeFrame / decodeStreamRecord ─────────────────────────────────────────

/**
 * Decode a single `agent_output.content` string into a `Frame`.
 *
 * - Unframed content (no `STREAM_RECORD_PREFIX`) → raw record, all flags
 *   false/null.
 * - Unparsable JSON, a non-object top-level value (`null`, `42`, arrays), or
 *   an unknown `kind` → malformed raw record, all flags false/null.
 * - This function never throws.
 */
export function decodeFrame(content: string): Frame {
  const rawFrame = (malformed: boolean): Frame => ({
    record: { kind: 'raw', text: content, malformed },
    live: false,
    unscreened: false,
    target: null,
    input: null,
    attempt: null,
    agentId: null,
  });

  if (!content.startsWith(STREAM_RECORD_PREFIX)) {
    return rawFrame(false);
  }

  const jsonStr = content.slice(STREAM_RECORD_PREFIX.length);
  let parsed: unknown;
  try {
    parsed = JSON.parse(jsonStr);
  } catch {
    return rawFrame(true);
  }

  // Must be a plain, non-null, non-array object.
  if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
    return rawFrame(true);
  }

  const envelope = parsed as WireEnvelope;
  const { kind, payload = {}, live, screened, attempt, agent_id } = envelope;

  const liveFlag = live === true;
  const unscreenedFlag = screened === false;
  const attemptVal = typeof attempt === 'number' ? attempt : null;
  const agentIdVal = typeof agent_id === 'string' ? agent_id : null;

  let record: StreamRecord;
  try {
    switch (kind) {
      case 'text':
        record = { kind: 'text', text: String(payload.text ?? '') };
        break;

      case 'reasoning':
        record = { kind: 'reasoning', text: String(payload.text ?? '') };
        break;

      case 'tool_start':
        record = {
          kind: 'tool_start',
          toolId: String(payload.tool_id ?? ''),
          tool: String(payload.tool ?? ''),
        };
        break;

      case 'tool_result': {
        const rawOutput = String(payload.output ?? '');
        const truncated = rawOutput.startsWith(TRUNCATED_PREFIX);
        const output = truncated ? rawOutput.slice(TRUNCATED_PREFIX.length) : rawOutput;
        record = { kind: 'tool_result', toolId: String(payload.tool_id ?? ''), output, truncated };
        break;
      }

      default:
        return rawFrame(true);
    }
  } catch {
    return rawFrame(true);
  }

  // Extract target / input only for tool_start.
  let target: string | null = null;
  let input: string | null = null;
  if (kind === 'tool_start') {
    const t = payload.target;
    target = typeof t === 'string' && t !== '' ? t : null;
    const inp = payload.input;
    input = typeof inp === 'string' ? inp : null;
  }

  return {
    record,
    live: liveFlag,
    unscreened: unscreenedFlag,
    target,
    input,
    attempt: attemptVal,
    agentId: agentIdVal,
  };
}

/**
 * Decode a single `agent_output.content` string into a `StreamRecord`.
 *
 * - Content that does not start with `STREAM_RECORD_PREFIX` is returned as
 *   `{kind:'raw', text: content, malformed: false}` (legacy / unframed output).
 * - A framed record whose JSON is unparsable, or whose `kind` is unknown, is
 *   returned as `{kind:'raw', text: content, malformed: true}`.
 * - This function never throws.
 *
 * This is a convenience wrapper around `decodeFrame` that returns only the
 * inner `record` field.
 */
export function decodeStreamRecord(content: string): StreamRecord {
  return decodeFrame(content).record;
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
  target?: string;
  input?: string;
  unscreened?: true;
};

/**
 * Coalesce a flat list of `TranscriptEntry` values into display-ready
 * `TranscriptBlock` values.
 *
 * Rules:
 * - Consecutive `text` entries are concatenated into a single `text` block.
 *   Screened and unscreened runs are never merged together; a screened-ness
 *   change flushes the current merge before starting a new one.
 * - Consecutive `reasoning` entries follow the same rule.
 * - A `tool_result` entry is attached to the open `tool` block whose `toolId`
 *   matches; if no such block exists it becomes a new `tool` block with
 *   `tool: ''`.
 * - A `divider` is passed through and resets any in-progress text/reasoning
 *   merge.
 * - A `step` entry whose toolId has a matching `tool_start` anywhere in
 *   `entries` (screened or unscreened) emits no block; its target (when
 *   non-null) is forwarded to that tool block.  A `step` with no matching
 *   `tool_start` is emitted as a `step` block at its position, ending any
 *   text or reasoning merge.
 * - An `unscreened` entry is rendered like its inner record, flagged with
 *   `unscreened: true` on the resulting block.
 */
export function toBlocks(entries: readonly TranscriptEntry[]): TranscriptBlock[] {
  // ── Pre-scan: find toolIds that have a tool_start; collect step targets ──────
  const toolIdHasToolStart = new Set<string>();
  const stepTargets = new Map<string, string | null>();

  for (const entry of entries) {
    if (entry.kind === 'tool_start') {
      toolIdHasToolStart.add(entry.toolId);
    } else if (entry.kind === 'unscreened' && entry.record.kind === 'tool_start') {
      toolIdHasToolStart.add(entry.record.toolId);
    } else if (entry.kind === 'step') {
      stepTargets.set(entry.toolId, entry.target);
    }
  }

  const blocks: TranscriptBlock[] = [];
  // Map from toolId → index in `blocks` for open tool blocks.
  const openTools = new Map<string, number>();

  // In-progress text/reasoning merges.  `unscreened` tracks whether the
  // current run is screened (false/absent) or unscreened (true).
  let currentText: { kind: 'text'; text: string; unscreened?: true } | null = null;
  let currentReasoning: { kind: 'reasoning'; text: string; unscreened?: true } | null = null;

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

  /** Add a text delta, merging only with the same screened-ness. */
  function addText(text: string, unscreened: boolean): void {
    // Flush any in-progress reasoning merge.
    if (currentReasoning !== null) {
      blocks.push(currentReasoning);
      currentReasoning = null;
    }
    // Flush if screened-ness changes.
    if (currentText !== null && !!currentText.unscreened !== unscreened) {
      blocks.push(currentText);
      currentText = null;
    }
    if (currentText === null) {
      currentText = unscreened ? { kind: 'text', text, unscreened: true } : { kind: 'text', text };
    } else {
      currentText.text += text;
    }
  }

  /** Add a reasoning delta, merging only with the same screened-ness. */
  function addReasoning(text: string, unscreened: boolean): void {
    // Flush any in-progress text merge.
    if (currentText !== null) {
      blocks.push(currentText);
      currentText = null;
    }
    // Flush if screened-ness changes.
    if (currentReasoning !== null && !!currentReasoning.unscreened !== unscreened) {
      blocks.push(currentReasoning);
      currentReasoning = null;
    }
    if (currentReasoning === null) {
      currentReasoning = unscreened
        ? { kind: 'reasoning', text, unscreened: true }
        : { kind: 'reasoning', text };
    } else {
      currentReasoning.text += text;
    }
  }

  /** Open a new tool block, attaching step target / input / unscreened as needed. */
  function addToolStart(
    toolId: string,
    tool: string,
    opts: { input?: string | null; unscreened?: boolean },
  ): void {
    flushMerge();
    const block: MutableToolBlock = {
      kind: 'tool',
      toolId,
      tool,
      glyph: toolGlyph(tool),
      output: null,
      truncated: false,
    };
    // Forward target from a matching step entry (only when non-null).
    const target = stepTargets.get(toolId);
    if (target !== undefined && target !== null) {
      block.target = target;
    }
    if (opts.input != null) {
      block.input = opts.input;
    }
    if (opts.unscreened) {
      block.unscreened = true;
    }
    openTools.set(toolId, blocks.length);
    blocks.push(block);
  }

  /** Attach a result to the open tool block, or create a standalone block. */
  function addToolResult(toolId: string, output: string, truncated: boolean): void {
    flushMerge();
    const idx = openTools.get(toolId);
    if (idx !== undefined) {
      const block = blocks[idx] as MutableToolBlock;
      block.output = output;
      block.truncated = truncated;
      openTools.delete(toolId);
    } else {
      // Orphan tool_result — create a standalone tool block.
      blocks.push({
        kind: 'tool',
        toolId,
        tool: '',
        glyph: toolGlyph(''),
        output,
        truncated,
      });
    }
  }

  for (const entry of entries) {
    switch (entry.kind) {
      case 'text':
        addText(entry.text, false);
        break;

      case 'reasoning':
        addReasoning(entry.text, false);
        break;

      case 'tool_start':
        addToolStart(entry.toolId, entry.tool, {});
        break;

      case 'tool_result':
        addToolResult(entry.toolId, entry.output, entry.truncated);
        break;

      case 'raw':
        flushMerge();
        blocks.push({ kind: 'raw', text: entry.text, malformed: entry.malformed });
        break;

      case 'divider':
        flushMerge();
        blocks.push({ kind: 'divider', attempt: entry.attempt });
        break;

      case 'step': {
        if (toolIdHasToolStart.has(entry.toolId)) {
          // Absorbed by the matching tool block — no block emitted here.
          break;
        }
        // No matching tool_start — emit a step block at this position.
        flushMerge();
        blocks.push({
          kind: 'step',
          toolId: entry.toolId,
          tool: entry.tool,
          glyph: toolGlyph(entry.tool),
          target: entry.target,
        });
        break;
      }

      case 'unscreened': {
        const { record, input } = entry;
        switch (record.kind) {
          case 'text':
            addText(record.text, true);
            break;
          case 'reasoning':
            addReasoning(record.text, true);
            break;
          case 'tool_start':
            addToolStart(record.toolId, record.tool, { input, unscreened: true });
            break;
          case 'tool_result':
            addToolResult(record.toolId, record.output, record.truncated);
            break;
          case 'raw':
            flushMerge();
            blocks.push({ kind: 'raw', text: record.text, malformed: record.malformed });
            break;
        }
        break;
      }
    }
  }

  flushMerge();
  return blocks;
}
