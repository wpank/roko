'use client';

import { useEffect, useState } from 'react';
import { currentTheme, pickTheme } from '@/lib/theme';
import type { Theme } from '@/lib/theme';

/** The day and night switch: it starts at the system's setting and keeps what the person picks. */
export function ThemeSwitch() {
  // The server render can't know the theme; the first effect reads it.
  const [theme, setTheme] = useState<Theme | null>(null);
  useEffect(() => setTheme(currentTheme()), []);
  const night = theme === 'dark';

  return (
    <button
      type="button"
      role="switch"
      aria-checked={night}
      aria-label="Night theme"
      data-action="theme"
      className="pt-theme pt-hide-narrow"
      onClick={() => {
        const next: Theme = night ? 'light' : 'dark';
        pickTheme(next);
        setTheme(next);
      }}
    >
      <span data-side="day">day</span>
      <i aria-hidden="true" />
      <span data-side="night">night</span>
    </button>
  );
}
