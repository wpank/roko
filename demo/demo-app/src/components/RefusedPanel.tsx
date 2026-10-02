import type { RefusalReason } from '../showcase/contracts';
import '../showcase/showcase.css';

/** What each refusal code means, in the words a visitor reads (S10 §4.3 L). */
const EXPLANATIONS: Record<RefusalReason, string> = {
  simulated: 'The data is marked simulated. Simulated numbers are never shown.',
  fixture: 'The data is a test fixture. Fixtures render only in a test build.',
  no_run_ids: 'No run ids back these numbers, so nobody could check them.',
  integrity: "A file's SHA-256 digest does not match the bundle's SHA256SUMS.",
  missing_provenance: 'The data carries no complete provenance envelope.',
  ci_missing: 'A rate has no confidence interval.',
  n_missing: 'A number has no sample size.',
};

interface RefusedPanelProps {
  reason: RefusalReason;
  /** The guard's detail, e.g. which file failed its digest. */
  detail?: string;
  /** What was refused, e.g. "P1 head-to-head". */
  subject?: string;
}

/** Shown instead of a view or chart the render guard refused; it never draws the data. */
export default function RefusedPanel({ reason, detail, subject }: RefusedPanelProps) {
  return (
    <section className="sc-refused" role="alert" data-refused-reason={reason}>
      <div className="sc-refused__head">
        <span className="sc-refused__mark" aria-hidden="true">{'⊘'}</span>
        <span className="sc-refused__title">Not rendered{subject ? `: ${subject}` : ''}</span>
        <code className="sc-refused__code">{reason}</code>
      </div>
      <p className="sc-refused__why">{EXPLANATIONS[reason]}</p>
      {detail && <p className="sc-refused__detail">{detail}</p>}
    </section>
  );
}
