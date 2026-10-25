import { useState } from "react";

import { motion } from "motion/react";

import type { PieceSummary } from "../bridge";
import { practisedWhen, weakestLine } from "../screens/Library";

/** Entrance only — the parent `motion.ul` staggers when each of these plays. */
const VARIANTS = {
  hidden: { opacity: 0, y: 10 },
  shown: { opacity: 1, y: 0, transition: { duration: 0.2, ease: "easeOut" } },
} as const;

/**
 * On a shelf: enters with the others, fades out when deleted, and slides to
 * close a gap a deleted neighbour leaves (ROADMAP §16.5: 160 ms, ease-out).
 */
const SHELVED = {
  variants: VARIANTS,
  layout: "position",
  exit: { opacity: 0, scale: 0.98, transition: { duration: 0.16, ease: "easeOut" } },
  transition: { layout: { duration: 0.16, ease: "easeOut" } },
} as const;

/**
 * One piece, on the library shelf: composer, title, how much music there is,
 * and when you last practised it with its weakest bars. No difficulty, rating
 * or progress score: nothing that grades the player or the piece.
 */
export function PieceCard({
  piece,
  note,
  now,
  onOpen,
  onRename,
  onCountHistory,
  onDelete,
}: {
  readonly piece: PieceSummary;
  /** What tells it apart from a piece that looks the same, if one does. */
  readonly note?: string | undefined;
  /** The time "2 days ago" counts from. */
  readonly now: number;
  readonly onOpen: (id: string) => void;
  /** Saves a new title; resolves to why it could not be, or `null`. */
  readonly onRename: (id: string, title: string) => Promise<string | null>;
  /** How many practice sessions deleting it would take with it. */
  readonly onCountHistory: (id: string) => Promise<number>;
  /** Deletes it; resolves to why it could not be, or `null`. */
  readonly onDelete: (id: string) => Promise<string | null>;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(piece.title);
  const [problem, setProblem] = useState<string | null>(null);
  // Set while asking "are you sure?": how much history would go with it.
  const [confirming, setConfirming] = useState<number | null>(null);

  const askToDelete = async () => {
    try {
      setConfirming(await onCountHistory(piece.id));
    } catch (error) {
      setProblem(String(error));
    }
  };

  const remove = async () => {
    const refused = await onDelete(piece.id);
    if (refused) {
      setConfirming(null);
      setProblem(refused);
    }
  };

  const rename = async () => {
    if (draft.trim() === "") {
      setProblem("A title cannot be blank.");
      return;
    }
    if (draft.trim() === piece.title) {
      setEditing(false);
      return;
    }
    const refused = await onRename(piece.id, draft);
    if (refused) setProblem(refused);
    else setEditing(false);
  };

  const stopEditing = () => {
    setEditing(false);
    setDraft(piece.title);
    setProblem(null);
  };

  if (confirming !== null) {
    return (
      <motion.li {...SHELVED} style={{ listStyle: "none" }}>
        <div role="alertdialog" aria-label={`Delete ${piece.title}?`} className={`hw-fade-in ${CARD}`}>
          <p className="text-sm text-ink">
            Delete <span className="font-serif">“{piece.title}”</span>?{" "}
            {confirming === 0
              ? "It has no practice history."
              : `Its practice history (${confirming} ${confirming === 1 ? "session" : "sessions"}) will be deleted too.`}{" "}
            This can’t be undone.
          </p>
          <div className="mt-4 flex gap-2">
            <button
              type="button"
              onClick={() => void remove()}
              className={`${ACTION} bg-wrong text-paper hover:opacity-90`}
            >
              Delete
            </button>
            <button
              type="button"
              autoFocus
              onClick={() => setConfirming(null)}
              className={`${ACTION} text-ink-muted hover:text-ink`}
            >
              Cancel
            </button>
          </div>
        </div>
      </motion.li>
    );
  }

  if (editing) {
    return (
      <motion.li {...SHELVED} style={{ listStyle: "none" }}>
        <form
          className={`hw-fade-in ${CARD}`}
          onSubmit={(event) => {
            event.preventDefault();
            void rename();
          }}
        >
          <label className="text-xs text-ink-muted" htmlFor={`title-${piece.id}`}>
            Title
          </label>
          <input
            id={`title-${piece.id}`}
            autoFocus
            value={draft}
            maxLength={200}
            onChange={(event) => {
              setDraft(event.target.value);
              setProblem(null);
            }}
            onKeyDown={(event) => {
              if (event.key === "Escape") stopEditing();
            }}
            className="mt-1 w-full rounded-md border border-rule bg-paper px-2 py-1 font-serif text-title focus-visible:outline-2 focus-visible:outline-accent"
          />
          {problem && <p className="mt-1 text-xs text-wrong">{problem}</p>}
          <div className="mt-3 flex gap-2">
            <button type="submit" className={`${ACTION} bg-ink text-paper hover:opacity-90`}>
              Save
            </button>
            <button type="button" onClick={stopEditing} className={`${ACTION} text-ink-muted hover:text-ink`}>
              Cancel
            </button>
          </div>
        </form>
      </motion.li>
    );
  }

  const practised =
    piece.lastPractisedMs === null
      ? "Not practised yet"
      : [`Last practised ${practisedWhen(piece.lastPractisedMs, now)}`, weakestLine(piece.weakest)]
          .filter(Boolean)
          .join(" · ");

  return (
    <motion.li {...SHELVED} className="group/card relative h-full" style={{ listStyle: "none" }}>
      <button
        type="button"
        onClick={() => onOpen(piece.id)}
        className={`hw-fade-in flex h-full w-full cursor-pointer flex-col ${CARD} text-left transition duration-[120ms] ease-out hover:-translate-y-0.5 hover:border-ink/20 hover:shadow-[0_8px_24px_-12px_rgb(28_26_23/0.18)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent motion-reduce:transition-none motion-reduce:hover:translate-y-0`}
      >
        <p className="min-h-4 pr-24 text-label text-ink-muted uppercase">{piece.composer}</p>
        <h3 className="mt-1 font-serif text-title italic text-pretty">{piece.title}</h3>
        {note && <p className="mt-0.5 truncate font-mono text-xs text-ink-muted">{note}</p>}

        <p className="mt-3 mb-4 font-mono text-xs text-ink-muted tabular-nums">
          {piece.bars} {piece.bars === 1 ? "bar" : "bars"}
          <span className="mx-1.5 opacity-40">·</span>
          {piece.notes} {piece.notes === 1 ? "note" : "notes"}
        </p>

        <p className="mt-auto border-t border-rule pt-3 text-xs text-ink-muted">
          {practised}
        </p>
      </button>

      {/* Beside the card's button, not inside it: a button cannot hold another.
          Quiet until the card is hovered or one of them has focus. */}
      {piece.imported && (
        <div className="absolute top-4 right-4 flex gap-1 opacity-0 transition-opacity duration-[120ms] group-hover/card:opacity-100 focus-within:opacity-100">
          <button
            type="button"
            aria-label={`Rename ${piece.title}`}
            onClick={() => setEditing(true)}
            className={`${ACTION} text-ink-muted hover:bg-ink/5 hover:text-ink`}
          >
            Rename
          </button>
          <button
            type="button"
            aria-label={`Delete ${piece.title}`}
            onClick={() => void askToDelete()}
            className={`${ACTION} text-ink-muted hover:bg-wrong/10 hover:text-wrong`}
          >
            Delete
          </button>
        </div>
      )}

      {problem && !editing && <p className="mt-1 px-2 text-xs text-wrong">{problem}</p>}
    </motion.li>
  );
}

const CARD = "block rounded-xl border border-rule bg-paper-raised px-6 py-5";
const ACTION =
  "cursor-pointer rounded-full px-3 py-1 text-xs font-medium transition focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent";
