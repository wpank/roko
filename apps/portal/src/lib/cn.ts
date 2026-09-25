/**
 * Class name composition utility.
 *
 * A thin wrapper around `clsx` that provides a familiar `cn()` API matching
 * the shadcn/ui convention.  The portal uses only `clsx` (no tailwind-merge)
 * because Tailwind v4 handles class deduplication at the CSS layer.
 *
 * @example
 * cn('base-class', isActive && 'active', { 'conditional': someFlag })
 * // => 'base-class active conditional'  (when isActive and someFlag are true)
 */
import { clsx, type ClassValue } from 'clsx';

export function cn(...inputs: ClassValue[]): string {
  return clsx(inputs);
}
