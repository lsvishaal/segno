import type { ReactNode } from "react";

import { AnimatePresence, motion } from "motion/react";

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

const DAY_MS = 24 * 60 * 60 * 1000;

/**
 * When something happened, as a person says it: "today", "yesterday",
 * "3 days ago", then a date ("12 Oct", with the year once it isn't this one).
 * Calendar days in local time, not 24-hour spans.
 */
export function practisedWhen(thenMs: number, nowMs: number): string {
  const then = new Date(thenMs);
  const now = new Date(nowMs);
  const days = Math.round(
    (new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime() -
      new Date(then.getFullYear(), then.getMonth(), then.getDate()).getTime()) /
      DAY_MS,
  );
  if (days <= 0) return "today";
  if (days === 1) return "yesterday";
  if (days < 7) return `${days} days ago`;
  return then.toLocaleDateString("en-GB", {
    day: "numeric",
    month: "short",
    ...(then.getFullYear() === now.getFullYear() ? {} : { year: "numeric" }),
  });
}

/** "weakest bar 9", "weakest bars 6, 14", or nothing. */
export function weakestLine(weakest: readonly number[]): string | null {
  if (weakest.length === 0) return null;
  return `weakest ${weakest.length === 1 ? "bar" : "bars"} ${weakest.join(", ")}`;
}

/** The starter shelves, in the order a first-time player meets them. */
const STARTERS = [
  { level: "beginner", label: "Starter · Beginner", note: "single lines and first two-hand pieces" },
  { level: "intermediate", label: "Starter · Intermediate", note: "around Grades 1–4" },
] as const;

/**
 * The shelf: everything available to practise. Where you left off first,
 * then the pieces you added, then the ones Hogwash comes with, by level.
 *
 * The bundled pieces mean it is never empty, so only "your pieces" has an
 * empty state.
 */
export function LibraryScreen({
  pieces,
  onOpen,
  onRename,
  onCountHistory,
  onDelete,
  onImport,
  importing,
  importError,
  onDismissError,
  now = Date.now(),
}: {
  readonly pieces: readonly PieceSummary[];
  readonly onOpen: (id: string) => void;
  readonly onRename: (id: string, title: string) => Promise<string | null>;
  readonly onCountHistory: (id: string) => Promise<number>;
  readonly onDelete: (id: string) => Promise<string | null>;
  readonly onImport: () => void;
  readonly importing: boolean;
  readonly importError: string | null;
  readonly onDismissError: () => void;
  /** The time "2 days ago" counts from. */
  readonly now?: number;
}) {
  const notes = distinguishing(pieces);
  const latest = pieces.reduce<PieceSummary | null>(
    (best, piece) =>
      piece.lastPractisedMs !== null && piece.lastPractisedMs > (best?.lastPractisedMs ?? -1) ? piece : best,
    null,
  );
  const yours = pieces
    .filter((piece) => piece.level === null)
    .sort((a, b) => (b.lastPractisedMs ?? 0) - (a.lastPractisedMs ?? 0));

  const shelf = (shown: readonly PieceSummary[]) => (
    <motion.ul
      variants={LIST}
      initial="hidden"
      animate="shown"
      className="mt-4 grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3"
    >
      <AnimatePresence initial={false}>
      {shown.map((piece) => (
        <PieceCard
          key={piece.id}
          piece={piece}
          note={notes.get(piece.id)}
          now={now}
          onOpen={onOpen}
          onRename={onRename}
          onCountHistory={onCountHistory}
          onDelete={onDelete}
        />
      ))}
      </AnimatePresence>
    </motion.ul>
  );

  return (
    <div className="mx-auto max-w-3xl px-6 pb-24 sm:px-8 lg:max-w-4xl lg:px-10 xl:max-w-6xl">
      <header className="flex items-baseline justify-between pt-10 pb-6">
        <h1 className="font-serif text-heading">Hogwash</h1>
      </header>

      {latest?.lastPractisedMs != null && (
        <Shelf label="Continue">
          <motion.button
            type="button"
            initial={{ opacity: 0, y: 6 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.2, ease: "easeOut" }}
            onClick={() => onOpen(latest.id)}
            aria-label={`Continue ${latest.title}`}
            data-continue
            className="group mt-4 flex w-full cursor-pointer items-center justify-between gap-6 rounded-xl border border-rule bg-paper-raised px-6 py-5 text-left transition duration-[120ms] ease-out hover:border-ink/20 hover:shadow-[0_8px_24px_-12px_rgb(28_26_23/0.18)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
          >
            <span className="min-w-0">
              {latest.composer && (
                <span className="block text-label text-ink-muted uppercase">{latest.composer}</span>
              )}
              <span className="mt-1 block truncate font-serif text-title italic">{latest.title}</span>
              <span className="mt-2 block text-xs text-ink-muted">
                {[`Practised ${practisedWhen(latest.lastPractisedMs, now)}`, weakestLine(latest.weakest)]
                  .filter(Boolean)
                  .join(" · ")}
              </span>
            </span>
            <span className="shrink-0 rounded-full bg-ink px-4 py-1.5 text-xs font-medium text-paper transition-opacity group-hover:opacity-90">
              Continue
            </span>
          </motion.button>
        </Shelf>
      )}

      <Shelf
        label="Your pieces"
        action={
          <motion.button
            type="button"
            onClick={onImport}
            disabled={importing}
            whileTap={{ scale: 0.96 }}
            title="Import a MusicXML file"
            className="cursor-pointer rounded-full border border-rule px-3 py-1 text-xs font-medium text-ink transition-colors duration-[120ms] hover:border-ink/30 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent disabled:cursor-default disabled:opacity-60"
          >
            {importing ? "Reading…" : "+ Add a piece"}
          </motion.button>
        }
      >
        {importError && (
          <div className="panel mt-4 flex items-start justify-between gap-4 rounded-lg px-4 py-3 text-sm whitespace-pre-line text-ink-muted">
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
        {yours.length > 0 ? (
          shelf(yours)
        ) : (
          <p className="mt-4 max-w-md text-sm text-ink-muted">
            Pieces you add appear here. Hogwash reads MusicXML — export it from MuseScore Studio or
            download from OpenScore.
          </p>
        )}
      </Shelf>

      {STARTERS.map(({ level, label, note }) => {
        const shown = pieces.filter((piece) => piece.level === level);
        return (
          shown.length > 0 && (
            <Shelf key={level} label={label} note={note}>
              {shelf(shown)}
            </Shelf>
          )
        );
      })}
    </div>
  );
}

/** One labelled section of the library. */
function Shelf({
  label,
  note,
  action,
  children,
}: {
  readonly label: string;
  readonly note?: string;
  readonly action?: ReactNode;
  readonly children: ReactNode;
}) {
  return (
    <section aria-label={label} className="mt-10 first-of-type:mt-2">
      <div className="flex items-center justify-between gap-4 border-b border-rule pb-2">
        <h2 className="text-label font-medium text-ink-muted uppercase">
          {label}
          {note && <span className="ml-2 tracking-normal normal-case opacity-80">— {note}</span>}
        </h2>
        {action}
      </div>
      {children}
    </section>
  );
}
