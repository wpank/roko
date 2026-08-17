import { createContext, useContext, useState, type ReactNode } from 'react';
import { DEFAULT_CONFIG, type DemoConfig } from '../pages/orchestrate/ConfigPanel';

interface ConfigContextValue {
  config: DemoConfig;
  setConfig: (config: DemoConfig) => void;
}

const ConfigContext = createContext<ConfigContextValue>({
  config: DEFAULT_CONFIG,
  setConfig: () => {},
});

export function ConfigProvider({ children }: { children: ReactNode }) {
  const [config, setConfig] = useState<DemoConfig>(DEFAULT_CONFIG);
  return (
    <ConfigContext.Provider value={{ config, setConfig }}>
      {children}
    </ConfigContext.Provider>
  );
}

export function useConfig() {
  return useContext(ConfigContext);
}
