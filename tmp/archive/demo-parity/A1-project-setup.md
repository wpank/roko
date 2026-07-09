# A1: Project setup -- design system, stores, router, layouts

## Context

**Repo:** `/Users/will/dev/nunchi/nunchi-dashboard`
**Branch:** `demo-rewrite`
**Tech stack:** React 19 + Vite 8 + TypeScript + Tailwind CSS v4
**Backend:** `roko-serve` runs at `http://localhost:6677` with ~85 REST routes + WebSocket at `ws://localhost:6677/ws`
**Auth:** Privy (env var `VITE_PRIVY_APP_ID`) with password fallback
**Design:** ROSEDUST dark palette -- bg_void `#060608`, rose `#AA7088`, bone `#C8B890`, rose_bright `#CC90A8`

### Before starting
1. `cd /Users/will/dev/nunchi/nunchi-dashboard`
2. `git checkout -b demo-rewrite 2>/dev/null || git checkout demo-rewrite`
3. `npm install`
4. Verify: `npm run dev` starts without errors

### After every task
1. `npm run typecheck` passes
2. `npm run dev` -- page renders without console errors
3. All existing tests pass: `npm test` (if test runner is configured)

---

## What this task produces

The foundational scaffolding that every subsequent task depends on: design tokens, a component library, state management, routing, and the two-layout shell (landing + app). When this task is done, `npm run dev` renders the AppLayout chrome at `/app/chat` and the landing page at `/`.

---

## Checklist

### 1. Install dependencies

- [ ] Run:
```bash
cd /Users/will/dev/nunchi/nunchi-dashboard
npm install react-router-dom@6 zustand @tanstack/react-query
```

### 2. Create `.env.local`

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/.env.local`:
```env
VITE_ROKO_API_URL=http://localhost:6677
VITE_ROKO_WS_URL=ws://localhost:6677/ws
VITE_PRIVY_APP_ID=
```

### 3. Add Vite proxy

- [ ] Replace `/Users/will/dev/nunchi/nunchi-dashboard/vite.config.ts` (rename from `.js` if needed):

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: {
      "/api": {
        target: "http://localhost:6677",
        changeOrigin: true,
      },
      "/ws": {
        target: "ws://localhost:6677",
        ws: true,
      },
    },
  },
});
```

### 4. Create design tokens

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/tokens.css`:

```css
/*
 * ROSEDUST design tokens
 *
 * Single source of truth for every color, spacing value, radius, and
 * transition duration in the dashboard. Components consume them via
 * var(--rd-*). The "rd" prefix avoids collisions with existing CSS
 * variables in index.css.
 *
 * Categories:
 *   Backgrounds  — surface hierarchy (void → surface-3)
 *   Foregrounds  — text hierarchy (primary → muted)
 *   Accents      — rose, bone, purple, gold
 *   Semantic     — success, warning, error
 *   Spacing      — 4px base scale (sp-1 through sp-8)
 *   Radius       — sm / md / lg
 *   Transitions  — fast / normal / slow / breathe
 */

:root {
  /* ── Backgrounds ─────────────────────────────────────────────────── */
  --rd-bg-void:      #060608;  /* page background */
  --rd-bg-surface-0: #0C0C10;  /* subtle surface */
  --rd-bg-surface-1: #141418;  /* card */
  --rd-bg-surface-2: #1C1C22;  /* inner fill */
  --rd-bg-surface-3: #24242C;  /* border / separator */

  /* ── Foregrounds ─────────────────────────────────────────────────── */
  --rd-fg-primary:   #E8E4DE;  /* headings, active labels */
  --rd-fg-secondary: #9B9590;  /* body text */
  --rd-fg-muted:     #6B655F;  /* captions, placeholders */

  /* ── Accent: rose ────────────────────────────────────────────────── */
  --rd-rose:         #AA7088;  /* primary CTA, active nav */
  --rd-rose-bright:  #CC90A8;  /* hover, highlighted value */
  --rd-rose-dim:     #6A4858;  /* gradient terminus, pressed */

  /* ── Accent: bone ────────────────────────────────────────────────── */
  --rd-bone:         #C8B890;

  /* ── Accent: purple ──────────────────────────────────────────────── */
  --rd-accent-purple: #7A6890;

  /* ── Accent: gold ────────────────────────────────────────────────── */
  --rd-accent-gold:  #C8A855;

  /* ── Semantic ────────────────────────────────────────────────────── */
  --rd-success: #70887A;
  --rd-warning: #AA8855;
  --rd-error:   #AA5555;

  /* ── Spacing (4px base grid) ─────────────────────────────────────── */
  --rd-sp-1:  4px;
  --rd-sp-2:  8px;
  --rd-sp-3: 12px;
  --rd-sp-4: 16px;
  --rd-sp-5: 24px;
  --rd-sp-6: 32px;
  --rd-sp-7: 48px;
  --rd-sp-8: 64px;

  /* ── Radius ──────────────────────────────────────────────────────── */
  --rd-radius-sm:  4px;
  --rd-radius-md:  8px;
  --rd-radius-lg: 12px;

  /* ── Transitions ─────────────────────────────────────────────────── */
  --rd-transition-fast:    150ms ease;
  --rd-transition-normal:  300ms ease;
  --rd-transition-slow:    500ms ease;
  --rd-transition-breathe: 6000ms ease-in-out;
}
```

- [ ] Import the tokens at the top of `/Users/will/dev/nunchi/nunchi-dashboard/src/index.css` by adding `@import "./design-system/tokens.css";` as the second line (after the Google Fonts import and before `@import "tailwindcss"`).

### 5. Create design system utilities

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/cn.ts`:

```ts
/**
 * Lightweight classname merger compatible with Tailwind v4.
 *
 * Filters falsy values and joins with a space. For conditional classes,
 * pass them as entries in an object: `cn("base", { "active": isActive })`.
 * Avoids the clsx / tailwind-merge bundle overhead for the majority of
 * call-sites that only need falsy filtering.
 */
export function cn(
  ...inputs: Array<string | undefined | null | false | Record<string, boolean>>
): string {
  const parts: string[] = [];
  for (const input of inputs) {
    if (!input) continue;
    if (typeof input === "string") {
      parts.push(input);
    } else {
      for (const [cls, active] of Object.entries(input)) {
        if (active) parts.push(cls);
      }
    }
  }
  return parts.join(" ");
}
```

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/useMediaQuery.ts`:

```ts
import { useState, useEffect } from "react";

