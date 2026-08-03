import type { PieceSummary } from "../bridge";

/**
 * One piece, on the library shelf.
 *
 * What it shows is deliberately limited to what the Score actually contains:
 * a title, a composer where the file gave one, and how much music there is.
 * No difficulty, no rating, no progress score — `001 §11` rules grading out,
 * and a "difficulty" badge would be exactly that, invented by the interface.
 */
export function PieceCard({
  piece,
  onOpen,
}: {
  readonly piece: PieceSummary;
  readonly onOpen: (id: string) => void;
}) {
  return (
    <li>
      <button
        type="button"
        onClick={() => onOpen(piece.id)}
        className="group w-full cursor-pointer rounded-lg border border-rule bg-paper-raised px-6 py-5 text-left transition duration-200 ease-out hover:-translate-y-0.5 hover:border-ink/20 hover:shadow-[0_8px_24px_-12px_rgb(28_26_23/0.18)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent motion-reduce:transition-none motion-reduce:hover:translate-y-0"
      >
        <h3 className="font-serif text-xl leading-snug tracking-tight text-pretty">
          {piece.title}
        </h3>

        {piece.composer && (
          <p className="mt-0.5 text-sm text-ink-muted">{piece.composer}</p>
        )}

        <p className="mt-3 font-mono text-xs tracking-wide text-ink-muted tabular-nums">
          {piece.bars} {piece.bars === 1 ? "bar" : "bars"}
          <span className="mx-1.5 opacity-40">·</span>
          {piece.notes} {piece.notes === 1 ? "note" : "notes"}
        </p>
      </button>
    </li>
  );
}
