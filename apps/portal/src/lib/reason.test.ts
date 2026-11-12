/**
 * Why a check failed, from its output (gap-6d2dd2): the line a person looks for, with the project's own paths.
 */
import { describe, expect, it } from 'vitest';
import { failureReason, projectPath } from '@/lib/reason';

const WORKTREE = '/work/greeting-app/.roko/worktrees/attempt-4f1c';

describe('failureReason', () => {
  it('ends a Python traceback on its error, at its last frame in the project', () => {
    const output = [
      "$ python3 -c 'import settings'",
      '---stderr---',
      'Traceback (most recent call last):',
      '  File "<string>", line 1, in <module>',
      `  File "${WORKTREE}/settings.py", line 2`,
      '    return dict(',
      '               ^',
      "SyntaxError: '(' was never closed",
      '✗ exit status 1',
    ].join('\n');
    expect(failureReason(output)).toEqual({ at: 'settings.py, line 2', message: "'(' was never closed" });
  });

  it('gives no place when every frame is outside the project', () => {
    const output = [
      'Traceback (most recent call last):',
      '  File "<string>", line 1, in <module>',
      "ModuleNotFoundError: No module named 'settings'",
    ].join('\n');
    expect(failureReason(output)).toEqual({ at: null, message: "No module named 'settings'" });
  });

  it('names the test pytest says failed', () => {
    const output = [
      '=========================== short test summary info ============================',
      'FAILED tests/test_settings.py::test_load - AssertionError: assert {} == {"a": "1"}',
      '========================= 1 failed, 2 passed in 0.12s =========================',
    ].join('\n');
    expect(failureReason(output)).toEqual({
      at: 'tests/test_settings.py::test_load',
      message: 'AssertionError: assert {} == {"a": "1"}',
    });
  });

  it('takes a compiler’s file and line', () => {
    const output = [
      '$ cargo build',
      'error[E0425]: cannot find value `nme` in this scope',
      ` --> ${WORKTREE}/src/main.rs:2:20`,
      '✗ exit status 101',
    ].join('\n');
    expect(failureReason(output)).toEqual({ at: 'src/main.rs, line 2', message: 'cannot find value `nme` in this scope' });
  });

  it('ends anything else on its last line, not the runner’s', () => {
    const output = ['$ rustc hello/main.rs -o hello/hello-bin', '---stderr---', 'bash: rustc: command not found', '✗ exit status 127'].join(
      '\n',
    );
    expect(failureReason(output)).toEqual({ at: null, message: 'bash: rustc: command not found' });
  });

  it('writes the project’s paths without the try’s worktree', () => {
    expect(failureReason(`cat: ${WORKTREE}/missing.txt: No such file or directory`)).toEqual({
      at: null,
      message: 'cat: missing.txt: No such file or directory',
    });
    expect(projectPath('/work/greeting-app/settings.py', '/work/greeting-app')).toBe('settings.py');
  });

  it('has nothing to say about an output with nothing in it', () => {
    expect(failureReason('$ true\n✗ exit status 1')).toBeNull();
  });
});
