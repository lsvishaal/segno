import { useCallback, useEffect, useState } from "react";

import { AnimatePresence, motion } from "motion/react";

import * as piano from "../audio/piano";
import {
  endAttempt,
  instruments as fetchInstruments,
  listenToPractice,
  startAttempt,
} from "../bridge";
import type { AttemptSummary, InstrumentSummary, OpenedPiece, PracticeEvent } from "../bridge";
import { Score } from "../components/Score";
import type { CursorState, Dot } from "../components/Score";
import { BEFORE_STARTING, cursorFor, dotFor } from "./overlayEvents";

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
  const [phase, setPhase] = useState<Phase>({ kind: "choosing", refusal: null });
  const [available, setAvailable] = useState<readonly InstrumentSummary[]>([]);
  const [cursor, setCursor] = useState<CursorState>(BEFORE_STARTING);
  const [dot, setDot] = useState<Dot | null>(null);
  const [lost, setLost] = useState<{ instrument: string; secondsRemaining: number } | null>(
    null,
  );

  // A piece switch (or leaving and reopening the same one) starts fresh.
  const pieceId = piece.id;
  useEffect(() => {
    setPhase({ kind: "choosing", refusal: null });
    setCursor(BEFORE_STARTING);
    setDot(null);
    setLost(null);
  }, [pieceId]);

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
          piano.play(event.pitch);
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
      // The click that starts an attempt is also the user gesture browsers
      // require before audio can play at all — there is no separate "enable
      // sound" step to ask for.
      piano.unlock();

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

        {phase.kind === "practicing" && (
          <button
            type="button"
            onClick={finish}
            className="cursor-pointer rounded-md border border-rule px-3 py-1 text-xs font-medium transition-colors hover:border-ink hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
          >
            {phase.reachedEnd ? "Finish" : "Stop"}
          </button>
        )}

        <fieldset className="glass flex shrink-0 items-center gap-0.5 rounded-full p-0.5">
          <legend className="sr-only">Score appearance</legend>
          {(["paper", "dark"] as const).map((option) => (
            <button
              key={option}
              type="button"
              aria-pressed={surface === option}
              onClick={() => setSurface(option)}
              className={`cursor-pointer rounded-full px-2.5 py-1 text-xs capitalize transition-colors focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent ${
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
            <p className="glass rounded-2xl px-5 py-3 text-sm text-ink-muted">
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

      <div className="hw-score min-h-0 flex-1 px-8 py-6" data-surface={surface}>
        <Score pages={piece.pages} marks={piece.marks} cursor={cursor} dot={dot} />
      </div>
    </div>
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
      <div className="glass shrink-0 rounded-2xl px-6 py-4 text-sm text-ink-muted">
        No MIDI keyboard is connected. Plug one in — Hogwash will notice once you try
        again.
        {refusal && <p className="mt-1 text-ink-muted">{refusal}</p>}
      </div>
    );
  }

  return (
    <div className="glass flex shrink-0 flex-wrap items-center gap-3 rounded-2xl px-6 py-4">
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

      <button
        type="button"
        disabled={!chosen}
        onClick={() => chosen && onChoose(chosen)}
        className="cursor-pointer rounded-full bg-ink px-4 py-1.5 text-sm font-medium text-paper transition-opacity hover:opacity-90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent disabled:cursor-not-allowed disabled:opacity-40"
      >
        Start
      </button>
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
    <div className="glass shrink-0 rounded-2xl px-6 py-5">
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
        <button
          type="button"
          onClick={onAgain}
          className="cursor-pointer rounded-full bg-ink px-4 py-1.5 text-sm font-medium text-paper transition-opacity hover:opacity-90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
        >
          Practice again
        </button>
        <button
          type="button"
          onClick={onChooseAnother}
          className="cursor-pointer rounded-full border border-rule px-4 py-1.5 text-sm transition-colors hover:border-ink hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
        >
          Choose a different instrument
        </button>
      </div>
    </div>
  );
}
