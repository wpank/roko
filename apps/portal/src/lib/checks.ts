/**
 * checks.ts — digest compiler/linter output into grouped, deduplicated diagnostics.
 *
 * Gate output from the runner follows the format:
 *   $ <command>
 *   <stdout/stderr lines…>
 *   ✗ exit status 1          ← closing line added by the runner on failure
 *
 * The closing `✗ …` line (last non-blank line of the output, when it is not
 * also the first line) is extracted into `Digest.exit` and stripped before
 * parsing so it never appears in `unparsed`. Recognised endings include:
 *   ✗ exit status <N>
 *   ✗ timed out after <N> ms
 *   ✗ terminated by a signal
 *   ✗ spawn failed: <reason>
 *
 * digestOutput() parses the remaining lines into a structured Digest without
 * ever throwing, and without dropping any line that didn't fit a known format.
 * digestEnding() and digestHeadline() render a Digest's ending and first line.
 */

export interface DigestEntry {
  severity: 'error' | 'warning';
  line: number | null;
  column: number | null;
  message: string;
  count: number;
}

export interface DigestGroup {
  file: string;
  entries: DigestEntry[];
}

export interface Digest {
  /** The command extracted from a leading `$ …` line, or null. */
  command: string | null;
  /** One group per file, errors-first groups, then alphabetical. */
  groups: DigestGroup[];
  /** Lines that matched no recognised format, in original order. */
  unparsed: string[];
  /**
   * The text after `✗ ` on the output's last non-blank line, when that line is
   * not the first line (the `$ …` line). null when absent.
   */
  exit: string | null;
  errorCount: number;
  warningCount: number;
}

// ── Regexes ───────────────────────────────────────────────────────────────────

/** Leading shell command: `$ cargo build` */
const RE_CMD = /^\$ (.+)$/;

/** Closing runner line: `✗ exit status 1`, `✗ timed out after …`, etc. */
const RE_EXIT = /^✗ (.+)$/;

/** Runner framing, not output: the dropped-lines leader and the stderr separator. */
const RE_FRAMING = /^(… (\d+ )?earlier (lines|output) not shown|---stderr---)$/;

/** rustc / cargo: `error[E0425]: msg` or `warning[xxx]: msg` or bare `error: msg` */
const RE_RUSTC = /^(error|warning)(?:\[[\w:]+\])?: (.+)$/;

/** Location emitted by rustc / cargo after the diagnostic: `  --> path:line:col` */
const RE_ARROW = /^\s*-->\s*(.+?):(\d+):(\d+)/;

/**
 * TypeScript compiler (tsc): `path(line,col): error TS2322: msg`
 * The error code is anything non-whitespace before the colon, e.g. TS2322.
 */
const RE_TSC = /^(.+?)\((\d+),(\d+)\): (error|warning) \S+: (.+)$/;

/**
 * Compact compiler form:
 *   `path:line:col: error: msg`
 *   `path:line: warning: msg`
 */
const RE_COMPACT = /^(.+?):(\d+)(?::(\d+))?: (error|warning): (.+)$/;

/**
 * Rust panic, new format (Rust ≥ 1.73):
 *   `thread 'main' panicked at src/main.rs:5:13:`
 * Message is on the next line.
 */
const RE_PANIC_NEW = /^thread '.*?' panicked at (.+?):(\d+):(\d+):?\s*$/;

/**
 * Rust panic, old format:
 *   `thread 'main' panicked at 'explicit panic', src/main.rs:5:13`
 * Message is inline.
 */
const RE_PANIC_OLD = /^thread '.*?' panicked at '(.+?)',\s+(.+?):(\d+)(?::(\d+))?\s*$/;

// ── Core ──────────────────────────────────────────────────────────────────────

/**
 * Digest the raw output of a check command into structured diagnostics.
 *
 * - Identical (file, line, col, severity, message) tuples are merged with a count.
 * - Groups that contain errors sort before warning-only groups; ties break by file name.
 * - Within a group, errors sort before warnings; ties break by line number (nulls last).
 * - Lines that match no recognised format are preserved in `unparsed`, in original order.
 * - Never throws.
 */
export function digestOutput(output: string): Digest {
  try {
    return _digest(output);
  } catch {
    // Fulfil the "never throw" contract unconditionally.
    const unparsed = output ? output.split('\n') : [];
    return { command: null, groups: [], unparsed, exit: null, errorCount: 0, warningCount: 0 };
  }
}

/** True when the step printed nothing besides its `$ command` and closing `✗` lines. */
export function printedNothing(d: Digest): boolean {
  return d.groups.length === 0 && d.unparsed.every((l) => l.trim() === '');
}

/**
 * How a failed step ended, as the checks view says it: the closing line's
 * text (`exit status 1`), plus `no output` when the step printed nothing.
 * null when there is neither.
 */
