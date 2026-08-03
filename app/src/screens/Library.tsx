import type { PieceSummary } from "../bridge";
import { PieceCard } from "../components/PieceCard";

/**
 * The shelf: everything available to practise.
 *
 * FR-207 says this is never empty, and that is guaranteed in Rust by the
 * `Library` type rather than checked here — so there is no empty state to
 * design. If this list is ever empty, something is wrong further down and an
 * apologetic illustration would be hiding it.
 */
export function LibraryScreen({
  pieces,
  onOpen,
}: {
  readonly pieces: readonly PieceSummary[];
  readonly onOpen: (id: string) => void;
}) {
  return (
    <div className="mx-auto max-w-3xl px-8 pb-24">
      <header className="pt-16 pb-10">
        <h1 className="font-serif text-[2.75rem] leading-none tracking-[-0.02em]">
          Hogwash
        </h1>
        <p className="mt-3 max-w-md text-[15px] leading-relaxed text-ink-muted text-pretty">
          Play a piece and the score waits for you. Every note you play is
          checked against what is written.
        </p>
      </header>

      <h2 className="border-t border-rule pt-6 text-xs font-medium tracking-[0.12em] text-ink-muted uppercase">
        Your music
      </h2>

      <ul className="mt-5 grid gap-3">
        {pieces.map((piece) => (
          <PieceCard key={piece.id} piece={piece} onOpen={onOpen} />
        ))}
      </ul>
    </div>
  );
}
