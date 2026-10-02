/**
 * Fixture-only routes for the `showcase-fixture` Playwright project (S10 §7): harnesses that
 * render showcase components from the e2e fixture bundles before the pages exist. `main.tsx`
 * mounts this module under `/__fixtures/` only in a non-production build with
 * `VITE_ALLOW_FIXTURES=1`, so none of it reaches a production bundle.
 */
import type { ReactElement } from 'react';
import { Route, Routes } from 'react-router';

export interface FixtureRoute {
  /** Path under `/__fixtures/`. */
  path: string;
  element: ReactElement;
}

/** The fixture routes. Chart harnesses are added here as the charts land. */
export const FIXTURE_ROUTES: FixtureRoute[] = [];

export default function FixtureRoutes() {
  return (
    <Routes>
      {FIXTURE_ROUTES.map((route) => (
        <Route key={route.path} path={route.path} element={route.element} />
      ))}
    </Routes>
  );
}
