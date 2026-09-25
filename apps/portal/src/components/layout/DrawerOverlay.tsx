'use client';

import { useEffect, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { X } from 'lucide-react';
import { clsx } from 'clsx';

// ---------------------------------------------------------------------------
// Focus trap — a minimal implementation that keeps focus inside the drawer
// while it is open, and restores it to the previously-focused element on close.
// ---------------------------------------------------------------------------

const FOCUSABLE_SELECTORS = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(', ');

function useFocusTrap(containerRef: React.RefObject<HTMLElement | null>, active: boolean) {
  // Remember what had focus before the drawer opened so we can restore it.
  const previouslyFocused = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!active) return;

    // Capture the currently focused element before we take focus.
    previouslyFocused.current = document.activeElement as HTMLElement;

    const container = containerRef.current;
    if (!container) return;

    // Move initial focus to the first focusable element inside the drawer.
    const firstFocusable = container.querySelector<HTMLElement>(FOCUSABLE_SELECTORS);
    firstFocusable?.focus();

    function handleKeyDown(e: KeyboardEvent) {
      if (e.key !== 'Tab') return;

      const focusable = Array.from(
        container!.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTORS),
      ).filter((el) => !el.closest('[aria-hidden="true"]'));

      if (focusable.length === 0) {
        e.preventDefault();
        return;
      }

      const first = focusable[0];
      const last = focusable[focusable.length - 1];

      if (e.shiftKey) {
        // Shift+Tab: if focus is on the first element, wrap to the last.
        if (document.activeElement === first) {
          e.preventDefault();
          last.focus();
        }
      } else {
        // Tab: if focus is on the last element, wrap to the first.
        if (document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    }

    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      // Restore focus when the trap is released.
      previouslyFocused.current?.focus();
    };
  }, [active, containerRef]);
}

// ---------------------------------------------------------------------------
// Animation variants
// ---------------------------------------------------------------------------

// 350ms with a cubic-bezier that approximates expo-out.
const EXPO_OUT = [0.16, 1, 0.3, 1] as const;

const drawerVariants = {
  hidden:  { x: '100%', opacity: 0.6 },
  visible: { x: 0,      opacity: 1   },
  exit:    { x: '100%', opacity: 0   },
} as const;

const backdropVariants = {
  hidden:  { opacity: 0 },
  visible: { opacity: 1 },
  exit:    { opacity: 0 },
} as const;

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface DrawerOverlayProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  /** Drawer panel width in px. Defaults to 480. */
  width?: number;
  children: React.ReactNode;
}

// ---------------------------------------------------------------------------
// DrawerOverlay
// ---------------------------------------------------------------------------

export default function DrawerOverlay({
  open,
  onClose,
  title,
  width = 480,
  children,
}: DrawerOverlayProps) {
  const panelRef = useRef<HTMLDivElement>(null);

  // Focus trap only active while the drawer is open.
  useFocusTrap(panelRef, open);

  // Escape key dismissal — registered at the document level so it works even
  // if the focus is on a non-interactive element inside the drawer.
  useEffect(() => {
    if (!open) return;

    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      }
    }

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose]);

  return (
    <AnimatePresence>
      {open && (
        <>
          {/* ---- Backdrop ---- */}
          <motion.div
            key="drawer-backdrop"
            className={clsx(
              'fixed inset-0',
              'bg-black/50',
              'z-[var(--z-overlay)]',
              'cursor-pointer',
            )}
            variants={backdropVariants}
            initial="hidden"
            animate="visible"
            exit="exit"
            transition={{ duration: 0.2, ease: 'easeOut' }}
            onClick={onClose}
            aria-hidden="true"
          />

          {/* ---- Panel ---- */}
          <motion.div
            key="drawer-panel"
            ref={panelRef}
            role="dialog"
            aria-modal="true"
            aria-label={title ?? 'Drawer'}
            style={{ width }}
            className={clsx(
              // Position — fixed right edge, full height
              'fixed inset-y-0 right-0',
              'flex flex-col',
              // Surfaces
              'bg-[var(--bg-glass)]',
              'backdrop-blur-md',
              'border-l border-l-[var(--text-ghost)]',
              'shadow-[var(--shadow-lg)]',
              // Stack above backdrop
              'z-[var(--z-modal)]',
            )}
            variants={drawerVariants}
            initial="hidden"
            animate="visible"
            exit="exit"
            transition={{ duration: 0.35, ease: EXPO_OUT }}
          >
            {/* ---- Drawer header ---- */}
            <div
              className={clsx(
                'flex items-center justify-between shrink-0',
                'h-12 px-4',
                'border-b border-b-[var(--text-ghost)]',
              )}
            >
              {title ? (
                <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-strong)] leading-none tracking-[var(--tracking-wide)]">
                  {title}
                </span>
              ) : (
                // Empty flex spacer so the close button stays right-aligned.
                <span />
              )}

              <button
                type="button"
                onClick={onClose}
                className={clsx(
                  'flex items-center justify-center',
                  'h-7 w-7 rounded-sm',
                  'text-[var(--text-faint)] hover:text-[var(--text-muted)]',
                  'hover:bg-[var(--bg-highlight)]',
                  'transition-colors duration-[80ms] ease-[var(--ease-out)]',
                  'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
                )}
                aria-label="Close drawer"
              >
                <X size={14} strokeWidth={1.5} aria-hidden />
              </button>
            </div>

            {/* ---- Scrollable content ---- */}
            <div className="flex-1 overflow-y-auto overflow-x-hidden">
              {children}
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
