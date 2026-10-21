import { useCallback, useEffect, useRef, useState } from "react";

import { AnimatePresence, motion } from "motion/react";

import {
  clearLoop,
  endAttempt,
  engravePiece,
  exportMusicXml,
  instruments as fetchInstruments,
  listenToPractice,
  loopBars,
  loopTarget,
  startAtBar,
  startAttempt,
} from "../bridge";
import type {
  AttemptSummary,
  Bar,
  InstrumentSummary,
  Mark,
  OpenedPiece,
  PracticeEvent,
  StartPoint,
} from "../bridge";
import { DeviceSelect } from "../components/DeviceSelect";
import { Score } from "../components/Score";
import type { CursorState, Dot } from "../components/Score";
import { BEFORE_STARTING, cursorFor, dotFor } from "./overlayEvents";
import { SettingsPanel } from "./PracticeSettings";
import { REFLOW_THRESHOLD, clampScale, loadZoom, saveZoom, scoreWidth, stepScale } from "./zoom";

/** How long to wait after the last zoom or width change before re-engraving —
 * a real Verovio pass, so not on every wheel tick or resize frame. */
const REFLOW_DEBOUNCE_MS = 250;

/** Clean passes in a row that end a loop, unless the performer picks otherwise. */
const LOOP_TARGET = 3;

/** Bars as a performer names them: "bar 9", "bars 9–12". */
function barsNamed(first: number, last: number): string {
  return first === last ? `bar ${first}` : `bars ${first}–${last}`;
}

