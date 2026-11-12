/**
 * writeFailure.ts — why roko couldn't write or revise a plan, in one sentence (gap-6c4d24). The planner's message
 * is long (its retries, its hint, where it kept the drafts); the sentence says what failed and what to do, and the
 * message itself folds, with the project's paths shown as the project names them.
 */

export interface WriteFailure {
  /** One sentence: what failed, and what to do. */
  sentence: string;
  /** The message as roko serve said it, without the operation's own prefix, paths made relative. */
  raw: string;
}

/** `message` with the project's absolute paths written relative to it (`.roko/plan-drafts/x`). */
function relative(message: string, root: string | null | undefined): string {
  const base = root?.replace(/\/+$/, '');
  let out = base ? message.split(`${base}/`).join('') : message;
  // Without the root, a path into the project's .roko still names the project's own place.
  out = out.replace(/(?:\/[^\s/]+)+\/(\.roko\/)/g, '$1');
  return out;
}

/**
 * The sentence for a plan roko couldn't write (`mode` generate) or revise, from the operation's error `message`.
 * `root` is the project's path.
 */
export function writeFailure(message: string, mode: 'generate' | 'revise', root?: string | null): WriteFailure {
  const raw = relative(message.replace(/^plan (?:generation|revision) failed for [^:]+:\s*/i, '').trim(), root);
  const what = mode === 'generate' ? 'write a plan for this request' : 'revise the plan';
  const lower = raw.toLowerCase();
  let sentence: string;
  if (/no toml block|no valid tasks\.toml|did not contain a parseable/.test(lower)) {
    sentence = `roko couldn’t ${what}: the model’s answer had no plan in it. Try again, or say more about what you want.`;
  } else if (/timed out|timeout|deadline/.test(lower)) {
    sentence = `roko couldn’t ${what}: the model took too long to answer. Try again.`;
  } else if (/rate.?limit|429|too many requests/.test(lower)) {
    sentence = `roko couldn’t ${what}: the model is busy. Try again in a minute.`;
  } else if (/api key|unauthori[sz]ed|401|403|no provider|not configured|credential/.test(lower)) {
    sentence = `roko couldn’t ${what}: it has no model it may use. Check the providers in roko.toml, then try again.`;
  } else if (/already exists/.test(lower)) {
    sentence = 'A plan by that name already exists. Open it, or ask for something else.';
  } else if (/does not load after generation|schema|parse/.test(lower)) {
    sentence = `roko couldn’t ${what}: the plan it wrote didn’t hold together. Try again, or say more about what you want.`;
  } else {
    sentence = `roko couldn’t ${what}. Try again; what roko said is below.`;
  }
  return { sentence, raw };
}
