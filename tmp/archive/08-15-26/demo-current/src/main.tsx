import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router';
import { AppShell } from './design/AppShell';
import { ErrorBoundary } from './design/ErrorBoundary';
import { LandingPage } from './pages/Landing';
import { OrchestratePage } from './pages/Orchestrate';
import './design/global.css';

function PlaceholderPage({ name }: { name: string }) {
  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      height: '100%',
      fontFamily: 'var(--display)',
      fontStyle: 'italic',
      fontSize: '24px',
      color: 'var(--text-soft)',
    }}>
      {name}
    </div>
  );
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <BrowserRouter>
      <Routes>
        {/* Landing page at root */}
        <Route path="/" element={<LandingPage />} />

        {/* App shell with section pages */}
        <Route path="/app" element={<AppShell />}>
          <Route index element={<Navigate to="/app/orchestrate" replace />} />
          <Route path="orchestrate" element={
            <ErrorBoundary name="Orchestrate">
              <OrchestratePage />
            </ErrorBoundary>
          } />
          <Route path="observe" element={
            <ErrorBoundary name="Observe">
              <PlaceholderPage name="Observe" />
            </ErrorBoundary>
          } />
          <Route path="observe/:section" element={
            <ErrorBoundary name="Observe">
              <PlaceholderPage name="Observe" />
            </ErrorBoundary>
          } />
          <Route path="evaluate" element={
            <ErrorBoundary name="Evaluate">
              <PlaceholderPage name="Evaluate" />
            </ErrorBoundary>
          } />
          <Route path="build" element={
            <ErrorBoundary name="Build">
              <PlaceholderPage name="Build" />
            </ErrorBoundary>
          } />
        </Route>
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  </StrictMode>,
);
