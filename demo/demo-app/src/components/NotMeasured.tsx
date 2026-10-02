import '../showcase/showcase.css';

interface NotMeasuredProps {
  /** S09 experiment ids that will measure it, e.g. `LOG1`, `R-H4`, `E-H5-live`. */
  plannedIn: string[];
  /** What has not been measured, e.g. "M4 audit lottery". */
  what?: string;
}

/**
 * "not yet measured · planned in <experiment id>" (S10 §4.1): what a view or tile shows when
 * there is no data, in place of any placeholder number.
 */
export default function NotMeasured({ plannedIn, what }: NotMeasuredProps) {
  return (
    <p className="sc-not-measured" data-not-measured={plannedIn.join(',')}>
      {what && <span className="sc-not-measured__what">{what}: </span>}
      not yet measured
      {plannedIn.length > 0 && ` · planned in ${plannedIn.join(', ')}`}
    </p>
  );
}
