import type { Unavailable } from "../bridge";

/**
 * The screen shown when Hogwash cannot start.
 *
 * `004 §8.2` requires a failure to resolve into an explanation, and the one
 * that reaches here is the worst case — usually a missing engraving library.
 * So it gets a whole screen: what happened, and every path that was tried, so
 * the performer can put the file in one of them.
 *
 * Every word comes from Rust. This component arranges them and adds nothing.
 */
export function UnavailableScreen({ failure }: { readonly failure: Unavailable }) {
  return (
    <main className="mx-auto flex min-h-full max-w-2xl flex-col justify-center px-8 py-16">
      <h1 className="font-serif text-3xl leading-tight tracking-tight text-balance">
        {failure.summary}
      </h1>

      <p className="mt-5 text-[15px] leading-relaxed text-ink-muted text-pretty">
        {failure.detail}
      </p>

      {failure.lookedIn.length > 0 && (
        <section className="mt-8 border-t border-rule pt-6">
          <h2 className="text-xs font-medium tracking-[0.12em] text-ink-muted uppercase">
            Looked in
          </h2>
          <ul className="mt-3 space-y-1.5">
            {failure.lookedIn.map((where) => (
              <li key={where} className="font-mono text-[13px] break-all text-ink-muted">
                {where}
              </li>
            ))}
          </ul>
        </section>
      )}
    </main>
  );
}