/**
 * Returns true while the given media query matches.
 * Cleans up the listener on unmount.
 *
 * @example
 *   const isMobile = useMediaQuery("(max-width: 768px)");
 *   const prefersReduced = useMediaQuery("(prefers-reduced-motion: reduce)");
 */
export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState<boolean>(() => {
    if (typeof window === "undefined") return false;
    return window.matchMedia(query).matches;
  });

  useEffect(() => {
    const mql = window.matchMedia(query);
    const handler = (e: MediaQueryListEvent) => setMatches(e.matches);
    mql.addEventListener("change", handler);
    return () => mql.removeEventListener("change", handler);
  }, [query]);

  return matches;
}

// ── Common breakpoint shortcuts ─────────────────────────────────────────────

export const BREAKPOINTS = {
  sm:  "(min-width: 640px)",
  md:  "(min-width: 768px)",
  lg:  "(min-width: 1024px)",
  xl:  "(min-width: 1280px)",
  "2xl": "(min-width: 1536px)",
  reducedMotion: "(prefers-reduced-motion: reduce)",
} as const;
```

### 6. Create design system components

Create the directory:
```bash
mkdir -p /Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components
```

#### 6a. Card

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Card.tsx`:

```tsx
import type { ReactNode } from "react";
import { cn } from "../cn";

const PADDING = {
  sm: "p-3",
  md: "p-4",
  lg: "p-6",
} as const;

type CardProps = {
  children: ReactNode;
  className?: string;
  padding?: keyof typeof PADDING;
};

export function Card({ children, className, padding = "md" }: CardProps) {
  return (
    <div
      className={cn(
        "bg-[var(--rd-bg-surface-1)] border border-[var(--rd-bg-surface-3)] rounded-lg",
        PADDING[padding],
        className
      )}
    >
      {children}
    </div>
  );
}
```

#### 6b. Badge

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Badge.tsx`:

```tsx
import { cn } from "../cn";

const VARIANTS = {
  default: "bg-[var(--rd-bg-surface-2)] text-[var(--rd-fg-secondary)]",
  success: "bg-[var(--rd-success)]/15 text-[var(--rd-success)]",
  warning: "bg-[var(--rd-warning)]/15 text-[var(--rd-warning)]",
  error:   "bg-[var(--rd-error)]/15 text-[var(--rd-error)]",
  info:    "bg-[var(--rd-accent-purple)]/15 text-[var(--rd-accent-purple)]",
  rose:    "bg-[var(--rd-rose)]/15 text-[var(--rd-rose-bright)]",
} as const;

type BadgeProps = {
  label: string;
  variant?: keyof typeof VARIANTS;
};

export function Badge({ label, variant = "default" }: BadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center px-2 py-0.5 rounded-full font-mono text-xs leading-tight",
        VARIANTS[variant]
      )}
    >
      {label}
    </span>
  );
}
```

#### 6c. Button

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Button.tsx`:

```tsx
import type { ReactNode, ButtonHTMLAttributes } from "react";
import { cn } from "../cn";

const VARIANT_STYLES = {
  primary:
    "bg-[var(--rd-rose)] text-white hover:bg-[var(--rd-rose-bright)] active:bg-[var(--rd-rose-dim)] focus-visible:ring-[var(--rd-rose)]",
  secondary:
    "bg-[var(--rd-bg-surface-2)] text-[var(--rd-fg-primary)] border border-[var(--rd-bg-surface-3)] hover:bg-[var(--rd-bg-surface-3)] focus-visible:ring-[var(--rd-fg-muted)]",
  ghost:
    "text-[var(--rd-fg-secondary)] hover:bg-[var(--rd-bg-surface-2)] hover:text-[var(--rd-fg-primary)] focus-visible:ring-[var(--rd-fg-muted)]",
  danger:
    "bg-[var(--rd-error)]/15 text-[var(--rd-error)] hover:bg-[var(--rd-error)]/25 focus-visible:ring-[var(--rd-error)]",
} as const;

const SIZE_STYLES = {
  sm: "px-2.5 py-1 text-xs rounded-md",
  md: "px-4 py-2 text-sm rounded-lg",
  lg: "px-6 py-3 text-base rounded-lg",
} as const;

type ButtonProps = {
  children: ReactNode;
  variant?: keyof typeof VARIANT_STYLES;
  size?: keyof typeof SIZE_STYLES;
  disabled?: boolean;
  loading?: boolean;
  onClick?: () => void;
} & Omit<ButtonHTMLAttributes<HTMLButtonElement>, "onClick">;

export function Button({
  children,
  variant = "primary",
  size = "md",
  disabled = false,
  loading = false,
  onClick,
  ...rest
}: ButtonProps) {
  return (
    <button
      onClick={onClick}
      disabled={disabled || loading}
      className={cn(
        "inline-flex items-center justify-center font-medium",
        "transition-[background-color,color,opacity] duration-[var(--rd-transition-fast)]",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--rd-bg-void)]",
        "disabled:opacity-50 disabled:cursor-not-allowed",
        VARIANT_STYLES[variant],
        SIZE_STYLES[size]
      )}
      {...rest}
    >
      {loading && (
        <svg
          className="animate-spin -ml-1 mr-2 h-4 w-4"
          fill="none"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <circle
            className="opacity-25"
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            strokeWidth="4"
          />
          <path
            className="opacity-75"
            fill="currentColor"
            d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
          />
        </svg>
      )}
      {children}
    </button>
  );
}
```

#### 6d. Input

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Input.tsx`:

```tsx
import { useId } from "react";
import { cn } from "../cn";

type InputProps = {
  label?: string;
  placeholder?: string;
  value: string;
  onChange: (value: string) => void;
  error?: string;
  type?: "text" | "number" | "textarea";
};

