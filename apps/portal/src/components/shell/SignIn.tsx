import { Wordmark } from '@/components/kit/Wordmark';

/**
 * What the portal shows while it isn't signed in to roko serve: one page, one way in. Nothing else on screen
 * pretends to work.
 */
export function SignIn() {
  return (
    <div className="pt-center">
      <section className="rk-device pt-sign" data-region="sign-in" aria-labelledby="sign-in-title">
        <Wordmark />
        <h1 id="sign-in-title" className="rk-title">
          Open roko with its sign-in link
        </h1>
        <p>
          This page signs in with the link <code className="pt-code">roko serve</code> prints when it starts. The link
          ends in <code className="pt-code">#token=…</code>, and it changes each time roko serve restarts.
        </p>
        <p>
          If roko serve started in the background, the link is the <code className="pt-code">portal:</code> line in
          the project’s <code className="pt-code">.roko/logs/serve.log</code>.
        </p>
      </section>
    </div>
  );
}
