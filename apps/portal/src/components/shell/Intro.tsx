/**
 * What a project with no plans opens on: what roko is, in two sentences, then (from Stage) the request box, then
 * the four steps from a request to merged work.
 */
export function Intro() {
  return (
    <section className="pt-intro" data-region="intro">
      <h1 className="rk-display">
        Ask for a change.
        <br />
        Get checked work.
      </h1>
      <p>
        <b>roko</b> turns a request into a plan of small tasks, each with a check your project can run. It works through
        them with AI models, tries again when a check fails, and keeps only work that passes, on its own branch.
      </p>
    </section>
  );
}

/** The four steps, under the request box. */
export function HowItWorks() {
  return (
    <ol className="pt-how" aria-label="How it works">
      <li>
        <b>1 · Ask</b>Say what you want, in plain words.
      </li>
      <li>
        <b>2 · Review</b>roko writes tasks and the checks that prove them. Edit anything.
      </li>
      <li>
        <b>3 · Run</b>Watch each task work, get checked and try again, under a spending cap.
      </li>
      <li>
        <b>4 · Merge</b>Read what changed, then take it into your branch.
      </li>
    </ol>
  );
}