export function Input({
  label,
  placeholder,
  value,
  onChange,
  error,
  type = "text",
}: InputProps) {
  const id = useId();

  const base = cn(
    "w-full bg-[var(--rd-bg-surface-0)] border text-[var(--rd-fg-primary)]",
    "placeholder-[var(--rd-fg-muted)] rounded-lg px-3 py-2 text-sm",
    "transition-[border-color,box-shadow] duration-[var(--rd-transition-fast)]",
    "focus:outline-none focus:ring-1",
    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--rd-bg-void)]"
  );

  const borderClass = error
    ? "border-[var(--rd-error)] focus:ring-[var(--rd-error)] focus-visible:ring-[var(--rd-error)]"
    : "border-[var(--rd-bg-surface-3)] focus:ring-[var(--rd-rose)] focus-visible:ring-[var(--rd-rose)] hover:border-[var(--rd-fg-muted)]";

  return (
    <div>
      {label && (
        <label
          htmlFor={id}
          className="block text-xs font-medium text-[var(--rd-fg-secondary)] mb-1.5"
        >
          {label}
        </label>
      )}
      {type === "textarea" ? (
        <textarea
          id={id}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          rows={4}
          className={cn(base, borderClass, "resize-y")}
        />
      ) : (
        <input
          id={id}
          type={type}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          className={cn(base, borderClass)}
        />
      )}
      {error && (
        <p role="alert" className="mt-1 text-xs text-[var(--rd-error)]">
          {error}
        </p>
      )}
    </div>
  );
}
```

#### 6e. Select

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Select.tsx`:

```tsx
import { useId } from "react";

type SelectOption = { value: string; label: string };

type SelectProps = {
  label?: string;
  value: string;
  onChange: (value: string) => void;
  options: SelectOption[];
};

export function Select({ label, value, onChange, options }: SelectProps) {
  const id = useId();

  return (
    <div>
      {label && (
        <label
          htmlFor={id}
          className="block text-xs font-medium text-[var(--rd-fg-secondary)] mb-1.5"
        >
          {label}
        </label>
      )}
      <select
        id={id}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className={[
          "w-full bg-[var(--rd-bg-surface-0)] border border-[var(--rd-bg-surface-3)]",
          "text-[var(--rd-fg-primary)] rounded-lg px-3 py-2 text-sm",
          "transition-[border-color] duration-[var(--rd-transition-fast)]",
          "hover:border-[var(--rd-fg-muted)]",
          "focus:outline-none focus:ring-1 focus:ring-[var(--rd-rose)]",
          "focus-visible:ring-2 focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--rd-bg-void)]",
          "appearance-none cursor-pointer",
        ].join(" ")}
      >
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </div>
  );
}
```

#### 6f. Gauge

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Gauge.tsx`:

```tsx
import { cn } from "../cn";

function fillColor(value: number): string {
  if (value > 0.7) return "var(--rd-success)";
  if (value >= 0.4) return "var(--rd-warning)";
  return "var(--rd-error)";
}

type GaugeProps = {
  /** 0–1 */
  value: number;
  label?: string;
  size?: "sm" | "md";
};

export function Gauge({ value, label, size = "md" }: GaugeProps) {
  const clamped = Math.max(0, Math.min(1, value));
  const heightClass = size === "sm" ? "h-1.5" : "h-2.5";

  return (
    <div className="w-full">
      {label && (
        <div className="flex items-center justify-between mb-1">
          <span className="text-xs text-[var(--rd-fg-secondary)]">{label}</span>
          <span className="text-xs font-mono text-[var(--rd-fg-muted)]">
            {Math.round(clamped * 100)}%
          </span>
        </div>
      )}
      <div
        role="progressbar"
        aria-valuenow={Math.round(clamped * 100)}
        aria-valuemin={0}
        aria-valuemax={100}
        className={cn(
          "w-full bg-[var(--rd-bg-surface-2)] rounded-full overflow-hidden",
          heightClass
        )}
      >
        <div
          className={cn(
            "rounded-full",
            "transition-[width] duration-[var(--rd-transition-slow)] ease-out",
            "will-change-[width]",
            heightClass
          )}
          style={{
            width: `${clamped * 100}%`,
            backgroundColor: fillColor(clamped),
          }}
        />
      </div>
    </div>
  );
}
```

#### 6g. Sparkline

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Sparkline.tsx`:

```tsx
type SparklineProps = {
  data: number[];
  color?: string;
};

/**
 * Responsive SVG sparkline. Uses a fixed internal coordinate space (80×24)
 * via viewBox — the rendered size is determined by the parent container.
 * Pass `className` on a wrapping element to control dimensions.
 */
export function Sparkline({
  data,
  color = "var(--rd-rose-bright)",
}: SparklineProps) {
  if (data.length < 2) return null;

  const VW = 80;
  const VH = 24;
  const PAD = 2;

  const min = Math.min(...data);
  const max = Math.max(...data);
  const range = max - min || 1;

  const points = data
    .map((v, i) => {
      const x = (i / (data.length - 1)) * (VW - PAD * 2) + PAD;
      const y = VH - PAD - ((v - min) / range) * (VH - PAD * 2);
      return `${x},${y}`;
    })
    .join(" ");

  return (
    <svg
      viewBox={`0 0 ${VW} ${VH}`}
      preserveAspectRatio="none"
      className="w-full h-full overflow-visible"
      aria-hidden="true"
    >
      <polyline
        points={points}
        fill="none"
        stroke={color}
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
```

