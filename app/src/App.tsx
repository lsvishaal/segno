import { useCallback, useEffect, useState } from "react";
import type { ReactNode } from "react";

import { AnimatePresence, motion, MotionConfig } from "motion/react";

import {
  deletePiece,
  historyCount,
  importMusicXml,
  library,
  openPiece,
  renamePiece,
  startupFailure,
} from "./bridge";
import type { ImportedPiece, OpenedPiece, PieceSummary, Unavailable } from "./bridge";
import { LibraryScreen } from "./screens/Library";
import { PracticeScreen } from "./screens/Practice";
import { ProgressScreen, type PractiseRange } from "./screens/Progress";
import { UnavailableScreen } from "./screens/Unavailable";

/**
 * A cross-fade between screens, never a hard cut: the old screen fades out
 * while the new one fades in, in the same grid cell of `#root`, so a switch
 * costs 120 ms rather than an exit followed by an entrance. The leaving screen
 * stops taking the pointer at once, so a click lands on the new one.
 *
 * The `key` that makes `AnimatePresence` treat one screen as leaving and the
 * next as entering belongs on this component itself at each call site, not
 * on the `motion.div` inside it — `AnimatePresence` only tracks the identity
 * of its direct children.
 */
function Transitioned({ children }: { readonly children: ReactNode }) {
  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.98 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.98, pointerEvents: "none" }}
      transition={{ duration: 0.12, ease: "easeOut" }}
      className="h-full [grid-area:1/1]"
    >
      {children}
    </motion.div>
  );
}

/**
 * What the window is showing.
 *
 * A discriminated union rather than several booleans, so "loaded but also
 * failed" is not a state that can be reached.
 */
type Screen =
  | { readonly kind: "starting" }
  | { readonly kind: "unavailable"; readonly failure: Unavailable }
  | { readonly kind: "library"; readonly pieces: readonly PieceSummary[] }
  | { readonly kind: "progress"; readonly pieces: readonly PieceSummary[]; readonly id: string }
  | {
      readonly kind: "practice";
      readonly pieces: readonly PieceSummary[];
      readonly piece: OpenedPiece;
      /** From the Progress page: loop this (null: from bar 1) and start at once. */
      readonly practise?: PractiseRange | undefined;
    };

/** How long a notice stays before it fades. */
const NOTICE_MS = 4000;

/**
 * The shelf after an import: the new piece added once, and nothing added
 * for a file identical to one already there.
 */
export function afterImport(
  pieces: readonly PieceSummary[],
  imported: ImportedPiece,
): readonly PieceSummary[] {
  if (imported.already || pieces.some((piece) => piece.id === imported.piece.id)) return pieces;
  return [...pieces, imported.piece];
}

