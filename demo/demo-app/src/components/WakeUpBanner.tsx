/** How long a session probe may take before the banner shows: longer is a Fly cold start. */
export const WAKE_DELAY_MS = 1500;

/**
 * "Waking the demo server…" while the first answer is slow (S10 §4.2): a Machine that stopped
 * on idle (G10) takes a few seconds to start again.
 */
export default function WakeUpBanner({ visible }: { visible: boolean }) {
  if (!visible) return null;
  return (
    <p className="sc-wakeup sc-dim" role="status" data-wakeup>
      waking the demo server…
    </p>
  );
}