#### 6h. Modal

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Modal.tsx`:

```tsx
import { useEffect, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { cn } from "../cn";

const SIZE_CLASSES = {
  sm:   "max-w-sm",
  md:   "max-w-lg",
  lg:   "max-w-2xl",
  full: "max-w-5xl",
} as const;

type ModalProps = {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  size?: keyof typeof SIZE_CLASSES;
};

export function Modal({
  isOpen,
  onClose,
  title,
  children,
  size = "md",
}: ModalProps) {
  // Escape key handler — cleans up on close or unmount
  useEffect(() => {
    if (!isOpen) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [isOpen, onClose]);

  // Lock body scroll while open
  useEffect(() => {
    if (!isOpen) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = prev;
    };
  }, [isOpen]);

  if (!isOpen) return null;

  return createPortal(
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-title"
      className="fixed inset-0 z-50 flex items-center justify-center"
    >
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/60 backdrop-blur-sm"
        onClick={onClose}
        aria-hidden="true"
      />
      {/* Panel */}
      <div
        className={cn(
          "relative w-full mx-4 bg-[var(--rd-bg-surface-1)] border border-[var(--rd-bg-surface-3)] rounded-xl shadow-2xl",
          SIZE_CLASSES[size]
        )}
      >
        <div className="flex items-center justify-between px-6 py-4 border-b border-[var(--rd-bg-surface-3)]">
          <h2
            id="modal-title"
            className="text-sm font-semibold text-[var(--rd-fg-primary)]"
          >
            {title}
          </h2>
          <button
            onClick={onClose}
            aria-label="Close"
            className={[
              "text-[var(--rd-fg-muted)] hover:text-[var(--rd-fg-primary)]",
              "transition-colors duration-[var(--rd-transition-fast)]",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--rd-rose)] rounded",
              "leading-none p-1",
            ].join(" ")}
          >
            <svg viewBox="0 0 16 16" className="w-4 h-4" fill="currentColor" aria-hidden="true">
              <path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.75.75 0 1 1 1.06 1.06L9.06 8l3.22 3.22a.75.75 0 1 1-1.06 1.06L8 9.06l-3.22 3.22a.75.75 0 0 1-1.06-1.06L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06z" />
            </svg>
          </button>
        </div>
        <div className="p-6">{children}</div>
      </div>
    </div>,
    document.body
  );
}
```

#### 6i. Toast

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Toast.tsx`:

```tsx
import {
  useState,
  useCallback,
  useRef,
  createContext,
  useContext,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { cn } from "../cn";

// ─── Types ───────────────────────────────────────────────────────────────────

type ToastVariant = "success" | "error" | "warning" | "info";

type ToastItem = {
  id: number;
  message: string;
  variant: ToastVariant;
};

type ToastContextValue = {
  toast: (message: string, variant?: ToastVariant) => void;
};

// ─── Context ─────────────────────────────────────────────────────────────────

const ToastContext = createContext<ToastContextValue>({ toast: () => {} });

export function useToast(): ToastContextValue {
  return useContext(ToastContext);
}

// ─── Styles ──────────────────────────────────────────────────────────────────

const VARIANT_STYLES: Record<ToastVariant, string> = {
  success: "border-[var(--rd-success)] text-[var(--rd-success)]",
  error:   "border-[var(--rd-error)] text-[var(--rd-error)]",
  warning: "border-[var(--rd-warning)] text-[var(--rd-warning)]",
  info:    "border-[var(--rd-accent-purple)] text-[var(--rd-accent-purple)]",
};

// ─── Provider (renders via portal so it is always on top) ────────────────────

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const counterRef = useRef(0);

  const toast = useCallback(
    (message: string, variant: ToastVariant = "info") => {
      const id = ++counterRef.current;
      setToasts((prev) => [...prev, { id, message, variant }]);
      setTimeout(() => {
        setToasts((prev) => prev.filter((t) => t.id !== id));
      }, 4000);
    },
    []
  );

  return (
    <ToastContext.Provider value={{ toast }}>
      {children}
      {createPortal(
        <div
          aria-live="polite"
          aria-atomic="false"
          className="fixed bottom-6 right-6 z-[100] flex flex-col gap-2 pointer-events-none"
        >
          {toasts.map((t) => (
            <div
              key={t.id}
              role="status"
              className={cn(
                "pointer-events-auto px-4 py-2.5 rounded-lg",
                "bg-[var(--rd-bg-surface-1)] border text-sm shadow-lg",
                "animate-slide-in",
                "will-change-transform",
                VARIANT_STYLES[t.variant]
              )}
            >
              {t.message}
            </div>
          ))}
        </div>,
        document.body
      )}
    </ToastContext.Provider>
  );
}
```

#### 6j. Skeleton

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/Skeleton.tsx`:

```tsx
import { cn } from "../cn";

type SkeletonProps = {
  width?: string;
  height?: string;
  rounded?: boolean;
  className?: string;
};

export function Skeleton({
  width = "100%",
  height = "1rem",
  rounded = false,
  className,
}: SkeletonProps) {
  return (
    <div
      aria-hidden="true"
      className={cn(
        "animate-pulse bg-[var(--rd-bg-surface-2)]",
        rounded ? "rounded-full" : "rounded-md",
        className
      )}
      style={{ width, height }}
    />
  );
}
```

#### 6k. StatusDot

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/StatusDot.tsx`:

```tsx
import { cn } from "../cn";

const STATUS_STYLES = {
  online:  { dot: "bg-[var(--rd-success)]",    ring: true  },
  offline: { dot: "bg-[var(--rd-fg-muted)]",   ring: false },
  warning: { dot: "bg-[var(--rd-warning)]",    ring: false },
  error:   { dot: "bg-[var(--rd-error)]",      ring: false },
} as const;

type StatusDotProps = {
  status: keyof typeof STATUS_STYLES;
};

export function StatusDot({ status }: StatusDotProps) {
  const { dot, ring } = STATUS_STYLES[status];

  return (
    <span
      role="img"
      aria-label={status}
      className="relative inline-flex h-2.5 w-2.5"
    >
      {ring && (
        <span
          className={cn(
            "absolute inline-flex h-full w-full rounded-full opacity-75",
            dot,
            "animate-ping"
          )}
        />
      )}
      <span className={cn("relative inline-flex h-2.5 w-2.5 rounded-full", dot)} />
    </span>
  );
}
```

#### 6l. EmptyState

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/EmptyState.tsx`:

```tsx
import { Button } from "./Button";

type EmptyStateProps = {
  title: string;
  description: string;
  action?: { label: string; onClick: () => void };
};

