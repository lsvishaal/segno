import { useCallback, useEffect, useMemo, useRef, useState } from "react";

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
  weakBars,
} from "../bridge";
import type {
  AttemptSummary,
  Bar,
  InstrumentSummary,
  Mark,
  OpenedPiece,
  PracticeEvent,
  ReviewCard,
  StartPoint,
  Suggested,
  WeakBars,
} from "../bridge";
import { DeviceSelect } from "../components/DeviceSelect";
import { Results } from "../components/Results";
import { Score } from "../components/Score";
import type { CursorState, Dot, ScoreHandle, ScoreReview } from "../components/Score";
import type { CutOutRequest } from "../components/scoreCutout";
import { BEFORE_STARTING, cursorFor, dotFor } from "./overlayEvents";
import { SettingsPanel } from "./PracticeSettings";
import { REFLOW_THRESHOLD, clampScale, loadZoom, saveZoom, scoreWidth, stepScale } from "./zoom";

/** How long to wait after the last zoom or width change before re-engraving —
 * a real Verovio pass, so not on every wheel tick or resize frame. */
const REFLOW_DEBOUNCE_MS = 250;

/** How long after the last note of the piece the run finishes by itself. */
const AUTO_FINISH_MS = 1500;

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
 * nothing: `correct`/`wrong` on a `Played` event is computed in
 * `hogwash-app`'s practice thread from `Cursor::expecting()`, never derived
 * here from whether the Cursor moved.
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
  // The bars that have been hard over time, tinted while not playing; W hides them.
  const [weak, setWeak] = useState<WeakBars | null>(null);
  const [showWeak, setShowWeak] = useState(true);
  const [playing, setPlaying] = useState(false);
  const [available, setAvailable] = useState<readonly InstrumentSummary[]>([]);
  const [cursor, setCursor] = useState<CursorState>(BEFORE_STARTING);
  // Where attempts start. Held by the practice thread; this is its echo.
  const [start, setStart] = useState<StartPoint>({ position: 0, measure: 1 });
  // Something said about a move or a loop, for a few seconds.
  const [moveRefusal, setMoveRefusal] = useState<string | null>(null);
  // After a loop's goal: the loop a bar wider, offered beside that message.
  const [widen, setWiden] = useState<Suggested | null>(null);
  // After a run: the score with its marks, instead of the results.
  const [showingScore, setShowingScore] = useState(false);
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
  const scoreApi = useRef<ScoreHandle>(null);

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

    const timeout = window.setTimeout(() => {
      setMoveRefusal(null);
      setWiden(null);
    }, widen ? 10000 : 5000);
    return () => window.clearTimeout(timeout);
  }, [moveRefusal, widen]);

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
        // No instrument connected is ordinary; the picker below
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

  const refreshWeak = useCallback(() => {
    weakBars(piece.id).then(setWeak, () => setWeak(null));
  }, [piece.id]);
  useEffect(refreshWeak, [refreshWeak]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    void listenToPractice((event: PracticeEvent) => {
      setCursor((current) => cursorFor(event, current));

      const next = dotFor(event);
      if (next) setDot(next);

      switch (event.kind) {
        case "played":
          setPlaying(true);
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
          setWiden(event.widen);
          break;
        case "ended":
          setLost(null);
          // The review colours the notes now; a flash still fading would undo one.
          setDot(null);
          setShowingScore(false);
          setPlaying(false);
          refreshWeak();
          if (event.summary.review.bars.length === 0) {
            setMoveRefusal(event.summary.reason ?? "Nothing played.");
            setPhase((current) =>
              current.kind === "practicing" ? { kind: "choosing", refusal: null } : current,
            );
            break;
          }
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
  }, [refreshWeak]);

  const begin = useCallback(
    (instrument: string, from: number = start.position) => {
      void (async () => {
        try {
          await startAttempt(pieceId, instrument);
          setCursor({ position: from });
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
  // Answers where the loop starts, or null if it could not.
  const loopTo = useCallback(
    async (first: number, last: number): Promise<number | null> => {
      try {
        const range = await loopBars(pieceId, first, last, target);
        setStart({ position: range.position, measure: range.first });
        setLoop({ first: range.first, last: range.last });
        setCleanInARow(0);
        setCursor({ position: range.position });
        setDot(null);
        setWiden(null);
        setPhase((current) =>
          current.kind === "practicing" ? { ...current, reachedEnd: false } : current,
        );
        return range.position;
      } catch (error) {
        setMoveRefusal(String(error));
        return null;
      }
    },
    [pieceId, target],
  );

  // A click starts there; Shift+click loops from the start bar to there.
  const onBarClick = useCallback(
    (measure: number, how: { readonly shift: boolean }) => {
      if (how.shift) void loopTo(start.measure, measure);
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
    else void loopTo(cursorBar, cursorBar);
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
      } else if (event.key.toLowerCase() === "w") {
        setShowWeak((shown) => !shown);
      } else if (event.key === "Escape" && loop && !settingsOpen && phase.kind !== "ended") {
        stopLooping();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [toggleLoop, stopLooping, loop, settingsOpen, keyboardLost, phase.kind]);

  const reachedEnd = phase.kind === "practicing" && phase.reachedEnd;
  const finish = useCallback(() => {
    void endAttempt(reachedEnd ? "finished" : "stop");
  }, [reachedEnd]);

  // The last note played (not looping): the run finishes by itself a moment
  // later, so the last chord rings and a trill's tail isn't cut off. Every
  // note heard meanwhile (each one is a new `dot`) starts the wait again;
  // Stop, R or a click on a bar leave the end and so cancel it.
  useEffect(() => {
    if (!reachedEnd || loop) return;
    const timeout = window.setTimeout(() => void endAttempt("finished"), AUTO_FINISH_MS);
    return () => window.clearTimeout(timeout);
  }, [reachedEnd, loop, dot]);

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

  // The review's suggestion: loop the bars it names and start playing them.
  const practise = useCallback(
    (suggested: Suggested, instrument: string) => {
      const { first, last } = suggested;
      if (first === null || last === null) {
        begin(instrument);
        return;
      }
      void (async () => {
        // Where the loop starts, which `start` will only say after this render.
        const at = await loopTo(first, last);
        if (at !== null) begin(instrument, at);
      })();
    },
    [begin, loopTo],
  );

  // A card's button: start at its bar, or loop the bars it is about, and
  // play.
  const practiseCard = useCallback(
    (card: ReviewCard, instrument: string) => {
      const { drillFirst: first, drillLast: last } = card;
      if (first === null || last === null) {
        begin(instrument);
      } else if (card.kind === "keepsStopping") {
        void (async () => {
          try {
            const point = await startAtBar(pieceId, first);
            setStart(point);
            setLoop(null);
            begin(instrument, point.position);
          } catch (error) {
            setMoveRefusal(String(error));
          }
        })();
      } else {
        practise({ label: card.button ?? "", first, last }, instrument);
      }
    },
    [begin, practise, pieceId],
  );

  // What the results ask of the drawn score.
  const cutOut = useCallback(
    (request: CutOutRequest) => scoreApi.current?.cutOut(request) ?? null,
    [],
  );

  // A column of the review's strip: that bar, brought into view.
  const showBar = useCallback((measure: number) => scoreApi.current?.showBar(measure), []);

  // Back to the score from the results, its marks left on; at a bar if one
  // was picked on the chart.
  const showScore = useCallback(
    (measure?: number) => {
      setShowingScore(true);
      if (measure !== undefined) window.setTimeout(() => showBar(measure), 220);
    },
    [showBar],
  );

  const resultsShown = phase.kind === "ended" && !showingScore;

  // On the results: Enter takes the first card's drill (or plays again), R plays
  // again, Esc shows the score. On the score after a run, Esc goes back.
  useEffect(() => {
    if (phase.kind !== "ended" || settingsOpen) return;
    const { instrument, summary } = phase;

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.ctrlKey || event.metaKey || event.altKey || typing(event)) return;
      if (event.key === "Escape") {
        event.preventDefault();
        setShowingScore((current) => !current);
      } else if (!showingScore && event.key === "Enter") {
        // A focused button answers Enter itself.
        if (event.target instanceof HTMLButtonElement) return;
        event.preventDefault();
        const card = summary.review.cards[0];
        if (card && card.drillFirst !== null) practiseCard(card, instrument);
        else again(instrument);
      } else if (event.key.toLowerCase() === "r") {
        event.preventDefault();
        again(instrument);
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [phase, showingScore, settingsOpen, practiseCard, again]);

  // What stays drawn on the score while the review is showing.
  const review = phase.kind === "ended" ? phase.summary.review : null;
  const scoreReview = useMemo<ScoreReview | null>(
    () =>
      review && {
        marks: review.marks,
        notReached: review.bars.filter((bar) => bar.state === "notReached").map((bar) => bar.measure),
      },
    [review],
  );

  // Off while playing and while this run's marks are on the score: the
  // amber is "hard over time", the marks "this run", never both at once.
  const weakShown = showWeak && !playing && !review && weak !== null && weak.runs > 0;
  const weakLevels = useMemo(
    () => new Map(weak?.bars.filter((bar) => bar.level > 0).map((bar) => [bar.measure, bar.level])),
    [weak],
  );

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

        <StatusLine
          lost={phase.kind === "practicing" ? lost : null}
          message={moveRefusal}
          widen={phase.kind === "practicing" ? widen : null}
          onWiden={(suggested) => void loopTo(suggested.first ?? 1, suggested.last ?? 1)}
          afterRun={phase.kind === "ended" && showingScore}
          onResults={() => setShowingScore(false)}
        />

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

      </AnimatePresence>

      <div className="relative min-h-0 flex-1">
        <div
          ref={scoreRef}
          className="hw-score absolute inset-0 px-4 py-6 sm:px-8 sm:py-8 lg:px-12 lg:py-10"
          style={{ visibility: resultsShown ? "hidden" : undefined }}
          aria-hidden={resultsShown || undefined}
        >
          <Score
            ref={scoreApi}
            hidden={!fitted}
            pages={engraving.pages}
            marks={engraving.marks}
            bars={engraving.bars}
            // No keyboard, nowhere to start: clicks wait for it to come back.
            onBarClick={keyboardLost ? undefined : onBarClick}
            startPosition={start.position}
            loop={loop}
            review={scoreReview}
            weak={weakShown ? weakLevels : null}
            cursor={cursor}
            dot={dot}
            showNoteNames={showNoteNames}
            showGeometry={showGeometry}
          />
        </div>

        <AnimatePresence>
          {weakShown && <WeakLegend weak={weak} hasWeak={weakLevels.size > 0} />}
        </AnimatePresence>

        <AnimatePresence>
          {resultsShown && (
            <motion.div
              key="results"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.2, ease: "easeOut" }}
              className="absolute inset-0 flex overflow-y-auto"
            >
              {/* Centred when it fits; scrolls from the top when it doesn't. */}
              <div className="m-auto w-full">
                <Results
                  review={phase.summary.review}
                  reason={phase.summary.autoEnded ? phase.summary.reason : null}
                  cutOut={cutOut}
                  onCard={(card) => practiseCard(card, phase.instrument)}
                  onNoteNames={() => setShowNoteNames(true)}
                  onAgain={() => again(phase.instrument)}
                  onShowScore={showScore}
                  onLibrary={leave}
                />
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}

/**
 * One slim line in the header for anything worth saying while practising,
 * so nothing ever sits on top of the score: the keyboard gone, a refusal,
 * a loop done (with its next step), or that the score shows the last run.
 */
function StatusLine({
  lost,
  message,
  widen,
  onWiden,
  afterRun,
  onResults,
}: {
  readonly lost: { instrument: string; secondsRemaining: number } | null;
  readonly message: string | null;
  readonly widen: Suggested | null;
  readonly onWiden: (suggested: Suggested) => void;
  readonly afterRun: boolean;
  readonly onResults: () => void;
}) {
  const link =
    "ml-2 cursor-pointer rounded-full border border-rule px-2.5 py-0.5 text-xs text-ink transition-colors hover:border-accent hover:text-accent focus-visible:outline-2 focus-visible:outline-accent";
  // A new message fades in rather than snapping.
  const showing = lost ? "lost" : (message ?? (afterRun ? "afterRun" : ""));
  return (
    <p role="status" className="min-w-0 shrink truncate text-xs text-ink-muted">
      <span key={showing} className="hw-fade-in">
      {lost ? (
        `${lost.instrument} stopped reporting · waiting ${lost.secondsRemaining} s · what you played is kept`
      ) : message ? (
        <>
          {message}
          {widen && (
            <button type="button" onClick={() => onWiden(widen)} className={link}>
              Join it back up · {widen.label}
            </button>
          )}
        </>
      ) : afterRun ? (
        <>
          Your last run is marked on the score
          <button type="button" onClick={onResults} className={link}>
            Results (Esc)
          </button>
        </>
      ) : null}
      </span>
    </p>
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

/** Lets the performer pick which connected instrument to play on. */
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

/** What the amber on the score means, in the corner while it shows. */
function WeakLegend({ weak, hasWeak }: { readonly weak: WeakBars; readonly hasWeak: boolean }) {
  const runs = `from ${weak.runs} ${weak.runs === 1 ? "run" : "runs"}`;
  return (
    <motion.div
      data-weak-legend
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.16, ease: "easeOut" }}
      className="pointer-events-none absolute right-4 bottom-3 flex items-center gap-2 rounded-md border border-rule bg-paper-raised/90 px-3 py-1.5 text-xs text-ink-muted"
    >
      {hasWeak ? (
        <>
          <span className="flex gap-0.5" aria-hidden="true">
            {["bg-pause/8", "bg-pause/16", "bg-pause/26", "bg-pause/38"].map((tint) => (
              <span key={tint} className={`h-3 w-3 rounded-sm ${tint}`} />
            ))}
          </span>
          <span>
            Weak bars {runs} · lighter = easier for you, deeper = harder · <kbd>W</kbd> to hide
          </span>
        </>
      ) : (
        <span>No weak bars yet ({runs})</span>
      )}
    </motion.div>
  );
}
