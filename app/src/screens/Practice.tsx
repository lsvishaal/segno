import { useCallback, useEffect, useRef, useState } from "react";

import { AnimatePresence, motion } from "motion/react";

import {
  endAttempt,
  exportMusicXml,
  instruments as fetchInstruments,
  listenToPractice,
  rescalePiece,
  startAttempt,
} from "../bridge";
import type { AttemptSummary, InstrumentSummary, Mark, OpenedPiece, PracticeEvent } from "../bridge";
import { Score } from "../components/Score";
import type { CursorState, Dot } from "../components/Score";
import { BEFORE_STARTING, cursorFor, dotFor } from "./overlayEvents";
import { SettingsPanel } from "./PracticeSettings";
import { clampScale, stepScale } from "./zoom";

/** How long to wait after the last zoom change before re-engraving — a real
 * Verovio parse, not a CSS transform, so firing on every wheel tick would
 * visibly lag. */
const RESCALE_DEBOUNCE_MS = 200;

/** Whether the score is drawn on paper or inverted. */
export type Surface = "paper" | "dark";

/** Where the practice loop is, for this piece. */
type Phase =
  | { readonly kind: "choosing"; readonly refusal: string | null }
  | { readonly kind: "practicing"; readonly instrument: string; readonly reachedEnd: boolean }
  | { readonly kind: "ended"; readonly instrument: string; readonly summary: AttemptSummary };

/** A panel appearing or leaving, above the score. */
const PANEL = {
  hidden: { opacity: 0, y: -6, height: 0 },
  shown: { opacity: 1, y: 0, height: "auto" },
};

/**
 * The practice surface.
 *
 * Everything it says about the music, and about how it was played, comes
 * from Rust — the engraved score, the marks, and every `PracticeEvent`. This
 * component turns them into a cursor position and a `Dot`, and it judges
 * nothing (`004 §8.8`): `correct`/`wrong` on a `Played` event is computed in
 * `hogwash-app`'s practice thread from `Cursor::expecting()`, never derived
 * here from whether the Cursor moved (`003 §7.16`).
 */
