import { useState } from "react";

import type { OpenedPiece } from "../bridge";
import { Score } from "../components/Score";
import type { Marked } from "../components/Score";

/** Whether the score is drawn on paper or inverted. */
export type Surface = "paper" | "dark";

/**
 * The practice surface.
 *
 * For now it shows the score and a cursor resting on the first expected
 * position; the keyboard arrives in the next stage. Everything it says about
 * the music comes from `OpenedPiece` — the title, the composer, where the
 * notes are — and it computes none of it.
 */
export function PracticeScreen({
  piece,
  marked,
  onLeave,
}: {
  readonly piece: OpenedPiece;
  readonly marked: Marked;
  readonly onLeave: () => void;
}) {
  const [surface, setSurface] = useState<Surface>("paper");

  return (
    <div className="flex h-full flex-col">
      <header className="flex shrink-0 items-baseline gap-4 border-b border-rule px-8 py-4">
        <button
          type="button"
          onClick={onLeave}
          className="cursor-pointer text-sm text-ink-muted transition-colors hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
        >
          ← Library
        </button>

        <div className="min-w-0 flex-1">
          <h1 className="truncate font-serif text-lg leading-tight tracking-tight">
            {piece.title}
          </h1>
          {piece.composer && (
            <p className="truncate text-xs text-ink-muted">{piece.composer}</p>
          )}
        </div>

        <fieldset className="flex shrink-0 items-center gap-0.5 rounded-md border border-rule p-0.5">
          <legend className="sr-only">Score appearance</legend>
          {(["paper", "dark"] as const).map((option) => (
            <button
              key={option}
              type="button"
              aria-pressed={surface === option}
              onClick={() => setSurface(option)}
              className={`cursor-pointer rounded px-2.5 py-1 text-xs capitalize transition-colors focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent ${
                surface === option
                  ? "bg-ink text-paper"
                  : "text-ink-muted hover:text-ink"
              }`}
            >
              {option}
            </button>
          ))}
        </fieldset>
      </header>

      <div
        className="hw-score min-h-0 flex-1 px-8 py-6"
        data-surface={surface}
      >
        <Score pages={piece.pages} marks={piece.marks} marked={marked} />
      </div>
    </div>
  );
}
