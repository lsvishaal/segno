import { useState } from "react";

import { motion } from "motion/react";

import type { PieceSummary } from "../bridge";

/** Entrance only — the parent `motion.ul` staggers when each of these plays. */
const VARIANTS = {
  hidden: { opacity: 0, y: 10 },
  shown: { opacity: 1, y: 0 },
};

/**
 * One piece, on the library shelf.
 *
 * What it shows is deliberately limited to what the Score actually contains:
 * a title, a composer where the file gave one, and how much music there is.
 * No difficulty, no rating, no progress score — `001 §11` rules grading out,
 * and a "difficulty" badge would be exactly that, invented by the interface.
 * Plain by design: one theme colour means the shelf carries none of it —
 * every card looks like every other, and only its content tells them apart.
 */
export function PieceCard({
  piece,
  note,
  onOpen,
  onRename,
  onCountHistory,
  onDelete,
}: {
  readonly piece: PieceSummary;
  /** What tells it apart from a piece that looks the same, if one does. */
  readonly note?: string | undefined;
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
      <motion.li variants={VARIANTS} style={{ listStyle: "none" }}>
        <div role="alertdialog" aria-label={`Delete ${piece.title}?`} className={CARD}>
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
      <motion.li variants={VARIANTS} style={{ listStyle: "none" }}>
        <form
          className={CARD}
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

  return (
    <motion.li variants={VARIANTS} className="relative" style={{ listStyle: "none" }}>
      <button
        type="button"
        onClick={() => onOpen(piece.id)}
        className={`group w-full cursor-pointer ${CARD} text-left transition duration-200 ease-out hover:-translate-y-0.5 hover:border-ink/20 hover:shadow-[0_8px_24px_-12px_rgb(28_26_23/0.18)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent motion-reduce:transition-none motion-reduce:hover:translate-y-0`}
      >
        <h3 className="font-serif text-title text-pretty">
          {piece.title}
        </h3>

        {piece.composer && (
          <p className="mt-0.5 text-sm text-ink-muted">{piece.composer}</p>
        )}

        {note && <p className="mt-0.5 truncate font-mono text-xs text-ink-muted">{note}</p>}

        <p className="mt-3 font-mono text-xs tracking-wide text-ink-muted tabular-nums">
          {piece.bars} {piece.bars === 1 ? "bar" : "bars"}
          <span className="mx-1.5 opacity-40">·</span>
          {piece.notes} {piece.notes === 1 ? "note" : "notes"}
        </p>
      </button>

      {/* Beside the card's button, not inside it: a button cannot hold another. */}
      {piece.imported && (
        <div className="absolute right-4 bottom-4 flex gap-1">
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

const CARD = "block rounded-lg border border-rule bg-paper-raised px-7 py-6";
const ACTION =
  "cursor-pointer rounded-full px-3 py-1 text-xs font-medium transition focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent";
