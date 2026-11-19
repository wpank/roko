import type { Metadata } from 'next';
import '@/styles/globals.css';
import { THEME_SCRIPT } from '@/lib/theme';
import { Providers } from './providers';

export const metadata: Metadata = {
  title: 'roko',
  // An empty inline icon. Without one the browser asks for /favicon.ico, which
  // the export does not have, and logs the 404 on every load.
  icons: 'data:,',
  other: { 'roko-portal': 'workspace' },
};

/**
 * Day and night follow the system until the person picks one; the script in <head> puts a kept pick on <html>
 * before the first paint (lib/theme.ts). The two faces load early, since every word on the page uses them.
 */
export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <script dangerouslySetInnerHTML={{ __html: THEME_SCRIPT }} />
        <link rel="preload" href="/fonts/archivo-latin.woff2" as="font" type="font/woff2" crossOrigin="anonymous" />
        <link
          rel="preload"
          href="/fonts/martian-mono-latin.woff2"
          as="font"
          type="font/woff2"
          crossOrigin="anonymous"
        />
      </head>
      <body className="rk-page">
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