export function PracticeScreen({
  piece,
  onLeave,
}: {
  readonly piece: OpenedPiece;
  readonly onLeave: () => void;
}) {
  const [surface, setSurface] = useState<Surface>("paper");
  const [showNoteNames, setShowNoteNames] = useState(false);
  // A developer view: what the geometry layer reads from the page.
  const [showGeometry, setShowGeometry] = useState(false);
  const [phase, setPhase] = useState<Phase>({ kind: "choosing", refusal: null });
  const [available, setAvailable] = useState<readonly InstrumentSummary[]>([]);
  const [cursor, setCursor] = useState<CursorState>(BEFORE_STARTING);
  const [dot, setDot] = useState<Dot | null>(null);
  const [lost, setLost] = useState<{ instrument: string; secondsRemaining: number } | null>(
    null,
  );
  const [exportError, setExportError] = useState<string | null>(null);
  const [zoomError, setZoomError] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const gearRef = useRef<HTMLButtonElement>(null);
  // 300 matches Rust's `Layout::default().scale` — the size a piece opens
  // at before anyone has touched the zoom control. Large by default: a
  // performer reading from arm's length benefits from noteheads that are
  // unmistakable rather than merely legible.
  const [scale, setScale] = useState(300);
  const [engraving, setEngraving] = useState<{
    pages: readonly string[];
    marks: readonly Mark[];
  }>({ pages: piece.pages, marks: piece.marks });
  const scoreRef = useRef<HTMLDivElement>(null);
  // A rescale is triggered by a *change* to `scale`, not by mounting with
  // one — otherwise every piece-open would fire a redundant re-engrave at
  // the scale it was already opened at.
  const skipNextRescale = useRef(true);

  // A refusal is transient — it says its piece and then gets out of the way.
  useEffect(() => {
    if (!exportError) return;

    const timeout = window.setTimeout(() => setExportError(null), 5000);
    return () => window.clearTimeout(timeout);
  }, [exportError]);

  useEffect(() => {
    if (!zoomError) return;

    const timeout = window.setTimeout(() => setZoomError(null), 5000);
    return () => window.clearTimeout(timeout);
  }, [zoomError]);

  // A piece switch (or leaving and reopening the same one) starts fresh.
  const pieceId = piece.id;
  useEffect(() => {
    setPhase({ kind: "choosing", refusal: null });
    setCursor(BEFORE_STARTING);
    setDot(null);
    setLost(null);
    setEngraving({ pages: piece.pages, marks: piece.marks });
    setScale(300);
    skipNextRescale.current = true;
  }, [pieceId, piece.pages, piece.marks]);

  // Re-engraves at the new scale, debounced so a rapid Ctrl+scroll doesn't
  // fire a real Verovio parse per wheel tick. Replaces `pages`/`marks`
  // together, atomically — never patched in place — because nothing here
  // assumes element ids survive a rescale (`crates/hogwash-engraving-verovio/
  // tests/engraving.rs` checks that they do, but the design doesn't depend
  // on it).
  useEffect(() => {
    if (skipNextRescale.current) {
      skipNextRescale.current = false;
      return;
    }

    let live = true;

    const timeout = window.setTimeout(() => {
      void rescalePiece(pieceId, scale)
        .then((opened) => {
          if (live) setEngraving({ pages: opened.pages, marks: opened.marks });
        })
        .catch((error: unknown) => {
          // Previously silent: a rejected rescale left the score exactly as
          // it was, indistinguishable from the control doing nothing at all.
          console.error("rescale_piece failed:", error);
          if (live) setZoomError(String(error));
        });
    }, RESCALE_DEBOUNCE_MS);

    return () => {
      live = false;
      window.clearTimeout(timeout);
    };
  }, [scale, pieceId]);

  // Ctrl+scroll zooms. A native listener with `{ passive: false }` rather
  // than JSX's `onWheel`: React may attach wheel listeners as passive by
  // default, which silently ignores `preventDefault()` and lets the page
  // scroll instead of zooming.
  useEffect(() => {
    const element = scoreRef.current;
    if (!element) return;

    const onWheel = (event: WheelEvent) => {
      if (!event.ctrlKey) return;
      event.preventDefault();
      setScale((current) => stepScale(current, event.deltaY));
    };

    element.addEventListener("wheel", onWheel, { passive: false });
    return () => element.removeEventListener("wheel", onWheel);
  }, []);

  useEffect(() => {
    let live = true;

    void (async () => {
      try {
        const found = await fetchInstruments();
        if (live) setAvailable(found);
      } catch {
        // No instrument connected is ordinary (FR-307); the picker below
        // says so on its own once `available` stays empty.
        if (live) setAvailable([]);
      }
    })();

    return () => {
      live = false;
    };
    // Re-checked whenever the picker is showing again, in case a keyboard
    // was plugged in since the last time it was asked.
  }, [phase.kind === "choosing"]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    void listenToPractice((event: PracticeEvent) => {
      setCursor((current) => cursorFor(event, current));

      const next = dotFor(event);
      if (next) setDot(next);

      switch (event.kind) {
        case "played":
          // No in-app sound any more — a real instrument or a MIDI Thru
          // route (`setThru`) is what makes noise now, not this listener.
          break;
        case "advanced":
          setLost(null);
          break;
        case "reachedEnd":
          setPhase((current) =>
            current.kind === "practicing" ? { ...current, reachedEnd: true } : current,
          );
          break;
        case "instrumentLost":
          setLost({ instrument: event.instrument, secondsRemaining: event.secondsRemaining });
          break;
        case "instrumentRestored":
          setLost(null);
          break;
        case "ended":
          setLost(null);
          setPhase((current) =>
            current.kind === "practicing"
              ? { kind: "ended", instrument: current.instrument, summary: event.summary }
              : current,
          );
          break;
      }
    }).then((stop) => {
      unlisten = stop;
    });

    return () => unlisten?.();
  }, []);

  const begin = useCallback(
    (instrument: string) => {
      void (async () => {
        try {
          await startAttempt(pieceId, instrument);
          setCursor(BEFORE_STARTING);
          setDot(null);
          setPhase({ kind: "practicing", instrument, reachedEnd: false });
        } catch (error) {
          setPhase({ kind: "choosing", refusal: String(error) });
        }
      })();
    },
    [pieceId],
  );

  const finish = useCallback(() => {
    void endAttempt();
  }, []);

  const again = useCallback(
    (instrument: string) => {
      begin(instrument);
    },
    [begin],
  );

  const chooseAnother = useCallback(() => {
    setPhase({ kind: "choosing", refusal: null });
  }, []);

  const exportPiece = useCallback(() => {
    void (async () => {
      try {
        await exportMusicXml(pieceId);
      } catch (error) {
        setExportError(String(error));
      }
    })();
  }, [pieceId]);

  return (
    <div className="flex h-full flex-col">
      <header className="flex shrink-0 items-baseline gap-6 px-10 py-6">
        <button
          type="button"
          onClick={onLeave}
          className="cursor-pointer rounded-md px-2 py-1 text-sm text-ink-muted transition-colors hover:bg-ink/5 hover:text-accent focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
        >
          ← Library
        </button>

        <div className="min-w-0 flex-1">
          <h1 className="truncate font-serif text-title leading-tight">
            {piece.title}
          </h1>
          {piece.composer && (
            <p className="truncate text-xs text-ink-muted">{piece.composer}</p>
          )}
        </div>

        {phase.kind === "practicing" && (
          <button
            type="button"
            onClick={finish}
            className="cursor-pointer rounded-md px-3 py-1.5 text-xs font-medium text-ink-muted transition-colors hover:bg-ink/5 hover:text-accent focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
          >
            {phase.reachedEnd ? "Finish" : "Stop"}
          </button>
        )}

        <div className="relative shrink-0">
          <button
            ref={gearRef}
            type="button"
            aria-haspopup="dialog"
            aria-expanded={settingsOpen}
            aria-label="Settings"
            onClick={() => setSettingsOpen((current) => !current)}
            className="flex cursor-pointer items-center justify-center rounded-full p-2 text-ink-muted transition-colors hover:bg-ink/5 hover:text-accent focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
          >
            <GearIcon />
          </button>

          <SettingsPanel
            open={settingsOpen}
            onClose={() => setSettingsOpen(false)}
            triggerRef={gearRef}
            surface={surface}
            onSurfaceChange={setSurface}
            showNoteNames={showNoteNames}
            onShowNoteNamesChange={setShowNoteNames}
            showGeometry={showGeometry}
            onShowGeometryChange={setShowGeometry}
            scale={scale}
            onScaleChange={(next) => setScale(clampScale(next))}
            zoomError={zoomError}
            onExport={exportPiece}
            exportError={exportError}
          />
        </div>
      </header>

      <AnimatePresence initial={false}>
        {phase.kind === "choosing" && (
          <motion.div
            key="picker"
            variants={PANEL}
            initial="hidden"
            animate="shown"
            exit="hidden"
            transition={{ duration: 0.16, ease: "easeOut" }}
            className="overflow-hidden px-8 pt-4"
          >
            <InstrumentPicker available={available} refusal={phase.refusal} onChoose={begin} />
          </motion.div>
        )}

        {lost && phase.kind === "practicing" && (
          <motion.div
            key="lost"
            variants={PANEL}
            initial="hidden"
            animate="shown"
            exit="hidden"
            transition={{ duration: 0.16, ease: "easeOut" }}
            className="shrink-0 overflow-hidden px-8 pt-4"
          >
            <p className="panel rounded-2xl px-5 py-3 text-sm text-ink-muted">
              {lost.instrument} stopped reporting. Waiting {lost.secondsRemaining}s for it to
              reconnect before ending the attempt — what you have played is kept either way.
            </p>
          </motion.div>
        )}

        {phase.kind === "ended" && (
          <motion.div
            key="summary"
            variants={PANEL}
            initial="hidden"
            animate="shown"
            exit="hidden"
            transition={{ duration: 0.2, ease: "easeOut" }}
            className="overflow-hidden px-8 pt-4"
          >
            <SummaryPanel
              summary={phase.summary}
              onAgain={() => again(phase.instrument)}
              onChooseAnother={chooseAnother}
            />
          </motion.div>
        )}
      </AnimatePresence>

      <div
        ref={scoreRef}
        className="hw-score relative min-h-0 flex-1 px-4 py-6 sm:px-8 sm:py-8 lg:px-12 lg:py-10"
        data-surface={surface}
      >
        <Score
          pages={engraving.pages}
          marks={engraving.marks}
          cursor={cursor}
          dot={dot}
          showNoteNames={showNoteNames}
          showGeometry={showGeometry}
        />
      </div>
    </div>
  );
}

