import { useEffect, useId, useState, type ReactNode } from 'react';
import type { Provenance, ViewMetric } from '../showcase/contracts';
import '../showcase/showcase.css';

interface ProvenanceDrawerProps {
  provenance: Provenance;
  /** What the drawer explains, e.g. "P1 head-to-head" or "pass^k chart". */
  subject: string;
  /** The metrics the panel shows, with their estimators and intervals. */
  metrics?: ViewMetric[];
  /** The URL of a bundle file, for download links; no links without it. */
  fileHref?: (path: string) => string;
}

function short(hash: string, length = 12): string {
  return hash.length > length ? `${hash.slice(0, length)}…` : hash;
}

interface FieldProps {
  name: string;
  label: string;
  children: ReactNode;
}

function Field({ name, label, children }: FieldProps) {
  return (
    <div className="sc-drawer__field" data-prov-field={name}>
      <dt>{label}</dt>
      <dd>{children}</dd>
    </div>
  );
}

/**
 * The ⓘ on every showcase tile, chart and table (S10 §4.3 L): kind, bundle and run ids,
 * sources with their SHA-256 and verification mark, commits, config hashes, price snapshot,
 * models, n, seeds, window, estimator and CI, the record filter, cost, reproduce commands and
 * download links. Grown from `ProvenanceCard`, which shows only model, run and cost.
 */
export default function ProvenanceDrawer({
  provenance: p,
  subject,
  metrics,
  fileHref,
}: ProvenanceDrawerProps) {
  const [open, setOpen] = useState(false);
  const titleId = useId();

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open]);

  return (
    <>
      <button
        type="button"
        className="sc-prov-btn"
        aria-label={`Provenance: ${subject}`}
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
      >
        {'ⓘ'}
      </button>
      {open && (
        <aside className="sc-drawer" role="dialog" aria-labelledby={titleId}>
          <header className="sc-drawer__head">
            <span id={titleId} className="sc-drawer__title">Provenance · {subject}</span>
            <button
              type="button"
              className="sc-drawer__close"
              aria-label="Close provenance"
              onClick={() => setOpen(false)}
            >
              {'✕'}
            </button>
          </header>
          <dl className="sc-drawer__fields">
            <Field name="kind" label="kind">
              <span className={`sc-kind sc-kind--${p.kind}`}>{p.kind.toUpperCase()}</span>
            </Field>
            <Field name="bundle_id" label="bundle">
              <code>{p.bundle_id}</code>
            </Field>
            <Field name="experiment_ids" label="experiments">{p.experiment_ids.join(', ')}</Field>
            <Field name="run_ids" label={`runs (${p.run_ids.length})`}>
              <span className="sc-drawer__list">
                {p.run_ids.map((id) => <code key={id}>{id}</code>)}
              </span>
            </Field>
            <Field name="sources" label="sources">
              <ul className="sc-drawer__sources">
                {p.sources.map((s) => (
                  <li key={s.path} data-sha256-verified={String(s.sha256_verified)}>
                    <code>{s.path}</code> · {s.schema} · {s.rows} rows
                    <br />
                    <span title={s.sha256}>sha256 {short(s.sha256, 16)}</span>{' '}
                    <span className={s.sha256_verified ? 'sc-ok' : 'sc-bad'}>
                      {s.sha256_verified ? '✓ verified' : '✗ unverified'}
                    </span>
                  </li>
                ))}
              </ul>
            </Field>
            <Field name="harness_commit" label="harness commit">
              <code>{p.harness_commit}</code>
              {p.dirty && <span className="sc-bad"> (dirty tree)</span>}
            </Field>
            <Field name="analysis_commit" label="analysis commit">
              <code>{p.analysis_commit}</code>
            </Field>
            <Field name="config_hashes" label="config hashes">
              <span className="sc-drawer__list">
                {p.config_hashes.map((h) => <code key={h} title={h}>{short(h, 20)}</code>)}
              </span>
            </Field>
            <Field name="price_snapshot_id" label="price snapshot">{p.price_snapshot_id}</Field>
            <Field name="models" label="models">{p.models.join(', ')}</Field>
            <Field name="n" label="n">{p.n}</Field>
            <Field name="seeds" label="seeds">{p.seeds.join(', ')}</Field>
            <Field name="window" label="window">{p.window.from} → {p.window.to}</Field>
            <Field name="estimator" label="estimator">{p.estimator}</Field>
            <Field name="ci" label="interval">
              {p.ci.method}, {Math.round(p.ci.level * 100)}%
              {p.ci.resamples !== null && `, ${p.ci.resamples} resamples`}
              {p.ci.strata.length > 0 && `, strata ${p.ci.strata.join(' × ')}`}
            </Field>
            <Field name="record_filter" label="record filter">
              <code>{p.record_filter}</code>
            </Field>
            <Field name="cost_usd" label="cost">${p.cost_usd.toFixed(2)}</Field>
            <Field name="reproduce" label="reproduce">
              {p.reproduce.map((command) => <pre key={command}>{command}</pre>)}
            </Field>
            {fileHref && (
              <Field name="downloads" label="download">
                <span className="sc-drawer__list">
                  {p.sources.map((s) => (
                    <a key={s.path} href={fileHref(s.path)} download>{s.path}</a>
                  ))}
                </span>
              </Field>
            )}
            {metrics && metrics.length > 0 && (
              <Field name="metrics" label="metrics">
                <ul className="sc-drawer__metrics">
                  {metrics.map((m) => (
                    <li key={m.metric_ref} data-metric-ref={m.metric_ref}>
                      <code>{m.metric}</code>
                      {m.arm && ` · ${m.arm}`} · n {m.n} · {m.estimator} · {m.ci_method}
                    </li>
                  ))}
                </ul>
              </Field>
            )}
          </dl>
        </aside>
      )}
    </>
  );
}
