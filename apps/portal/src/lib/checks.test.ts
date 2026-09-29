import { describe, it, expect } from 'vitest';
import { digestEnding, digestHeadline, digestOutput } from './checks';

// ── helpers ───────────────────────────────────────────────────────────────────

const lines = (...ls: string[]) => ls.join('\n');

// ── tests ─────────────────────────────────────────────────────────────────────

describe('digestOutput – empty input', () => {
  it('returns a zero Digest for an empty string', () => {
    const d = digestOutput('');
    expect(d.command).toBe(null);
    expect(d.groups).toEqual([]);
    expect(d.unparsed).toEqual([]);
    expect(d.errorCount).toBe(0);
    expect(d.warningCount).toBe(0);
  });
});

describe('digestOutput – command line', () => {
  it('extracts the leading `$ cmd` line as command', () => {
    const d = digestOutput(lines(
      '$ cargo build --release',
      'error[E0425]: not found',
      '  --> src/main.rs:5:13',
    ));
    expect(d.command).toBe('cargo build --release');
  });

  it('sets command to null when no leading `$` line', () => {
    const d = digestOutput('error[E0425]: not found\n  --> src/main.rs:5:13');
    expect(d.command).toBe(null);
  });
});

describe('digestOutput – rustc / cargo format', () => {
  it('parses a bracketed error code with location', () => {
    const d = digestOutput(lines(
      'error[E0425]: cannot find value `foo` in this scope',
      '  --> src/main.rs:5:13',
    ));
    expect(d.groups).toHaveLength(1);
    const [g] = d.groups;
    expect(g.file).toBe('src/main.rs');
    const [e] = g.entries;
    expect(e.severity).toBe('error');
    expect(e.line).toBe(5);
    expect(e.column).toBe(13);
    expect(e.message).toBe('cannot find value `foo` in this scope');
    expect(e.count).toBe(1);
    expect(d.errorCount).toBe(1);
    expect(d.warningCount).toBe(0);
  });

  it('parses a warning with a bracketed code', () => {
    const d = digestOutput(lines(
      'warning[unused_imports]: unused import: `std::io`',
      '  --> src/lib.rs:1:5',
    ));
    expect(d.groups[0].entries[0].severity).toBe('warning');
    expect(d.groups[0].file).toBe('src/lib.rs');
    expect(d.warningCount).toBe(1);
    expect(d.errorCount).toBe(0);
  });

  it('sends a rustc line without a following `-->` to unparsed', () => {
    // Summary line: "error: aborting due to 2 previous errors" has no location.
    const d = digestOutput('error: aborting due to 2 previous errors');
    expect(d.groups).toHaveLength(0);
    expect(d.unparsed).toContain('error: aborting due to 2 previous errors');
  });
});

describe('digestOutput – tsc format', () => {
  it('parses `path(line,col): error TSXXXX: msg`', () => {
    const d = digestOutput(
      "src/app/page.tsx(23,7): error TS2322: Type 'string' is not assignable to type 'number'.",
    );
    expect(d.groups).toHaveLength(1);
    const e = d.groups[0].entries[0];
    expect(e.severity).toBe('error');
    expect(e.line).toBe(23);
    expect(e.column).toBe(7);
    expect(e.message).toBe("Type 'string' is not assignable to type 'number'.");
    expect(d.groups[0].file).toBe('src/app/page.tsx');
  });

  it('parses a tsc warning', () => {
    const d = digestOutput("src/util.ts(5,3): warning TS6133: 'x' is declared but its value is never read.");
    expect(d.groups[0].entries[0].severity).toBe('warning');
    expect(d.warningCount).toBe(1);
  });
});

describe('digestOutput – compact compiler form', () => {
  it('parses `path:line:col: error: msg`', () => {
    const d = digestOutput('src/main.rs:10:5: error: something went wrong');
    const e = d.groups[0].entries[0];
    expect(e.severity).toBe('error');
    expect(e.line).toBe(10);
    expect(e.column).toBe(5);
    expect(e.message).toBe('something went wrong');
  });

  it('parses `path:line: warning: msg` (no column)', () => {
    const d = digestOutput('src/util.rs:42: warning: might overflow');
    const e = d.groups[0].entries[0];
    expect(e.severity).toBe('warning');
    expect(e.line).toBe(42);
    expect(e.column).toBe(null);
    expect(e.message).toBe('might overflow');
  });
});

describe('digestOutput – Rust panics', () => {
  it('parses new-format panic with message on the next line', () => {
    const d = digestOutput(lines(
      "thread 'main' panicked at src/main.rs:5:13:",
      'explicit panic',
    ));
    expect(d.groups).toHaveLength(1);
    const e = d.groups[0].entries[0];
    expect(e.severity).toBe('error');
    expect(d.groups[0].file).toBe('src/main.rs');
    expect(e.line).toBe(5);
    expect(e.column).toBe(13);
    expect(e.message).toBe('explicit panic');
  });

  it('parses old-format panic with inline message', () => {
    const d = digestOutput("thread 'main' panicked at 'assertion failed', src/tests.rs:12:5");
    const e = d.groups[0].entries[0];
    expect(e.severity).toBe('error');
    expect(e.message).toBe('assertion failed');
    expect(e.line).toBe(12);
  });
});

