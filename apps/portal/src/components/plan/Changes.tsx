'use client';

/**
 * Changes — what a run, or one task, changed (gap-6d2dd2): a line per file (`README.md +7 new file`) and, folded
 * under it, the file's patch with added lines green and removed lines red.
 */

import { Fold } from '@/components/kit/Fold';

/** One changed file, from a run's diff or a task's. */
export interface ChangedFile {
  path: string;
  /** `added`, `deleted` or `modified`. */
  status: string;
  additions: number;
  deletions: number;
  /** The unified patch; null for a binary file. */
  patch: string | null;
  binary?: boolean;
  truncated?: boolean;
}

/** `README.md +7 new file`: the file, its line counts and, when it came or went, that. */
export function FileLine({ file }: { file: ChangedFile }) {
  const how = file.status === 'added' ? 'new file' : file.status === 'deleted' ? 'deleted' : null;
  return (
    <span className="pt-file" data-file={file.path}>
      <b>{file.path}</b>
      {file.binary ? (
        <span className="pt-dim"> binary</span>
      ) : (
        <>
          {file.additions > 0 && <span className="pt-add"> +{file.additions}</span>}
          {file.deletions > 0 && <span className="pt-del"> −{file.deletions}</span>}
        </>
      )}
      {how && <span className="pt-dim"> {how}</span>}
    </span>
  );
}

/**
 * A unified patch from its first hunk (git's header lines say nothing a person needs), line by line: `+` lines
 * green, `-` lines red, hunk headers dim.
 */
export function Patch({ patch, truncated }: { patch: string; truncated?: boolean }) {
  const all = patch.replace(/\n$/, '').split('\n');
  const firstHunk = all.findIndex((line) => line.startsWith('@@'));
  const lines = firstHunk > 0 ? all.slice(firstHunk) : all;
  return (
    <pre className="pt-patch" data-patch>
      {lines.map((line, i) => (
        <span key={i} data-line={kind(line)}>
          {line}
          {'\n'}
        </span>
      ))}
      {truncated && <span data-line="hunk">… the rest of this file’s change is not shown</span>}
    </pre>
  );
}

function kind(line: string): 'add' | 'del' | 'hunk' | 'meta' | 'same' {
  if (line.startsWith('+++') || line.startsWith('---') || line.startsWith('diff ') || line.startsWith('index ')) {
    return 'meta';
  }
  if (line.startsWith('@@')) return 'hunk';
  if (line.startsWith('+')) return 'add';
  if (line.startsWith('-')) return 'del';
  return 'same';
}

/** Each file on its own line, its patch folded under it. */
export function ChangeList({ files }: { files: readonly ChangedFile[] }) {
  return (
    <ul className="pt-changes">
      {files.map((file) => (
        <li key={file.path}>
          {file.patch ? (
            <Fold summary={<FileLine file={file} />}>
              <Patch patch={file.patch} truncated={file.truncated} />
            </Fold>
          ) : (
            <FileLine file={file} />
          )}
        </li>
      ))}
    </ul>
  );
}
