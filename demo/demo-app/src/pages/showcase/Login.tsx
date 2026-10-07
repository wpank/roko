import { useEffect, useState, type FormEvent } from 'react';
import { useSearchParams } from 'react-router';
import WakeUpBanner, { WAKE_DELAY_MS } from '../../components/WakeUpBanner';
import { SERVE_URL } from '../../lib/serve-url';
import '../../showcase/showcase.css';
import { CSRF_HEADER, SESSION_PATH, probeSession } from '../../transport/api';

/** The app's base path: `/demo/` in a build, `/` in dev. */
function appBase(): string {
  const env = (import.meta as { env?: { BASE_URL?: string } }).env;
  return env?.BASE_URL ?? '/';
}

/** Where to go after logging in: `next` when it is a path of this site, else the showcase home. */
function safeNext(next: string | null): string {
  return next && next.startsWith('/') && !next.startsWith('//') ? next : appBase();
}

interface LoginError {
  code: string;
  text: string;
}

/** The server's answer to a failed login, in words, keeping its error code verbatim. */
async function loginError(res: Response): Promise<LoginError> {
  const body = (await res.json().catch(() => ({}))) as { error?: string; retry_after_s?: number };
  const code = body.error ?? `http_${res.status}`;
  const retry = body.retry_after_s ?? Number(res.headers.get('Retry-After') ?? NaN);
  const wait = Number.isFinite(retry) ? ` Try again in ${retry} s.` : '';
  if (code === 'invalid_passphrase') return { code, text: 'That passphrase is not right.' };
  if (code === 'login_locked') return { code, text: `Too many failed tries.${wait}` };
  if (code === 'login_busy') return { code, text: `The demo server is busy.${wait}` };
  return { code, text: 'The login did not go through.' };
}

/** `/demo/login`: the passphrase form of showcase mode (S10 §4.2, S11 §4.3). */
export default function Login() {
  const [params] = useSearchParams();
  const next = safeNext(params.get('next'));
  const [passphrase, setPassphrase] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<LoginError | null>(null);
  const [waking, setWaking] = useState(false);

  // Ask the server first: it may be waking from idle, and a live session goes straight on.
  useEffect(() => {
    let live = true;
    const timer = setTimeout(() => {
      if (live) setWaking(true);
    }, WAKE_DELAY_MS);
    void probeSession().then((probe) => {
      if (!live) return;
      clearTimeout(timer);
      setWaking(false);
      if (probe?.authenticated) window.location.assign(next);
    });
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [next]);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    const res = await fetch(`${SERVE_URL}${SESSION_PATH}`, {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json', [CSRF_HEADER]: '1' },
      body: JSON.stringify({ passphrase }),
    }).catch(() => null);
    setBusy(false);
    if (!res) {
      setError({ code: 'unreachable', text: 'The demo server did not answer.' });
      return;
    }
    if (res.ok) {
      window.location.assign(next);
      return;
    }
    setError(await loginError(res));
  }

  return (
    <section className="sc-page" data-showcase-page="login">
      <header className="sc-hero">
        <h1 className="sc-hero__title">A Cybernetic Agent Harness</h1>
        <p className="sc-hero__sub">This showcase is private. Enter the passphrase you were given.</p>
      </header>
      <WakeUpBanner visible={waking} />
      <form className="sc-login" onSubmit={submit}>
        <label htmlFor="sc-passphrase">Passphrase</label>
        <input
          id="sc-passphrase"
          type="password"
          autoComplete="current-password"
          value={passphrase}
          onChange={(event) => setPassphrase(event.target.value)}
          required
        />
        <button type="submit" disabled={busy || passphrase.length === 0}>
          Log in
        </button>
      </form>
      {error && (
        <p className="sc-error" role="alert" data-login-error={error.code}>
          {error.text} <code>{error.code}</code>
        </p>
      )}
    </section>
  );
}