export function digestEnding(d: Digest): string | null {
  const parts = [d.exit, printedNothing(d) ? 'no output' : null].filter((p) => p !== null);
  return parts.length > 0 ? parts.join(' · ') : null;
}

/**
 * The digest's first line, as one line: the first diagnostic with its
 * location (`src/main.rs:2:5 cannot find value`), else the first output line
 * the digest kept unparsed, else how the step ended.
 */
export function digestHeadline(d: Digest): string | null {
  const group = d.groups[0];
  const entry = group?.entries[0];
  if (group && entry) {
    const at = entry.line === null ? '' : `:${entry.line}${entry.column === null ? '' : `:${entry.column}`}`;
    return `${group.file}${at} ${entry.message}`;
  }
  const line = d.unparsed.map((l) => l.trim()).find((l) => l !== '' && !RE_FRAMING.test(l));
  return line ?? digestEnding(d);
}

// ── Internal ──────────────────────────────────────────────────────────────────

interface PendingRustc {
  severity: 'error' | 'warning';
  message: string;
  /** The original source line, kept so we can add it to unparsed if we never get `-->`. */
  original: string;
}

interface PendingPanic {
  file: string;
  panicLine: number;
  panicCol: number;
  /** The original source line, kept so we can add it to unparsed if truncated. */
  original: string;
}