export function EmptyState({ title, description, action }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center py-16 text-center">
      <div className="w-12 h-12 rounded-full bg-[var(--rd-bg-surface-2)] flex items-center justify-center mb-4">
        <svg
          className="w-6 h-6 text-[var(--rd-fg-muted)]"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          aria-hidden="true"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={1.5}
            d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"
          />
        </svg>
      </div>
      <h3 className="text-sm font-medium text-[var(--rd-fg-primary)] mb-1">
        {title}
      </h3>
      <p className="text-xs text-[var(--rd-fg-muted)] max-w-xs mb-4">
        {description}
      </p>
      {action && (
        <Button size="sm" onClick={action.onClick}>
          {action.label}
        </Button>
      )}
    </div>
  );
}
```

#### 6m. ErrorState

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/ErrorState.tsx`:

```tsx
import { Button } from "./Button";

type ErrorStateProps = {
  /** User-friendly message. Keep it brief — avoid exposing raw API errors. */
  message?: string;
  onRetry?: () => void;
};

export function ErrorState({
  message = "Something went wrong. Please try again.",
  onRetry,
}: ErrorStateProps) {
  return (
    <div className="flex flex-col items-center justify-center py-16 text-center">
      <div className="w-12 h-12 rounded-full bg-[var(--rd-error)]/15 flex items-center justify-center mb-4">
        <svg
          className="w-6 h-6 text-[var(--rd-error)]"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          aria-hidden="true"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={1.5}
            d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.834-2.694-.834-3.464 0L3.34 16.5c-.77.833.192 2.5 1.732 2.5z"
          />
        </svg>
      </div>
      <h3 className="text-sm font-medium text-[var(--rd-fg-primary)] mb-1">
        Something went wrong
      </h3>
      <p className="text-xs text-[var(--rd-fg-muted)] max-w-xs mb-4">{message}</p>
      {onRetry && (
        <Button variant="secondary" size="sm" onClick={onRetry}>
          Try again
        </Button>
      )}
    </div>
  );
}
```

#### 6n. Barrel export

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/components/index.ts`:

```ts
export { Card } from "./Card";
export { Badge } from "./Badge";
export { Button } from "./Button";
export { Input } from "./Input";
export { Select } from "./Select";
export { Gauge } from "./Gauge";
export { Sparkline } from "./Sparkline";
export { Modal } from "./Modal";
export { ToastProvider, useToast } from "./Toast";
export { Skeleton } from "./Skeleton";
export { StatusDot } from "./StatusDot";
export { EmptyState } from "./EmptyState";
export { ErrorState } from "./ErrorState";
```

Also export utilities from the design system root:

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/design-system/index.ts`:

```ts
export * from "./components";
export { cn } from "./cn";
export { useMediaQuery, BREAKPOINTS } from "./useMediaQuery";
```

### 7. Create Zustand stores

```bash
mkdir -p /Users/will/dev/nunchi/nunchi-dashboard/src/stores
```

#### 7a. authStore

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/stores/authStore.ts`:

```ts
import { create } from "zustand";

const STORAGE_KEY = "nunchi_auth_token";

type User = {
  id: string;
  email?: string;
  address?: string;
};

type AuthState = {
  user: User | null;
  token: string | null;
  isAuthenticated: boolean;
  login: (token: string, user?: User) => void;
  logout: () => void;
};

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  token: localStorage.getItem(STORAGE_KEY),
  isAuthenticated: !!localStorage.getItem(STORAGE_KEY),

  login: (token, user) => {
    localStorage.setItem(STORAGE_KEY, token);
    set({ token, user: user ?? null, isAuthenticated: true });
  },

  logout: () => {
    localStorage.removeItem(STORAGE_KEY);
    set({ token: null, user: null, isAuthenticated: false });
  },
}));
```

#### 7b. wsStore

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/stores/wsStore.ts`:

```ts
import { create } from "zustand";

const RING_BUFFER_SIZE = 200;

export type WsEvent = {
  type: string;
  payload: unknown;
  receivedAt: number;
};

type WsState = {
  connected: boolean;
  events: WsEvent[];
  lastEventAt: number | null;
  setConnected: (connected: boolean) => void;
  pushEvent: (event: WsEvent) => void;
  clearEvents: () => void;
};

export const useWsStore = create<WsState>((set) => ({
  connected: false,
  events: [],
  lastEventAt: null,

  setConnected: (connected) => set({ connected }),

  pushEvent: (event) =>
    set((state) => {
      const next = [...state.events, event];
      // Trim the ring buffer from the front to avoid unbounded growth
      if (next.length > RING_BUFFER_SIZE) {
        next.splice(0, next.length - RING_BUFFER_SIZE);
      }
      return { events: next, lastEventAt: event.receivedAt };
    }),

  clearEvents: () => set({ events: [], lastEventAt: null }),
}));
```

#### 7c. uiStore

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/stores/uiStore.ts`:

```ts
import { create } from "zustand";

type UiState = {
  sidebarCollapsed: boolean;
  rightPanelVisible: boolean;
  activeModal: string | null;
  /** Always "dark" — light theme is not planned. */
  theme: "dark";
  toggleSidebar: () => void;
  toggleRightPanel: () => void;
  openModal: (id: string) => void;
  closeModal: () => void;
};

export const useUiStore = create<UiState>((set) => ({
  sidebarCollapsed: false,
  rightPanelVisible: true,
  activeModal: null,
  theme: "dark",

  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  toggleRightPanel: () => set((s) => ({ rightPanelVisible: !s.rightPanelVisible })),
  openModal: (id) => set({ activeModal: id }),
  closeModal: () => set({ activeModal: null }),
}));
```

### 8. Create router

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/router.tsx`:

