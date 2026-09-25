import type { Metadata } from 'next';
import '@/styles/globals.css';
import { Providers } from './providers';
import { AppShell } from '@/components/layout/AppShell';

export const metadata: Metadata = {
  title: 'roko portal',
  description: 'Web control plane for roko',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body>
        <Providers>
          <AppShell>{children}</AppShell>
        </Providers>
      </body>
    </html>
  );
}
