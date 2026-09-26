import { motion } from "motion/react";

import type { PieceSummary } from "../bridge";
import { PieceCard } from "../components/PieceCard";

/** A title or name as a person reads it: no accents, case or extra spaces. */
function loosely(text: string | null): string {
  return (text ?? "")
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase()
    .replace(/\s+/g, " ")
    .trim();
}

/**
 * What tells a piece apart from others that look the same on the shelf.
 *
 * Only for pieces whose titles match (loosely: "Für Elise" and "fur elise"
 * are the same to a reader). The composer is always shown, so a piece whose
 * composer already differs from the rest needs nothing more; otherwise it
 * gets the name of the file it came from.
 */
export function distinguishing(pieces: readonly PieceSummary[]): ReadonlyMap<string, string> {
  const notes = new Map<string, string>();

  for (const piece of pieces) {
    const alike = pieces.filter((other) => loosely(other.title) === loosely(piece.title));
    if (alike.length < 2) continue;

    const composerIsUnique =
      piece.composer !== null &&
      alike.filter((other) => loosely(other.composer) === loosely(piece.composer)).length === 1;
    if (composerIsUnique) continue;

    notes.set(piece.id, piece.fileName ?? "Bundled with Hogwash");
  }

  return notes;
}

const LIST = {
  hidden: {},
  shown: { transition: { staggerChildren: 0.05 } },
};

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
  onRename,
  onImport,
  importing,
  importError,
  onDismissError,
}: {
  readonly pieces: readonly PieceSummary[];
  readonly onOpen: (id: string) => void;
  readonly onRename: (id: string, title: string) => Promise<string | null>;
  readonly onImport: () => void;
  readonly importing: boolean;
  readonly importError: string | null;
  readonly onDismissError: () => void;
}) {
  const notes = distinguishing(pieces);

  return (
    <div className="mx-auto max-w-3xl px-6 pb-24 sm:px-8 lg:max-w-4xl lg:px-10 xl:max-w-6xl">
      <motion.header
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4, ease: "easeOut" }}
        className="pt-20 pb-14"
      >
        <h1 className="font-serif text-display">Hogwash</h1>
        <p className="mt-3 max-w-md text-[15px] leading-relaxed text-ink-muted text-pretty">
          Play a piece and the score waits for you. Every note you play is
          checked against what is written.
        </p>
      </motion.header>

      <div className="staff-rule" />

      <div className="flex items-baseline justify-between pt-4">
        <h2 className="text-label font-medium text-ink-muted uppercase">
          Your music
        </h2>

        <motion.button
          type="button"
          onClick={onImport}
          disabled={importing}
          whileTap={{ scale: 0.96 }}
          className="cursor-pointer rounded-full bg-ink px-4 py-1.5 text-xs font-medium text-paper transition-opacity hover:opacity-90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent disabled:cursor-default disabled:opacity-60"
        >
          {importing ? "Reading…" : "Import MusicXML"}
        </motion.button>
      </div>

      {importError && (
        <div className="panel mt-5 flex items-start justify-between gap-4 rounded-lg px-4 py-3 text-sm whitespace-pre-line text-ink-muted">
          <p>{importError}</p>
          <button
            type="button"
            onClick={onDismissError}
            className="shrink-0 cursor-pointer text-xs tracking-wide text-ink-muted underline decoration-rule underline-offset-4 transition hover:text-ink"
          >
            Dismiss
          </button>
        </div>
      )}

      <motion.ul
        variants={LIST}
        initial="hidden"
        animate="shown"
        className="mt-6 grid grid-cols-1 gap-4 lg:grid-cols-2 lg:gap-5 xl:grid-cols-3"
      >
        {pieces.map((piece) => (
          <PieceCard
            key={piece.id}
            piece={piece}
            note={notes.get(piece.id)}
            onOpen={onOpen}
            onRename={onRename}
          />
        ))}
      </motion.ul>
    </div>
  );
}
