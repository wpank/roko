import type { Metadata } from 'next';
import '@/styles/globals.css';
import { Providers } from './providers';

export const metadata: Metadata = {
  title: 'roko',
  // An empty inline icon. Without one the browser asks for /favicon.ico, which
  // the export does not have, and logs the 404 on every load.
  icons: 'data:,',
  other: { 'roko-portal': 'workspace' },
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body>
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