describe('digestOutput – deduplication', () => {
  it('increments count for identical (file, line, col, severity, message)', () => {
    const same = 'src/main.rs:10:5: error: duplicate error';
    const d = digestOutput(lines(same, same, same));
    expect(d.groups).toHaveLength(1);
    const [e] = d.groups[0].entries;
    expect(e.count).toBe(3);
    // errorCount reflects the total occurrences, not unique entries.
    expect(d.errorCount).toBe(3);
  });

  it('does not merge entries that differ by line', () => {
    const d = digestOutput(lines(
      'src/main.rs:1:1: error: same message',
      'src/main.rs:2:1: error: same message',
    ));
    expect(d.groups[0].entries).toHaveLength(2);
    expect(d.errorCount).toBe(2);
  });
});

describe('digestOutput – ordering', () => {
  it('puts error groups before warning-only groups', () => {
    const d = digestOutput(lines(
      'b.rs:1:1: warning: warn in b',
      'a.rs:1:1: error: error in a',
    ));
    expect(d.groups[0].file).toBe('a.rs'); // error group first
    expect(d.groups[1].file).toBe('b.rs');
  });

  it('sorts error groups alphabetically when both have errors', () => {
    const d = digestOutput(lines(
      'z.rs:1:1: error: z error',
      'a.rs:1:1: error: a error',
    ));
    expect(d.groups[0].file).toBe('a.rs');
    expect(d.groups[1].file).toBe('z.rs');
  });

  it('puts errors before warnings within the same group, then sorts by line', () => {
    const d = digestOutput(lines(
      'src/main.rs:20:1: warning: late warning',
      'src/main.rs:5:1: error: early error',
      'src/main.rs:10:1: warning: middle warning',
    ));
    const { entries } = d.groups[0];
    expect(entries[0].severity).toBe('error');
    expect(entries[0].line).toBe(5);
    expect(entries[1].severity).toBe('warning');
    expect(entries[1].line).toBe(10);
    expect(entries[2].severity).toBe('warning');
    expect(entries[2].line).toBe(20);
  });
});

describe('digestOutput – unparsed retention', () => {
  it('keeps unrecognised lines in order', () => {
    const d = digestOutput(lines('random text', 'more stuff', 'note: something'));
    expect(d.unparsed).toEqual(['random text', 'more stuff', 'note: something']);
    expect(d.groups).toHaveLength(0);
  });

  it('keeps source-snippet lines from cargo output in unparsed', () => {
    // After the `-->` line is consumed, remaining snippet lines should go to unparsed.
    const d = digestOutput(lines(
      'error[E0425]: cannot find value `foo`',
      '  --> src/main.rs:5:13',
      '   |',
      '5  |     let x = foo;',
      '   |             ^^^ not found',
    ));
    expect(d.groups).toHaveLength(1);
    expect(d.groups[0].entries[0].line).toBe(5);
    // The three snippet lines are unrecognised and go to unparsed.
    expect(d.unparsed).toContain('   |');
    expect(d.unparsed).toContain('5  |     let x = foo;');
  });

  it('does not throw on malformed / adversarial input', () => {
    expect(() => digestOutput('null\x00bytes\nand\ttabs\n\n\n')).not.toThrow();
    expect(() => digestOutput('\n\n\n')).not.toThrow();
    expect(() => digestOutput('$')).not.toThrow();
  });
});

describe('digestEnding', () => {
  it('says how a step ended, and no output when it printed nothing', () => {
    expect(digestEnding(digestOutput('$ test -f MISSING.md\n✗ exit status 1'))).toBe('exit status 1 · no output');
    expect(digestEnding(digestOutput(lines('$ make', 'make: *** No rule', '✗ exit status 2')))).toBe('exit status 2');
    expect(digestEnding(digestOutput('$ test -f MISSING.md'))).toBe('no output');
    expect(digestEnding(digestOutput(lines('$ make', 'make: *** No rule')))).toBeNull();
  });
});

describe('digestHeadline', () => {
  it('names the first diagnostic with its location', () => {
    const d = digestOutput(lines(
      '$ cargo build',
      'warning: unused import',
      '  --> src/lib.rs:1:1',
      'error[E0425]: cannot find value `x` in this scope',
      '  --> src/main.rs:2:5',
      '✗ exit status 101',
    ));
    expect(digestHeadline(d)).toBe('src/main.rs:2:5 cannot find value `x` in this scope');
  });

  it('leaves out a column it does not know', () => {
    expect(digestHeadline(digestOutput('src/a.c:7: warning: unused variable'))).toBe('src/a.c:7 unused variable');
  });

  it('falls back to the first output line, past the runner’s framing', () => {
    const d = digestOutput(lines('$ cargo build', '… 12 earlier lines not shown', '', '---stderr---', 'error: could not compile `hello`', '✗ exit status 101'));
    expect(digestHeadline(d)).toBe('error: could not compile `hello`');
  });

  it('says how a silent step ended', () => {
    expect(digestHeadline(digestOutput('$ test -f MISSING.md\n✗ exit status 1'))).toBe('exit status 1 · no output');
  });
});