```tsx
import { createBrowserRouter, Navigate } from "react-router-dom";
import { lazy, Suspense, type ComponentType } from "react";
import { Skeleton } from "./design-system/components";

// Layouts
import { AppLayout } from "./layouts/AppLayout";
import { LandingLayout } from "./layouts/LandingLayout";

// ── Lazy-load helper ──────────────────────────────────────────────────────────
// Wraps a dynamic import in a full-height Suspense skeleton fallback.

function lazyPage(factory: () => Promise<{ default: ComponentType }>) {
  const Component = lazy(factory);
  return (
    <Suspense
      fallback={
        <div className="flex flex-col gap-3 p-8">
          <Skeleton height="1.5rem" width="40%" />
          <Skeleton height="1rem" width="70%" />
          <Skeleton height="1rem" width="55%" />
        </div>
      }
    >
      <Component />
    </Suspense>
  );
}

// ── Placeholder ───────────────────────────────────────────────────────────────
// Used for pages that do not yet have their own file.
// Tasks A3–A7 replace these with real implementations.

function Placeholder({ name }: { name: string }) {
  return (
    <div className="p-8">
      <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)] mb-2">
        {name}
      </h1>
      <p className="text-sm text-[var(--rd-fg-muted)]">
        Implemented in a later task.
      </p>
    </div>
  );
}

const Landing = lazy(() => import("./pages/Landing"));

export const router = createBrowserRouter([
  {
    path: "/",
    element: (
      <LandingLayout>
        <Suspense fallback={null}>
          <Landing />
        </Suspense>
      </LandingLayout>
    ),
  },
  {
    path: "/app",
    element: <AppLayout />,
    children: [
      { index: true, element: <Navigate to="chat" replace /> },

      // Command
      { path: "chat",     element: <Placeholder name="Chat" /> },
      { path: "research", element: <Placeholder name="Research" /> },

      // Observatory
      { path: "observatory/agents",    element: <Placeholder name="Live agents" /> },
      { path: "observatory/plans",     element: <Placeholder name="Plans" /> },
      { path: "observatory/learning",  element: <Placeholder name="Learning" /> },
      { path: "observatory/conductor", element: <Placeholder name="Conductor" /> },
      { path: "observatory/costs",     element: <Placeholder name="Costs" /> },

      // Network
      { path: "network/agents",     element: <Placeholder name="Agent network" /> },
      { path: "network/pheromones", element: <Placeholder name="Pheromone field" /> },
      { path: "network/knowledge",  element: <Placeholder name="Knowledge graph" /> },

      // Marketplace
      { path: "marketplace",        element: <Placeholder name="Job board" /> },
      { path: "marketplace/create", element: <Placeholder name="Create job" /> },
      { path: "marketplace/:id",    element: <Placeholder name="Job detail" /> },

      // Agent Studio
      { path: "studio/overview",  element: <Placeholder name="Agent overview" /> },
      { path: "studio/strategy",  element: <Placeholder name="Agent strategy" /> },
      { path: "studio/keys",      element: <Placeholder name="Agent keys" /> },
      { path: "studio/deploy",    element: <Placeholder name="Agent deploy" /> },

      // Atelier
      { path: "atelier",           element: <Placeholder name="Atelier" /> },
      { path: "atelier/prds",      element: <Placeholder name="PRD browser" /> },
      { path: "atelier/execution", element: <Placeholder name="Execution monitor" /> },

      // Settings
      { path: "settings", element: <Placeholder name="Settings" /> },
    ],
  },
]);
```

### 9. Create layouts

#### 9a. AppLayout

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/layouts/AppLayout.tsx`:

```tsx
import { Outlet, NavLink, useLocation } from "react-router-dom";
import { useUiStore } from "../stores/uiStore";
import { cn } from "../design-system/cn";

type NavItem = { to: string; label: string; icon: string };
type NavSection = { title: string; items: NavItem[] };

const NAV_SECTIONS: NavSection[] = [
  {
    title: "Command",
    items: [
      { to: "/app/chat",     label: "Chat",     icon: "chat" },
      { to: "/app/research", label: "Research", icon: "science" },
    ],
  },
  {
    title: "Observatory",
    items: [
      { to: "/app/observatory/agents",    label: "Live agents", icon: "groups" },
      { to: "/app/observatory/plans",     label: "Plans",       icon: "assignment" },
      { to: "/app/observatory/learning",  label: "Learning",    icon: "school" },
      { to: "/app/observatory/conductor", label: "Conductor",   icon: "tune" },
      { to: "/app/observatory/costs",     label: "Costs",       icon: "payments" },
    ],
  },
  {
    title: "Network",
    items: [
      { to: "/app/network/agents",     label: "Agent network", icon: "hub" },
      { to: "/app/network/pheromones", label: "Pheromones",   icon: "blur_on" },
      { to: "/app/network/knowledge",  label: "Knowledge",    icon: "auto_stories" },
    ],
  },
  {
    title: "Marketplace",
    items: [
      { to: "/app/marketplace", label: "Job board", icon: "storefront" },
    ],
  },
  {
    title: "Agent Studio",
    items: [
      { to: "/app/studio/overview",  label: "Overview",  icon: "smart_toy" },
      { to: "/app/studio/strategy",  label: "Strategy",  icon: "psychology" },
      { to: "/app/studio/keys",      label: "Keys",      icon: "key" },
      { to: "/app/studio/deploy",    label: "Deploy",    icon: "rocket_launch" },
    ],
  },
  {
    title: "Atelier",
    items: [
      { to: "/app/atelier",           label: "Dashboard",  icon: "dashboard" },
      { to: "/app/atelier/prds",      label: "PRDs",       icon: "description" },
      { to: "/app/atelier/execution", label: "Execution",  icon: "play_arrow" },
    ],
  },
];

