import type { Unavailable } from "../bridge";

/**
 * The screen shown when Segno cannot start.
 *
 * The design requires a failure to resolve into an explanation, and the one
 * that reaches here is the worst case — usually a missing engraving library.
 * So it gets a whole screen: what happened, and every path that was tried, so
 * the performer can put the file in one of them.
 *
 * Every word comes from Rust. This component arranges them and adds nothing.
 */
export function UnavailableScreen({ failure }: { readonly failure: Unavailable }) {
  return (
    <main className="mx-auto flex min-h-full max-w-2xl flex-col justify-center px-8 py-16">
      <h1 className="font-serif text-heading text-balance">
        {failure.summary}
      </h1>

      <p className="mt-5 text-[15px] leading-relaxed text-ink-muted text-pretty">
        {failure.detail}
      </p>

      {failure.lookedIn.length > 0 && (
        <section className="mt-8">
          <div className="border-t border-rule" />
          <h2 className="pt-4 text-label font-medium text-ink-muted uppercase">
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