/** A gear glyph for the settings trigger — inline so no icon dependency is needed for one glyph. */
function GearIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z" />
    </svg>
  );
}

/** Lets the performer pick which connected instrument to play on (FR-801). */
function InstrumentPicker({
  available,
  refusal,
  onChoose,
}: {
  readonly available: readonly InstrumentSummary[];
  readonly refusal: string | null;
  readonly onChoose: (instrument: string) => void;
}) {
  const [chosen, setChosen] = useState<string | null>(available[0]?.name ?? null);

  useEffect(() => {
    setChosen((current) =>
      current && available.some((instrument) => instrument.name === current)
        ? current
        : (available[0]?.name ?? null),
    );
  }, [available]);

  if (available.length === 0) {
    return (
      <div className="panel shrink-0 rounded-2xl px-7 py-5 text-sm text-ink-muted">
        No MIDI keyboard is connected. Plug one in — Hogwash will notice once you try
        again.
        {refusal && <p className="mt-1 text-ink-muted">{refusal}</p>}
      </div>
    );
  }

  return (
    <div className="panel flex shrink-0 flex-wrap items-center gap-4 rounded-2xl px-7 py-5">
      <span className="text-sm text-ink-muted">Play on</span>

      <div
        role="radiogroup"
        aria-label="Instrument"
        className="flex flex-wrap items-center gap-1 rounded-full border border-rule bg-paper-raised/60 p-0.5"
      >
        {available.map((instrument) => (
          <button
            key={instrument.name}
            type="button"
            role="radio"
            aria-checked={chosen === instrument.name}
            onClick={() => setChosen(instrument.name)}
            className={`cursor-pointer rounded-full px-2.5 py-1 text-xs transition-colors focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent ${
              chosen === instrument.name
                ? "bg-ink text-paper"
                : "text-ink-muted hover:text-ink"
            }`}
          >
            {instrument.name}
          </button>
        ))}
      </div>

      <motion.button
        type="button"
        disabled={!chosen}
        onClick={() => chosen && onChoose(chosen)}
        whileTap={{ scale: 0.96 }}
        className="cursor-pointer rounded-full bg-ink px-4 py-1.5 text-sm font-medium text-paper transition-opacity hover:opacity-90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent disabled:cursor-not-allowed disabled:opacity-40"
      >
        Start
      </motion.button>
      {refusal && <p className="text-sm text-ink-muted">{refusal}</p>}
    </div>
  );
}

