/**
 * Fixture-only routes for the `showcase-fixture` Playwright project (S10 §7): harnesses that
 * render showcase components from the e2e fixture bundles before the pages exist. `main.tsx`
 * mounts this module under `/__fixtures/` only in a non-production build with
 * `VITE_ALLOW_FIXTURES=1`, so none of it reaches a production bundle.
 */
import { useEffect, useState, type ReactElement } from 'react';
import { Route, Routes, useSearchParams } from 'react-router';
import BandLineChart from '../components/Charts/BandLineChart';
import EnvelopeTable from '../components/Charts/EnvelopeTable';
import ParetoFrontierChart from '../components/Charts/ParetoFrontierChart';
import PassKChart from '../components/Charts/PassKChart';
import ProvenanceDrawer from '../components/ProvenanceDrawer';
import type { HeadToHeadView, M4AuditsView } from './contracts';
import './showcase.css';

export interface FixtureRoute {
  /** Path under `/__fixtures/`. */
  path: string;
  element: ReactElement;
}

async function fixtureJson<T>(path: string): Promise<T> {
  const response = await fetch(`${import.meta.env.BASE_URL}bundles/${path}`);
  if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`);
  return (await response.json()) as T;
}

/**
 * `/__fixtures/charts?bundle=<id>`: every CI-aware chart, drawn from one fixture bundle's
 * stored views (default `fx-golden`). The views are not verified here; pages do that.
 */
function ChartHarness() {
  const [params] = useSearchParams();
  const bundle = params.get('bundle') ?? 'fx-golden';
  const [views, setViews] = useState<{ h2h: HeadToHeadView; m4: M4AuditsView } | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    Promise.all([
      fixtureJson<HeadToHeadView>(`${bundle}/views/p1-head-to-head.json`),
      fixtureJson<M4AuditsView>(`${bundle}/views/m4-audits.json`),
    ]).then(
      ([h2h, m4]) => {
        if (live) setViews({ h2h, m4 });
      },
      (err: unknown) => {
        if (live) setError(String(err));
      },
    );
    return () => {
      live = false;
    };
  }, [bundle]);

  if (error) return <main className="sc-harness" data-harness-error={error}>{error}</main>;
  if (!views) return <main className="sc-harness">loading {bundle}</main>;
  const { h2h, m4 } = views;
  return (
    <main className="sc-harness" data-harness="charts" data-bundle={bundle}>
      <ParetoFrontierChart
        arms={h2h.arms}
        frontierArms={h2h.pareto.frontier_arms}
        provenance={<ProvenanceDrawer provenance={h2h.provenance} subject="Pareto chart" />}
      />
      <PassKChart
        arms={h2h.arms}
        provenance={<ProvenanceDrawer provenance={h2h.provenance} subject="pass^k chart" />}
      />
      <EnvelopeTable
        rows={h2h.envelope}
        provenance={<ProvenanceDrawer provenance={h2h.provenance} subject="envelope" />}
      />
      <BandLineChart
        chart="fgr"
        title="False-green rate over time"
        kind="rate"
        yLabel="false-green rate"
        series={[{ id: 'false_green', label: 'false-green', points: m4.series }]}
        provenance={<ProvenanceDrawer provenance={m4.provenance} subject="false-green series" />}
      />
    </main>
  );
}

/** The fixture routes. */
export const FIXTURE_ROUTES: FixtureRoute[] = [
  { path: 'charts', element: <ChartHarness /> },
];

export default function FixtureRoutes() {
  return (
    <Routes>
      {FIXTURE_ROUTES.map((route) => (
        <Route key={route.path} path={route.path} element={route.element} />
      ))}
    </Routes>
  );
}
