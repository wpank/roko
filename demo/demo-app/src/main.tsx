import { StrictMode, Suspense, lazy } from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter, Navigate, Routes, Route, useLocation } from 'react-router';
import ErrorBoundary from './components/ErrorBoundary';
import AppShell from './components/AppShell';
import { ToastProvider } from './components/Toast';
import { bootstrapTransport } from './app/bootstrap';
import './styles/rosedust.css';
import './styles/typography.css';
import './styles/animations.css';
import './styles/motion.css';
import './styles/interactions.css';
import './styles/loading.css';
import './styles/ambient.css';
import './styles/scrollbar.css';
import './styles/focus.css';
import './styles/gradient-borders.css';

const Landing = lazy(() => import('./pages/Landing'));
const DashboardLayout = lazy(() => import('./pages/dashboard/Layout'));
const CostDashboard = lazy(() => import('./pages/dashboard/CostDashboard'));
const AgentFleet = lazy(() => import('./pages/dashboard/AgentFleet'));
const KnowledgeGraph = lazy(() => import('./pages/dashboard/KnowledgeGraph'));
const IntegrityView = lazy(() => import('./pages/dashboard/IntegrityView'));
const CascadeRouter = lazy(() => import('./pages/dashboard/CascadeRouter'));
const KnowledgeEntries = lazy(() => import('./pages/dashboard/KnowledgeEntries'));
const DreamsView = lazy(() => import('./pages/dashboard/DreamsView'));
const FeedsDashboard = lazy(() => import('./pages/feeds/FeedsDashboard'));
const RelayDashboard = lazy(() => import('./pages/dashboard/RelayDashboard'));
const Terminal = lazy(() => import('./pages/Terminal'));
const Builder = lazy(() => import('./pages/Builder'));
const Explorer = lazy(() => import('./pages/Explorer/index'));
const Bench = lazy(() => import('./pages/Bench'));
const BenchRunDetail = lazy(() => import('./pages/BenchRunDetail'));
const BenchCompare = lazy(() => import('./pages/BenchCompare'));
const Settings = lazy(() => import('./pages/Settings'));
const SharePage = lazy(() => import('./pages/Share'));

// Fixture-only harness routes (S10 §7). The condition is replaced at build time, so a build
// without VITE_ALLOW_FIXTURES=1 compiles the import out.
const FixtureRoutes =
  import.meta.env.MODE !== 'production' && import.meta.env.VITE_ALLOW_FIXTURES === '1'
    ? lazy(() => import('./showcase/fixtureRoutes'))
    : null;

/** The legacy pages that moved under /lab (D23); their old top-level paths redirect. */
const LAB_PATHS = [
  'demo',
  'dashboard',
  'terminal',
  'builder',
  'explorer',
  'settings',
  'bench',
  'share',
];

/** An old top-level path: the same page under /lab, keeping the query and hash. */
function ToLab() {
  const { pathname, search, hash } = useLocation();
  return <Navigate to={`/lab${pathname}${search}${hash}`} replace />;
}

/** The showcase home until the R1 pages land (S10 §4.1): nothing measured is shown yet. */
function ShowcaseHome() {
  return (
    <section data-showcase-page="overview" style={{ padding: 'var(--sp-8) var(--sp-6)' }}>
      <h1 style={{ fontSize: 'var(--text-2xl)' }}>A Cybernetic Agent Harness</h1>
      <p>Cheap models, made dependable by measured self-regulation.</p>
      <p data-not-measured="PILOT">not yet measured · planned in PILOT</p>
    </section>
  );
}

function RouteLoading() {
  return (
    <div className="route-loading progressive-reveal">
      {/* Fake nav row */}
      <div className="route-loading__nav">
        <div className="skeleton route-loading__nav-pill" />
        <div className="skeleton route-loading__nav-pill" style={{ width: 96 }} />
        <div className="skeleton route-loading__nav-pill" style={{ width: 56 }} />
      </div>

      {/* Fake header */}
      <div className="route-loading__header">
        <div className="skeleton skeleton-circle" />
        <div className="skeleton skeleton-title" />
      </div>

      {/* Fake mosaic stats */}
      <div className="route-loading__mosaic">
        {Array.from({ length: 4 }, (_, i) => (
          <div key={i} className="route-loading__mosaic-cell">
            <div className="skeleton skeleton-text" style={{ width: '50%' }} />
            <div className="skeleton skeleton-title" style={{ width: '70%' }} />
          </div>
        ))}
      </div>

      {/* Fake body lines */}
      <div className="route-loading__body">
        <div className="skeleton-card skeleton" />
        <div className="route-loading__row">
          <div className="skeleton skeleton-text" style={{ width: '40%' }} />
          <div className="skeleton skeleton-text" style={{ width: '25%' }} />
        </div>
        <div className="skeleton skeleton-text" style={{ width: '80%' }} />
        <div className="skeleton skeleton-text" style={{ width: '55%' }} />
      </div>
    </div>
  );
}

// Initialize transport layer before React render.
const cleanupTransport = bootstrapTransport();
if (import.meta.hot) {
  import.meta.hot.dispose(cleanupTransport);
}

// Derive the router basename from Vite's BASE_URL.
// In a production build (base: '/demo/') this resolves to '/demo'.
// In the dev server (base: '/') this resolves to '/', keeping Playwright specs unchanged.
const routerBasename = import.meta.env.BASE_URL.replace(/\/$/, '') || '/';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <BrowserRouter basename={routerBasename}>
      <ErrorBoundary>
        <ToastProvider>
          <Suspense fallback={<RouteLoading />}>
            <Routes>
              {FixtureRoutes && <Route path="__fixtures/*" element={<FixtureRoutes />} />}
              <Route element={<AppShell />}>
                {/* The showcase (S10 §4.2): measured claims at the home. */}
                <Route index element={<ShowcaseHome />} />
                {/* Legacy pages, local only (D23). */}
                <Route path="lab">
                  <Route index element={<Landing />} />
                  {/* AppShell keeps the scenario player mounted while this path is open. */}
                  <Route path="demo" />
                  <Route path="dashboard" element={<DashboardLayout />}>
                    <Route index element={<CostDashboard />} />
                    <Route path="fleet" element={<AgentFleet />} />
                    <Route path="knowledge" element={<KnowledgeGraph />} />
                    <Route path="integrity" element={<IntegrityView />} />
                    <Route path="entries" element={<KnowledgeEntries />} />
                    <Route path="routing" element={<CascadeRouter />} />
                    <Route path="dreams" element={<DreamsView />} />
                    <Route path="feeds" element={<FeedsDashboard />} />
                    <Route path="relay" element={<RelayDashboard />} />
                  </Route>
                  <Route path="terminal" element={<Terminal />} />
                  <Route path="builder" element={<Builder />} />
                  <Route path="explorer" element={<Explorer />} />
                  <Route path="settings" element={<Settings />} />
                  <Route path="bench" element={<Bench />} />
                  <Route path="bench/run/:id" element={<BenchRunDetail />} />
                  <Route path="bench/compare" element={<BenchCompare />} />
                  <Route path="share/:token" element={<SharePage />} />
                  <Route path="share" element={<SharePage />} />
                </Route>
                {LAB_PATHS.map((path) => (
                  <Route key={path} path={`${path}/*`} element={<ToLab />} />
                ))}
              </Route>
            </Routes>
          </Suspense>
        </ToastProvider>
      </ErrorBoundary>
    </BrowserRouter>
  </StrictMode>,
);