/** Whether a key press is being typed into something rather than meant for the screen. */
function typing(event: KeyboardEvent): boolean {
  return (
    event.target instanceof Element &&
    event.target.closest("input, select, textarea, [contenteditable]") !== null
  );
}

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
  const [showNoteNames, setShowNoteNames] = useState(false);
  // A developer view: what the geometry layer reads from the page.
  const [showGeometry, setShowGeometry] = useState(false);
  const [phase, setPhase] = useState<Phase>({ kind: "choosing", refusal: null });
  const [available, setAvailable] = useState<readonly InstrumentSummary[]>([]);
  const [cursor, setCursor] = useState<CursorState>(BEFORE_STARTING);
  // Where attempts start. Held by the practice thread; this is its echo.
  const [start, setStart] = useState<StartPoint>({ position: 0, measure: 1 });
  // Something said about a move or a loop, for a few seconds.
  const [moveRefusal, setMoveRefusal] = useState<string | null>(null);
  // The bars being looped, and how many clean passes in a row so far.
  const [loop, setLoop] = useState<{ first: number; last: number } | null>(null);
  const [cleanInARow, setCleanInARow] = useState(0);
  const [target, setTarget] = useState<number | null>(LOOP_TARGET);
  const [dot, setDot] = useState<Dot | null>(null);
  const [lost, setLost] = useState<{ instrument: string; secondsRemaining: number } | null>(
    null,
  );
  const [exportError, setExportError] = useState<string | null>(null);
  const [zoomError, setZoomError] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const gearRef = useRef<HTMLButtonElement>(null);
  // Zoom, remembered across pieces and restarts.
  const [scale, setScale] = useState(loadZoom);
  const [engraving, setEngraving] = useState<{
    pages: readonly string[];
    marks: readonly Mark[];
    bars: readonly Bar[];
  }>({ pages: piece.pages, marks: piece.marks, bars: piece.bars });
  // The piece arrives engraved for no particular window; it stays hidden
  // until it has been engraved for this one, so the music never visibly jumps.
  const [fitted, setFitted] = useState(false);
  const fittedRef = useRef(false);
  fittedRef.current = fitted;
  // The width the score is engraved for, in pixels, once measured.
  const [width, setWidth] = useState<number | null>(null);
  const scoreRef = useRef<HTMLDivElement>(null);

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

  useEffect(() => {
    if (!moveRefusal) return;

    const timeout = window.setTimeout(() => setMoveRefusal(null), 5000);
    return () => window.clearTimeout(timeout);
  }, [moveRefusal]);

  // A piece switch (or leaving and reopening the same one) starts fresh.
  const pieceId = piece.id;
  useEffect(() => {
    setPhase({ kind: "choosing", refusal: null });
    setCursor(BEFORE_STARTING);
    setStart({ position: 0, measure: 1 });
    setLoop(null);
    setTarget(LOOP_TARGET);
    setDot(null);
    setLost(null);
    setEngraving({ pages: piece.pages, marks: piece.marks, bars: piece.bars });
    setFitted(false);
  }, [pieceId, piece.pages, piece.marks, piece.bars]);

  useEffect(() => saveZoom(scale), [scale]);

  // The room the score has. Small changes (a scrollbar coming and going) are
  // ignored, or they could re-engrave back and forth.
  useEffect(() => {
    const element = scoreRef.current;
    if (!element) return;

    const observer = new ResizeObserver(([entry]) => {
      if (!entry) return;
      const next = scoreWidth(entry.contentRect.width);
      setWidth((current) =>
        current === null || Math.abs(current - next) >= REFLOW_THRESHOLD ? next : current,
      );
    });

    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  // Re-engraves to fit the width at the zoom, so the music reflows: at once
  // for a newly opened piece, debounced after that. Pages and marks are
  // replaced together, never patched.
  useEffect(() => {
    if (width === null) return;

    let live = true;

    const timeout = window.setTimeout(
      () => {
        void engravePiece(pieceId, scale, width)
          .then((opened) => {
            if (live) setEngraving({ pages: opened.pages, marks: opened.marks, bars: opened.bars });
          })
          .catch((error: unknown) => {
            console.error("engrave_piece failed:", error);
            if (live) setZoomError(String(error));
          })
          .finally(() => {
            // Shown either way: a failed reflow still leaves a readable score.
            if (live) setFitted(true);
          });
      },
      fittedRef.current ? REFLOW_DEBOUNCE_MS : 0,
    );

    return () => {
      live = false;
      window.clearTimeout(timeout);
    };
  }, [scale, width, pieceId]);

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
        case "loopCount":
        case "looped":
          setCleanInARow(event.cleanInARow);
          break;
        case "loopDone":
          setLoop(null);
          setMoveRefusal(`${event.passes} clean passes · ${barsNamed(event.first, event.last)}`);
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
          setCursor({ position: start.position });
          setDot(null);
          setPhase({ kind: "practicing", instrument, reachedEnd: false });
        } catch (error) {
          setPhase({ kind: "choosing", refusal: String(error) });
        }
      })();
    },
    [pieceId, start.position],
  );

  // A bar clicked: the practice thread moves the start there, and, while
  // practising, keeps the attempt so far and begins a new one at it.
  const moveTo = useCallback(
    (measure: number) => {
      void (async () => {
        try {
          const point = await startAtBar(pieceId, measure);
          setStart(point);
          setLoop(null);
          setCursor({ position: point.position });
          setDot(null);
          setPhase((current) =>
            current.kind === "practicing" ? { ...current, reachedEnd: false } : current,
          );
        } catch (error) {
          setMoveRefusal(String(error));
        }
      })();
    },
    [pieceId],
  );

  // Loops bars `first` to `last`; while practising, the first pass begins.
  const loopTo = useCallback(
    (first: number, last: number) => {
      void (async () => {
        try {
          const range = await loopBars(pieceId, first, last, target);
          setStart({ position: range.position, measure: range.first });
          setLoop({ first: range.first, last: range.last });
          setCleanInARow(0);
          setCursor({ position: range.position });
          setDot(null);
          setPhase((current) =>
            current.kind === "practicing" ? { ...current, reachedEnd: false } : current,
          );
        } catch (error) {
          setMoveRefusal(String(error));
        }
      })();
    },
    [pieceId, target],
  );

  // A click starts there; Shift+click loops from the start bar to there.
  const onBarClick = useCallback(
    (measure: number, how: { readonly shift: boolean }) => {
      if (how.shift) loopTo(start.measure, measure);
      else moveTo(measure);
    },
    [loopTo, moveTo, start.measure],
  );

  const stopLooping = useCallback(() => {
    setLoop(null);
    void clearLoop();
  }, []);

  // The Loop button and L: loop the bar the cursor is in, or stop looping.
  const cursorBar =
    engraving.marks.find((mark) => mark.position === cursor.position)?.measure ?? start.measure;
  const toggleLoop = useCallback(() => {
    if (loop) stopLooping();
    else loopTo(cursorBar, cursorBar);
  }, [loop, stopLooping, loopTo, cursorBar]);

  const chooseTarget = useCallback(
    (next: number | null) => {
      setTarget(next);
      if (loop) void loopTarget(next);
    },
    [loop],
  );

  const keyboardLost = lost !== null && phase.kind === "practicing";

  // L loops or stops looping; Esc stops looping. Not while typing, nor while
  // the settings panel has Esc for itself.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.ctrlKey || event.metaKey || event.altKey || typing(event)) return;
      if (event.key.toLowerCase() === "l" && !keyboardLost) {
        event.preventDefault();
        toggleLoop();
      } else if (event.key === "Escape" && loop && !settingsOpen) {
        stopLooping();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [toggleLoop, stopLooping, loop, settingsOpen, keyboardLost]);

  const reachedEnd = phase.kind === "practicing" && phase.reachedEnd;
  const finish = useCallback(() => {
    void endAttempt(reachedEnd ? "finished" : "stop");
  }, [reachedEnd]);

  const again = useCallback(
    (instrument: string) => {
      begin(instrument);
    },
    [begin],
  );

  // Starting again while an attempt runs ends it and keeps it (the backend
  // does that), and begins afresh at the start point.
  const instrumentInUse = phase.kind === "practicing" ? phase.instrument : null;
  const restart = useCallback(() => {
    if (instrumentInUse) begin(instrumentInUse);
  }, [instrumentInUse, begin]);

  // R restarts, unless it is being typed into something.
  useEffect(() => {
    if (!instrumentInUse) return;

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key.toLowerCase() !== "r" || event.ctrlKey || event.metaKey || event.altKey) return;
      if (typing(event)) return;
      event.preventDefault();
      restart();
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [instrumentInUse, restart]);

  const chooseAnother = useCallback(() => {
    setPhase({ kind: "choosing", refusal: null });
  }, []);

  // Leaving ends an attempt still running (kept if anything was played)
  // instead of letting it carry on unseen, and puts the start point back at
  // bar 1, running or not. Leaving by the button does it at once; this covers
  // every other way out. Nothing running answers with a refusal, ignored.
  useEffect(
    () => () => {
      void endAttempt("left").catch(() => undefined);
    },
    [],
  );

  const leave = useCallback(() => {
    void endAttempt("left").catch(() => undefined);
    onLeave();
  }, [onLeave]);

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
          onClick={leave}
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

        <div className="flex shrink-0 items-center gap-2">
          <button
            type="button"
            onClick={toggleLoop}
            disabled={keyboardLost}
            aria-pressed={loop !== null}
            title={loop ? "Stop looping (L)" : "Loop this bar (L)"}
            className={`cursor-pointer rounded-md px-3 py-1.5 text-xs font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent disabled:cursor-default disabled:opacity-40 ${
              loop ? "bg-loop-band/8 text-ink hover:bg-loop-band/14" : "text-ink-muted hover:bg-ink/5 hover:text-accent"
            }`}
          >
            {loop ? `⟲ Loop ${loop.first === loop.last ? loop.first : `${loop.first}–${loop.last}`}` : "⟲ Loop"}
          </button>
          {loop && (
            <>
              <span role="status" className="text-xs text-ink-muted tabular-nums">
                {cleanInARow} clean in a row
              </span>
              <label className="flex items-center gap-1 text-xs text-ink-muted">
                stop at
                <select
                  value={target ?? ""}
                  onChange={(event) =>
                    chooseTarget(event.target.value === "" ? null : Number(event.target.value))
                  }
                  className="cursor-pointer rounded-md bg-transparent px-1 py-0.5 text-xs text-ink hover:bg-ink/5 focus-visible:outline-2 focus-visible:outline-accent"
                >
                  {Array.from({ length: 10 }, (_, index) => index + 1).map((count) => (
                    <option key={count} value={count}>
                      {count}
                    </option>
                  ))}
                  <option value="">never</option>
                </select>
              </label>
            </>
          )}
        </div>

        {phase.kind === "practicing" && (
          <button
            type="button"
            onClick={restart}
            title={loop ? "Start this pass again (R)" : "Start again from the start (R)"}
            className="cursor-pointer rounded-md px-3 py-1.5 text-xs font-medium text-ink-muted transition-colors hover:bg-ink/5 hover:text-accent focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
          >
            Restart
          </button>
        )}

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

        {moveRefusal && (
          <motion.div
            key="move-refusal"
            variants={PANEL}
            initial="hidden"
            animate="shown"
            exit="hidden"
            transition={{ duration: 0.16, ease: "easeOut" }}
            className="shrink-0 overflow-hidden px-8 pt-4"
          >
            <p role="status" className="panel rounded-2xl px-5 py-3 text-sm text-ink-muted">
              {moveRefusal}
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
      >
        <Score
          hidden={!fitted}
          pages={engraving.pages}
          marks={engraving.marks}
          bars={engraving.bars}
          // No keyboard, nowhere to start: clicks wait for it to come back.
          onBarClick={keyboardLost ? undefined : onBarClick}
          startPosition={start.position}
          loop={loop}
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

      <div className="flex min-w-48 max-w-80 flex-1">
        <DeviceSelect
          label="Instrument"
          value={chosen}
          options={available.map((instrument) => instrument.name)}
          onChange={setChosen}
        />
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