/** One count, in its own tile. */
function Stat({ label, value, tone }: { readonly label: string; readonly value: number; readonly tone?: "correct" | "wrong" }) {
  return (
    <div className="rounded-xl border border-rule bg-paper-raised/70 px-4 py-3">
      <p
        className={`font-mono text-2xl tabular-nums ${
          tone === "correct" ? "text-correct" : tone === "wrong" ? "text-wrong" : "text-ink"
        }`}
      >
        {value}
      </p>
      <p className="mt-0.5 text-xs tracking-wide text-ink-muted uppercase">{label}</p>
    </div>
  );
}

/**
 * What an ended attempt has to say.
 *
 * The four counts and the lines both come straight from Rust — `Feedback`'s
 * `Summary` and `lines()` (`004 §8.8`); this only lays them out. Full design
 * of a history-worthy summary screen is Stage E's job, not this pass's.
 */
function SummaryPanel({
  summary,
  onAgain,
  onChooseAnother,
}: {
  readonly summary: AttemptSummary;
  readonly onAgain: () => void;
  readonly onChooseAnother: () => void;
}) {
  return (
    <div className="panel shrink-0 rounded-2xl px-7 py-6">
      {summary.autoEnded && summary.reason && (
        <p className="mb-3 text-sm text-ink-muted">{summary.reason}</p>
      )}

      <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
        <Stat label="Correct" value={summary.correct} tone="correct" />
        <Stat label="Not as written" value={summary.incorrect} tone="wrong" />
        <Stat label="Never played" value={summary.omitted} />
        <Stat label="Unexpected" value={summary.unexpected} />
      </div>

      <ul className="mt-4 space-y-1 text-sm text-ink-muted">
        {summary.lines.map((line, index) => (
          // An ordered, fixed-shape report from Rust, not a list a key could
          // be drawn from on its own — two lines could coincide in text.
          <li key={`${index}-${line}`}>{line}</li>
        ))}
      </ul>

      <div className="mt-4 flex gap-3">
        <motion.button
          type="button"
          onClick={onAgain}
          whileTap={{ scale: 0.96 }}
          className="cursor-pointer rounded-full bg-ink px-4 py-1.5 text-sm font-medium text-paper transition-opacity hover:opacity-90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
        >
          Practice again
        </motion.button>
        <button
          type="button"
          onClick={onChooseAnother}
          className="cursor-pointer rounded-full border border-rule px-4 py-1.5 text-sm transition-colors hover:border-accent hover:text-accent focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
        >
          Choose a different instrument
        </button>
      </div>
    </div>
  );
}