function _digest(output: string): Digest {
  if (!output) {
    return { command: null, groups: [], unparsed: [], exit: null, errorCount: 0, warningCount: 0 };
  }

  const lines = output.split('\n');
  let command: string | null = null;
  let start = 0;

  // Extract leading command line.
  const cmdMatch = lines[0]?.match(RE_CMD);
  if (cmdMatch) {
    command = cmdMatch[1];
    start = 1;
  }

  // ── Exit line extraction ──────────────────────────────────────────────────
  // Find the last non-blank line. If it matches `✗ <text>` and is not the
  // first line (the `$ …` line, index 0), extract it as the exit reason and
  // exclude it (plus any trailing blank lines) from parsing.
  let exit: string | null = null;
  let end = lines.length; // exclusive upper bound for the main parse loop

  let lastNonBlank = lines.length - 1;
  while (lastNonBlank >= 0 && lines[lastNonBlank].trim() === '') {
    lastNonBlank--;
  }
  if (lastNonBlank > 0) {
    const exitMatch = lines[lastNonBlank].match(RE_EXIT);
    if (exitMatch) {
      exit = exitMatch[1];
      end = lastNonBlank; // exclude the ✗ line and any blank lines after it
    }
  }

  // file → (dedupe-key → entry)
  const fileMap = new Map<string, Map<string, DigestEntry>>();
  const unparsed: string[] = [];

  const addEntry = (
    file: string,
    severity: 'error' | 'warning',
    line: number | null,
    column: number | null,
    message: string,
  ): void => {
    if (!fileMap.has(file)) fileMap.set(file, new Map());
    const key = `${severity}\0${line}\0${column}\0${message}`;
    const m = fileMap.get(file)!;
    const existing = m.get(key);
    if (existing) {
      existing.count++;
    } else {
      m.set(key, { severity, line, column, message, count: 1 });
    }
  };

  let pendingRustc: PendingRustc | null = null;
  let pendingPanic: PendingPanic | null = null;

  const flushPendingRustc = (): void => {
    if (pendingRustc) {
      unparsed.push(pendingRustc.original);
      pendingRustc = null;
    }
  };

  for (let i = start; i < end; i++) {
    const raw = lines[i];

    // ── Pending panic: consume the very next line as the message ──────────────
    if (pendingPanic !== null) {
      const msg = raw.trim();
      if (msg) {
        addEntry(pendingPanic.file, 'error', pendingPanic.panicLine, pendingPanic.panicCol, msg);
        pendingPanic = null;
      } else {
        // Blank line before the message (unusual); skip and keep waiting.
        unparsed.push(raw);
      }
      continue;
    }

    // ── Pending rustc: scan for the `-->` location ────────────────────────────
    if (pendingRustc !== null) {
      const arrow = raw.match(RE_ARROW);
      if (arrow) {
        addEntry(arrow[1], pendingRustc.severity, parseInt(arrow[2], 10), parseInt(arrow[3], 10), pendingRustc.message);
        pendingRustc = null;
        continue;
      }

      // Another rustc diagnostic — flush the previous one and start fresh.
      const nextRustc = raw.match(RE_RUSTC);
      if (nextRustc) {
        flushPendingRustc();
        pendingRustc = { severity: nextRustc[1] as 'error' | 'warning', message: nextRustc[2], original: raw };
        continue;
      }

      // Old-style panic (more specific) overrides the pending rustc entry.
      const panicOld = raw.match(RE_PANIC_OLD);
      if (panicOld) {
        flushPendingRustc();
        addEntry(panicOld[2], 'error', parseInt(panicOld[3], 10), panicOld[4] ? parseInt(panicOld[4], 10) : null, panicOld[1]);
        continue;
      }

      // New-style panic overrides the pending rustc entry.
      const panicNew = raw.match(RE_PANIC_NEW);
      if (panicNew) {
        flushPendingRustc();
        pendingPanic = { file: panicNew[1], panicLine: parseInt(panicNew[2], 10), panicCol: parseInt(panicNew[3], 10), original: raw };
        continue;
      }

      // tsc line overrides the pending rustc entry.
      const tsc = raw.match(RE_TSC);
      if (tsc) {
        flushPendingRustc();
        addEntry(tsc[1], tsc[4] as 'error' | 'warning', parseInt(tsc[2], 10), parseInt(tsc[3], 10), tsc[5]);
        continue;
      }

      // Compact line overrides the pending rustc entry.
      const compact = raw.match(RE_COMPACT);
      if (compact) {
        flushPendingRustc();
        addEntry(compact[1], compact[4] as 'error' | 'warning', parseInt(compact[2], 10), compact[3] ? parseInt(compact[3], 10) : null, compact[5]);
        continue;
      }

      // Neither an arrow nor a new parseable line (e.g. source snippet or blank):
      // add to unparsed but keep waiting for `-->`.
      unparsed.push(raw);
      continue;
    }

    // ── Idle: try each format in priority order ───────────────────────────────

    // Old-style Rust panic must come BEFORE new-style: the new-style regex
    // can greedily match the `'message', path` portion, so we disambiguate
    // by checking the more specific (quoted-message) pattern first.
    const panicOld = raw.match(RE_PANIC_OLD);
    if (panicOld) {
      addEntry(panicOld[2], 'error', parseInt(panicOld[3], 10), panicOld[4] ? parseInt(panicOld[4], 10) : null, panicOld[1]);
      continue;
    }

    // New-style Rust panic: message on the next line.
    const panicNew = raw.match(RE_PANIC_NEW);
    if (panicNew) {
      pendingPanic = { file: panicNew[1], panicLine: parseInt(panicNew[2], 10), panicCol: parseInt(panicNew[3], 10), original: raw };
      continue;
    }

    // rustc / cargo: location follows on the next `-->` line.
    const rustc = raw.match(RE_RUSTC);
    if (rustc) {
      pendingRustc = { severity: rustc[1] as 'error' | 'warning', message: rustc[2], original: raw };
      continue;
    }

    // tsc: `path(line,col): error TSXXXX: msg`
    const tsc = raw.match(RE_TSC);
    if (tsc) {
      addEntry(tsc[1], tsc[4] as 'error' | 'warning', parseInt(tsc[2], 10), parseInt(tsc[3], 10), tsc[5]);
      continue;
    }

    // Compact: `path:line[:col]: error|warning: msg`
    const compact = raw.match(RE_COMPACT);
    if (compact) {
      addEntry(compact[1], compact[4] as 'error' | 'warning', parseInt(compact[2], 10), compact[3] ? parseInt(compact[3], 10) : null, compact[5]);
      continue;
    }

    // Nothing matched — preserve the line.
    unparsed.push(raw);
  }

  // Flush any dangling pending state at end-of-input.
  flushPendingRustc();
  if (pendingPanic) {
    unparsed.push(pendingPanic.original);
    pendingPanic = null;
  }

  // ── Build, sort, and aggregate ────────────────────────────────────────────

  const groups: DigestGroup[] = [];
  for (const [file, entryMap] of fileMap) {
    const entries = Array.from(entryMap.values());
    // Within a group: errors first, then ascending line number (nulls last).
    entries.sort((a, b) => {
      if (a.severity !== b.severity) return a.severity === 'error' ? -1 : 1;
      const al = a.line ?? Infinity;
      const bl = b.line ?? Infinity;
      return al - bl;
    });
    groups.push({ file, entries });
  }

  // Across groups: groups with errors sort before warning-only groups; ties by file name.
  groups.sort((a, b) => {
    const ae = a.entries.some(e => e.severity === 'error');
    const be = b.entries.some(e => e.severity === 'error');
    if (ae !== be) return ae ? -1 : 1;
    return a.file.localeCompare(b.file);
  });

  let errorCount = 0;
  let warningCount = 0;
  for (const g of groups) {
    for (const e of g.entries) {
      if (e.severity === 'error') errorCount += e.count;
      else warningCount += e.count;
    }
  }

  return { command, groups, unparsed, exit, errorCount, warningCount };
}
