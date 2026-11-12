/**
 * Day and night. The portal follows the system's light or dark setting until the person picks one with the switch
 * in the top bar; the pick is kept in this browser and set on <html data-theme> before the first paint.
 */

export type Theme = 'light' | 'dark';

export const THEME_KEY = 'roko.theme';

/** The theme the person picked in this browser, or null when they never did (or storage is off). */
export function pickedTheme(): Theme | null {
  try {
    const value = localStorage.getItem(THEME_KEY);
    return value === 'light' || value === 'dark' ? value : null;
  } catch {
    return null;
  }
}

/** The system's theme. */
export function systemTheme(): Theme {
  try {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  } catch {
    return 'light';
  }
}

/** The theme on screen: the person's pick, else the system's. */
export function currentTheme(): Theme {
  return pickedTheme() ?? systemTheme();
}

/** Show `theme` and keep it as the person's pick. */
export function pickTheme(theme: Theme): void {
  document.documentElement.setAttribute('data-theme', theme);
  try {
    localStorage.setItem(THEME_KEY, theme);
  } catch {
    // Storage off: the pick lasts for this page only.
  }
}

/**
 * Runs in <head> before the page paints: puts a kept pick on <html>, so a night page never flashes day. Plain
 * ES5 with no imports, since it runs before the app's code.
 */
export const THEME_SCRIPT = `try{var t=localStorage.getItem(${JSON.stringify(THEME_KEY)});if(t==="light"||t==="dark")document.documentElement.setAttribute("data-theme",t)}catch(e){}`;