export function App() {
  const [screen, setScreen] = useState<Screen>({ kind: "starting" });
  const [importing, setImporting] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);
  // A short word about something that just happened ("Already in your library").
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    if (!notice) return;
    const timeout = window.setTimeout(() => setNotice(null), NOTICE_MS);
    return () => window.clearTimeout(timeout);
  }, [notice]);

  useEffect(() => {
    let live = true;

    // Whether Segno started is asked first: if it did not, nothing else is
    // worth asking.
    void (async () => {
      try {
        const failure = await startupFailure();
        if (!live) return;

        if (failure) {
          setScreen({ kind: "unavailable", failure });
          return;
        }

        const pieces = await library();
        if (live) setScreen({ kind: "library", pieces });
      } catch (error) {
        if (live) setScreen({ kind: "unavailable", failure: defect(error) });
      }
    })();

    return () => {
      live = false;
    };
  }, []);

  const open = useCallback(
    (id: string, practise?: PractiseRange) => {
      void (async () => {
        try {
          const piece = await openPiece(id);

          setScreen((current) =>
            current.kind === "library" || current.kind === "practice" || current.kind === "progress"
              ? { kind: "practice", pieces: current.pieces, piece, practise }
              : current,
          );
        } catch (error) {
          setScreen({ kind: "unavailable", failure: defect(error) });
        }
      })();
    },
    [],
  );

  const leave = useCallback(() => {
    setScreen((current) =>
      current.kind === "practice" || current.kind === "progress"
        ? { kind: "library", pieces: current.pieces }
        : current,
    );
  }, []);

  const showProgress = useCallback((id: string) => {
    setScreen((current) =>
      current.kind === "library" ? { kind: "progress", pieces: current.pieces, id } : current,
    );
  }, []);

  // A refused import surfaces as a banner on the library screen, not a
  // full-screen takeover: one file that did not work is not "Segno cannot
  // start" (`UnavailableScreen` is reserved for that). The command's error is
  // already the refusal's reason and route, joined verbatim — nothing here
  // adds wording of its own.
  const importPiece = useCallback(() => {
    void (async () => {
      setImporting(true);
      setImportError(null);

      try {
        const imported = await importMusicXml();

        if (imported) {
          setScreen((current) =>
            current.kind === "library"
              ? { kind: "library", pieces: afterImport(current.pieces, imported) }
              : current,
          );

          // The same music is already here: go to it rather than add a copy.
          if (imported.already) {
            setNotice("Already in your library");
            open(imported.piece.id);
          }
        }
      } catch (error) {
        setImportError(String(error));
      } finally {
        setImporting(false);
      }
    })();
  }, [open]);

  // A refusal goes back to the card being edited, which shows it there.
  const rename = useCallback(async (id: string, title: string): Promise<string | null> => {
    try {
      const renamed = await renamePiece(id, title);
      setScreen((current) =>
        current.kind === "library"
          ? {
              kind: "library",
              pieces: current.pieces.map((piece) => (piece.id === id ? renamed : piece)),
            }
          : current,
      );
      setNotice("Renamed");
      return null;
    } catch (error) {
      return String(error);
    }
  }, []);

  const remove = useCallback(async (id: string): Promise<string | null> => {
    try {
      await deletePiece(id);
      setScreen((current) =>
        current.kind === "library"
          ? { kind: "library", pieces: current.pieces.filter((piece) => piece.id !== id) }
          : current,
      );
      setNotice("Deleted");
      return null;
    } catch (error) {
      return String(error);
    }
  }, []);

  const dismissImportError = useCallback(() => setImportError(null), []);

  // `reducedMotion="user"` makes every `motion.*` component below honour
  // `prefers-reduced-motion` automatically, in one place, rather than each
  // component checking it independently.
  return (
    <MotionConfig reducedMotion="user">
      <AnimatePresence>
        {(() => {
          switch (screen.kind) {
            case "starting":
              // Deliberately blank. Startup is milliseconds, and a spinner
              // that flashes for one frame is worse than nothing.
              return <main key="starting" className="min-h-full [grid-area:1/1]" />;

            case "unavailable":
              return (
                <Transitioned key="unavailable">
                  <UnavailableScreen failure={screen.failure} />
                </Transitioned>
              );

            case "library":
              return (
                <Transitioned key="library">
                  <LibraryScreen
                    pieces={screen.pieces}
                    onOpen={open}
                    onProgress={showProgress}
                    onRename={rename}
                    onCountHistory={historyCount}
                    onDelete={remove}
                    onImport={importPiece}
                    importing={importing}
                    importError={importError}
                    onDismissError={dismissImportError}
                  />
                </Transitioned>
              );

            case "progress":
              return (
                <Transitioned key={`progress-${screen.id}`}>
                  <ProgressScreen
                    id={screen.id}
                    backLabel="Library"
                    onBack={leave}
                    onPractise={(range) => open(screen.id, range)}
                  />
                </Transitioned>
              );

            case "practice":
              return (
                <Transitioned key={`practice-${screen.piece.id}`}>
                  <PracticeScreen piece={screen.piece} practise={screen.practise} onLeave={leave} />
                </Transitioned>
              );
          }
        })()}
      </AnimatePresence>

      <AnimatePresence>
        {notice && (
          <motion.p
            key={notice}
            role="status"
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 8 }}
            className="panel fixed bottom-6 left-1/2 -translate-x-1/2 rounded-full px-5 py-2 text-sm text-ink shadow-lg"
          >
            {notice}
          </motion.p>
        )}
      </AnimatePresence>
    </MotionConfig>
  );
}

/**
 * A command that threw.
 *
 * That is a defect in Segno rather than something the performer did, and
 * saying so is better than a blank window.
 */
function defect(error: unknown): Unavailable {
  return {
    summary: "Segno ran into a problem.",
    detail: String(error),
    lookedIn: [],
  };
}
