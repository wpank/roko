import { Link } from 'react-router';
import { MetricValue } from '../../components/Charts/ChartTable';
import NotMeasured from '../../components/NotMeasured';
import type {
  MetricKind,
  OverviewTile,
  OverviewView,
  TileRow,
  ViewMetric,
} from '../../showcase/contracts';
import {
  Badges,
  Drawer,
  GuardedView,
  VIEW_PATHS,
  useBundleChoice,
  type BundleChoice,
} from './ViewFrame';

function kindOf(view: OverviewView, metricRef: string): MetricKind {
  return view.metrics.find((m) => m.metric_ref === metricRef)?.kind ?? 'score';
}

function metricsOf(view: OverviewView, rows: TileRow[]): ViewMetric[] {
  const refs = new Set(rows.map((row) => row.estimate.metric_ref));
  return view.metrics.filter((m) => refs.has(m.metric_ref));
}

function Rows({ view, rows }: { view: OverviewView; rows: TileRow[] }) {
  return (
    <dl className="sc-tile__rows">
      {rows.map((row) => (
        <div key={row.label} className="sc-tile__row">
          <dt>{row.label}</dt>
          <dd>
            <MetricValue estimate={row.estimate} kind={kindOf(view, row.estimate.metric_ref)} />
          </dd>
        </div>
      ))}
    </dl>
  );
}

function Tile({ tile, view, query }: { tile: OverviewTile; view: OverviewView; query: string }) {
  const measured = tile.rows.length > 0;
  return (
    <article className="sc-tile" data-tile={tile.id} data-claim-state={tile.claim_state}>
      <header className="sc-tile__head">
        <span className="sc-tile__title">
          {tile.mechanism && <span className="sc-tile__mechanism">{tile.mechanism} </span>}
          {tile.title}
        </span>
        {tile.hypothesis && <span className="sc-tile__hypothesis">{tile.hypothesis}</span>}
        <Drawer provenance={view.provenance} subject={tile.title} metrics={metricsOf(view, tile.rows)} />
      </header>
      {measured ? <Rows view={view} rows={tile.rows} /> : <NotMeasured plannedIn={tile.planned_in} />}
      <footer className="sc-tile__foot">
        {measured && (
          <span className={`sc-claim__state sc-claim__state--${tile.claim_state}`}>
            {tile.claim_state.replace(/_/g, ' ')}
          </span>
        )}
        {tile.view && <Link to={`${VIEW_PATHS[tile.view]}${query}`}>open {'→'}</Link>}
      </footer>
    </article>
  );
}

/** The claims board (S10 §4.3 A): P1 and M4 tiles, and the strip of negative results. */
export function OverviewBody({ view, choice }: { view: OverviewView; choice: BundleChoice }) {
  const pillars = [
    { id: 'P1', title: 'P1 · Dependability economics' },
    { id: 'P2', title: 'P2 · Self-regulation' },
  ] as const;
  return (
    <>
      <Badges choice={choice} provenance={view.provenance} />
      {pillars.map((pillar) => {
        const tiles = view.tiles.filter((tile) => tile.pillar === pillar.id);
        if (tiles.length === 0) return null;
        return (
          <section key={pillar.id} className="sc-pillar" data-pillar={pillar.id}>
            <h2 className="sc-section-title">{pillar.title}</h2>
            <div className="sc-tiles">
              {tiles.map((tile) => (
                <Tile key={tile.id} tile={tile} view={view} query={choice.query} />
              ))}
            </div>
          </section>
        );
      })}
      <section className="sc-negatives" data-negatives={view.negatives.length}>
        <h2 className="sc-section-title">Where it does not hold</h2>
        {view.negatives.length === 0 ? (
          <p className="sc-dim">This bundle records no negative result.</p>
        ) : (
          <ul className="sc-negatives__list">
            {view.negatives.map((negative) => (
              <li key={negative.id} data-negative={negative.kind}>
                <span className="sc-negatives__text">{negative.text}</span>
                {negative.rows.length > 0 && <Rows view={view} rows={negative.rows} />}
                {negative.view && (
                  <Link to={`${VIEW_PATHS[negative.view]}${choice.query}`}>open {'→'}</Link>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>
      <nav className="sc-links" aria-label="More">
        <Link to="/replays">Replays {'→'}</Link>
        <span className="sc-dim">Run console: off (replay-only)</span>
      </nav>
    </>
  );
}

/** `/demo/`: the showcase home (S10 §4.3 A). */
export default function Overview() {
  const choice = useBundleChoice();
  return (
    <section className="sc-page" data-showcase-page="overview">
      <header className="sc-hero">
        <h1 className="sc-hero__title">A Cybernetic Agent Harness</h1>
        <p className="sc-hero__sub">Cheap models, made dependable by measured self-regulation.</p>
      </header>
      <GuardedView choice={choice} view="overview" subject="Overview">
        {(view) => <OverviewBody view={view} choice={choice} />}
      </GuardedView>
    </section>
  );
}
