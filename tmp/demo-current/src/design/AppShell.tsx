import { Outlet, useSearchParams } from 'react-router';
import { type CSSProperties, useState, useEffect, useCallback, useMemo } from 'react';
import { TopNav } from './TopNav';
import { HealthBar } from './HealthBar';
import { RokoApi, ApiContext } from '../data/api';
import { ConfigProvider } from '../lib/config-context';
import type { DataMode } from '../lib/types';

const shellStyle: CSSProperties = {
  minHeight: '100vh',
  display: 'flex',
  flexDirection: 'column',
};

const bodyStyle: CSSProperties = {
  marginTop: 56,
  flex: 1,
  overflow: 'auto',
  height: 'calc(100vh - 56px)',
};

export function AppShell() {
  const [params] = useSearchParams();
  const showDebug = params.get('debug') === 'true';
  const [dataMode, setDataMode] = useState<DataMode>('seed');

  const handleStatusChange = useCallback((mode: DataMode) => {
    setDataMode(mode);
    document.title = mode === 'live' ? '[LIVE] Roko' : '[SEED] Roko';
  }, []);

  const api = useMemo(() => new RokoApi(handleStatusChange), [handleStatusChange]);

  useEffect(() => {
    api.probe();
    const interval = setInterval(() => api.probe(), 30_000);
    return () => clearInterval(interval);
  }, [api]);

  return (
    <ApiContext.Provider value={api}>
      <ConfigProvider>
        <div style={shellStyle}>
          <TopNav dataMode={dataMode} />

          <div style={bodyStyle}>
            <Outlet />
          </div>

          {showDebug && <HealthBar dataMode={dataMode} />}
        </div>
      </ConfigProvider>
    </ApiContext.Provider>
  );
}