export function AppLayout() {
  const { sidebarCollapsed, rightPanelVisible, toggleSidebar } = useUiStore();
  const location = useLocation();

  // Build breadcrumb from path segments
  const segments = location.pathname
    .replace("/app/", "")
    .split("/")
    .filter(Boolean);
  const breadcrumb = segments.map(
    (s) => s.charAt(0).toUpperCase() + s.slice(1).replace(/-/g, " ")
  );

  const navWidth = sidebarCollapsed ? 60 : 240;

  return (
    <div className="min-h-screen bg-[var(--rd-bg-void)] text-[var(--rd-fg-primary)]">
      {/* ── Left nav ──────────────────────────────────────────────────── */}
      <aside
        style={{ width: navWidth }}
        className={cn(
          "fixed left-0 top-0 h-screen",
          "bg-[var(--rd-bg-surface-0)] border-r border-[var(--rd-bg-surface-2)]",
          "flex flex-col z-40 overflow-y-auto overflow-x-hidden",
          "transition-[width] duration-[var(--rd-transition-normal)]"
        )}
      >
        {/* Logo */}
        <div className="flex items-center gap-2 px-4 h-14 shrink-0">
          <div className="w-7 h-7 shrink-0 rounded-lg bg-gradient-to-br from-[var(--rd-rose)] to-[var(--rd-rose-dim)] flex items-center justify-center text-white text-xs font-bold">
            N
          </div>
          {!sidebarCollapsed && (
            <span className="text-sm font-semibold text-[var(--rd-fg-primary)] tracking-tight truncate">
              Nunchi
            </span>
          )}
        </div>

        {/* Nav sections */}
        <nav className="flex-1 px-2 py-2 space-y-4" aria-label="Main navigation">
          {NAV_SECTIONS.map((section) => (
            <div key={section.title}>
              {!sidebarCollapsed && (
                <div className="px-2 mb-1 text-[10px] font-medium uppercase tracking-wider text-[var(--rd-fg-muted)]">
                  {section.title}
                </div>
              )}
              <div className="space-y-0.5">
                {section.items.map((item) => (
                  <NavLink
                    key={item.to}
                    to={item.to}
                    end={item.to === "/app/marketplace"}
                    title={sidebarCollapsed ? item.label : undefined}
                    className={({ isActive }) =>
                      cn(
                        "flex items-center gap-2.5 px-2.5 py-1.5 rounded-md text-sm",
                        "transition-colors duration-[var(--rd-transition-fast)]",
                        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--rd-rose)]",
                        isActive
                          ? "bg-[var(--rd-rose)]/10 text-[var(--rd-rose-bright)] font-medium"
                          : "text-[var(--rd-fg-secondary)] hover:bg-[var(--rd-bg-surface-1)] hover:text-[var(--rd-fg-primary)]"
                      )
                    }
                  >
                    <span className="material-symbols-outlined text-[18px] shrink-0">
                      {item.icon}
                    </span>
                    {!sidebarCollapsed && <span className="truncate">{item.label}</span>}
                  </NavLink>
                ))}
              </div>
            </div>
          ))}
        </nav>

        {/* Collapse toggle */}
        <button
          onClick={toggleSidebar}
          aria-label={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
          className={cn(
            "mx-2 mb-3 p-2 rounded-md",
            "text-[var(--rd-fg-muted)] hover:text-[var(--rd-fg-secondary)] hover:bg-[var(--rd-bg-surface-1)]",
            "transition-colors duration-[var(--rd-transition-fast)]",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--rd-rose)]"
          )}
        >
          <span className="material-symbols-outlined text-[18px]">
            {sidebarCollapsed ? "chevron_right" : "chevron_left"}
          </span>
        </button>

        {/* Settings link */}
        <div className="px-2 pb-3">
          <NavLink
            to="/app/settings"
            title={sidebarCollapsed ? "Settings" : undefined}
            className={({ isActive }) =>
              cn(
                "flex items-center gap-2.5 px-2.5 py-1.5 rounded-md text-sm",
                "transition-colors duration-[var(--rd-transition-fast)]",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--rd-rose)]",
                isActive
                  ? "bg-[var(--rd-rose)]/10 text-[var(--rd-rose-bright)]"
                  : "text-[var(--rd-fg-muted)] hover:bg-[var(--rd-bg-surface-1)] hover:text-[var(--rd-fg-secondary)]"
              )
            }
          >
            <span className="material-symbols-outlined text-[18px] shrink-0">settings</span>
            {!sidebarCollapsed && <span className="truncate">Settings</span>}
          </NavLink>
        </div>
      </aside>

      {/* ── Top bar ───────────────────────────────────────────────────── */}
      <header
        className={cn(
          "fixed top-0 z-30 h-14 flex items-center justify-between px-6",
          "bg-[var(--rd-bg-surface-0)]/80 backdrop-blur-md border-b border-[var(--rd-bg-surface-2)]",
          "transition-[left] duration-[var(--rd-transition-normal)]"
        )}
        style={{
          left: navWidth,
          right: rightPanelVisible ? 280 : 0,
        }}
      >
        {/* Breadcrumbs */}
        <nav aria-label="Breadcrumb">
          <ol className="flex items-center gap-1.5 text-xs text-[var(--rd-fg-muted)]">
            {breadcrumb.map((crumb, i) => (
              <li key={i} className="flex items-center gap-1.5">
                {i > 0 && <span aria-hidden="true">/</span>}
                <span
                  className={
                    i === breadcrumb.length - 1
                      ? "text-[var(--rd-fg-primary)]"
                      : undefined
                  }
                >
                  {crumb}
                </span>
              </li>
            ))}
          </ol>
        </nav>

        {/* Right controls */}
        <div className="flex items-center gap-3">
          {/* MOCK: search input -- wire in A8 */}
          <div className="w-48 h-7 bg-[var(--rd-bg-surface-1)] rounded-md border border-[var(--rd-bg-surface-3)] flex items-center px-2.5 text-xs text-[var(--rd-fg-muted)]">
            Search...
          </div>
          {/* MOCK: network pulse -- wire to WS status in A8 */}
          <div className="flex items-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full bg-[var(--rd-success)]" aria-hidden="true" />
            <span className="text-[10px] text-[var(--rd-fg-muted)] font-mono">Connected</span>
          </div>
        </div>
      </header>

      {/* ── Main content ──────────────────────────────────────────────── */}
      <main
        className="pt-14 min-h-screen transition-[margin] duration-[var(--rd-transition-normal)]"
        style={{
          marginLeft: navWidth,
          marginRight: rightPanelVisible ? 280 : 0,
        }}
      >
        <Outlet />
      </main>

      {/* ── Right panel ───────────────────────────────────────────────── */}
      {rightPanelVisible && (
        <aside className="fixed right-0 top-14 bottom-0 w-[280px] bg-[var(--rd-bg-surface-0)] border-l border-[var(--rd-bg-surface-2)] z-30 overflow-y-auto">
          <div className="p-4 text-xs text-[var(--rd-fg-muted)]">
            {/* MOCK: C-Factor, ISFR, agent cards -- wire in A8 */}
            <div className="mb-4">
              <div className="text-[10px] font-medium uppercase tracking-wider text-[var(--rd-fg-muted)] mb-2">
                C-Factor
              </div>
              <div className="text-2xl font-mono text-[var(--rd-rose-bright)]">
                +18.4%
              </div>
              <div className="text-[10px] text-[var(--rd-fg-muted)]">
                Collective intelligence surplus
              </div>
            </div>
            <div className="border-t border-[var(--rd-bg-surface-2)] pt-4">
              <div className="text-[10px] font-medium uppercase tracking-wider text-[var(--rd-fg-muted)] mb-2">
                Active agents
              </div>
              <div className="space-y-2">
                {/* MOCK: wire to GET /api/managed-agents -- Task A2 provides this */}
                {["conductor-01", "researcher-02", "validator-03"].map((name) => (
                  <div
                    key={name}
                    className="flex items-center gap-2 px-2 py-1.5 rounded-md bg-[var(--rd-bg-surface-1)]"
                  >
                    <span className="w-1.5 h-1.5 rounded-full bg-[var(--rd-success)]" aria-hidden="true" />
                    <span className="text-xs text-[var(--rd-fg-secondary)]">{name}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </aside>
      )}
    </div>
  );
}
```

#### 9b. LandingLayout

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/layouts/LandingLayout.tsx`:

```tsx
import type { ReactNode } from "react";

type LandingLayoutProps = {
  children: ReactNode;
};

export function LandingLayout({ children }: LandingLayoutProps) {
  return (
    <div className="min-h-screen bg-[var(--rd-bg-void)] text-[var(--rd-fg-primary)]">
      {children}
    </div>
  );
}
```

### 10. Create placeholder landing page

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/Landing.tsx`:

```tsx
import { useNavigate } from "react-router-dom";
import { Button } from "../design-system/components";

export default function Landing() {
  const navigate = useNavigate();

  return (
    <div className="min-h-screen flex flex-col items-center justify-center px-4">
      <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-[var(--rd-rose)] to-[var(--rd-rose-dim)] flex items-center justify-center text-white text-2xl font-bold mb-8">
        N
      </div>
      <h1 className="text-4xl font-bold text-[var(--rd-fg-primary)] mb-3 text-center">
        Nunchi
      </h1>
      <p className="text-lg text-[var(--rd-fg-secondary)] mb-8 text-center max-w-md">
        Hyperdimensional intelligence for autonomous agent orchestration.
      </p>
      <Button size="lg" onClick={() => navigate("/app/chat")}>
        Launch dashboard
      </Button>
      <p className="mt-4 text-xs text-[var(--rd-fg-muted)]">
        {/* Placeholder -- Task A3 provides the full landing page. */}
        Full landing page implemented in Task A3.
      </p>
    </div>
  );
}
```

### 11. Rewrite App.tsx

- [ ] Replace the entire contents of `/Users/will/dev/nunchi/nunchi-dashboard/src/App.tsx`:

```tsx
import { useEffect } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "react-router-dom";
import { ToastProvider } from "./design-system/components";
import { router } from "./router";
import { connectWs, disconnectWs } from "./services/ws";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      retry: 2,
      refetchOnWindowFocus: false,
    },
  },
});

export default function App() {
  useEffect(() => {
    connectWs();
    return () => disconnectWs();
  }, []);

  return (
    <QueryClientProvider client={queryClient}>
      <ToastProvider>
        <RouterProvider router={router} />
      </ToastProvider>
    </QueryClientProvider>
  );
}
```

### 12. Update main.jsx

- [ ] The existing `/Users/will/dev/nunchi/nunchi-dashboard/src/main.jsx` already wraps `<App />` in PrivyProvider with `theme: 'dark'`. No changes needed — the auth gates still work. The only difference is that `App` now renders the router instead of the monolithic `KoraiDashboard` component. If you need to confirm the Privy wrapping still works, check that `npm run dev` loads the password gate (when `VITE_PRIVY_APP_ID` is empty) and after entering the password renders the new AppLayout.

### 13. Add animations and accessibility overrides to index.css

- [ ] Add to the bottom of `/Users/will/dev/nunchi/nunchi-dashboard/src/index.css`:

```css
/* ── Toast slide-in ─────────────────────────────────────────────────────── */
@keyframes slide-in {
  from {
    transform: translateX(100%);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}

.animate-slide-in {
  animation: slide-in 200ms ease-out;
}

/* ── Hero gradient breathe ──────────────────────────────────────────────── */
@keyframes gradient-breathe {
  0%, 100% { opacity: 0.06; }
  50%       { opacity: 0.12; }
}

.animate-gradient-breathe {
  animation: gradient-breathe var(--rd-transition-breathe) ease-in-out infinite;
}

/* ── Reduced-motion overrides ───────────────────────────────────────────── */
@media (prefers-reduced-motion: reduce) {
  .animate-slide-in,
  .animate-gradient-breathe,
  .animate-ping,
  .animate-pulse {
    animation: none;
  }

  .transition-all,
  .transition-colors,
  .transition-opacity,
  [class*="transition-"] {
    transition-duration: 0ms !important;
  }
}
```

---

## Verification

Run from `/Users/will/dev/nunchi/nunchi-dashboard`:

- [ ] `npm run typecheck` -- exits 0
- [ ] `npm run dev` -- open `http://localhost:5173/`
  - Landing page renders with "Launch dashboard" button
  - Clicking the button navigates to `/app/chat`
  - Left nav renders all sections with correct links
  - Right panel shows mock C-Factor and agent cards
  - Collapsing the sidebar (chevron button) shrinks the nav to 60px with a smooth transition
  - Top bar shows breadcrumbs matching the current route
  - Navigating to any `/app/*` route shows the placeholder
- [ ] Open browser devtools console -- zero errors
- [ ] Verify token CSS variables load: in devtools, inspect `:root` -- `--rd-bg-void` should be `#060608`
- [ ] Verify accessibility: tab through the nav links -- each should show a visible focus ring
- [ ] Verify `cn()` works: import it and call `cn("a", false, "b", { c: true, d: false })` -- result should be `"a b c"`
