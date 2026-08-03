import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  endAttempt,
  instruments as fetchInstruments,
  listenToPractice,
  startAttempt,
} from "../bridge";
import type { AttemptSummary, InstrumentSummary, OpenedPiece, PracticeEvent } from "../bridge";
import { Score } from "../components/Score";
import type { Marked } from "../components/Score";
import { BEFORE_STARTING, markFor } from "./practiceMarks";

/** Whether the score is drawn on paper or inverted. */
export type Surface = "paper" | "dark";

/** Where the practice loop is, for this piece. */
type Phase =
  | { readonly kind: "choosing"; readonly refusal: string | null }
  | { readonly kind: "practicing"; readonly instrument: string; readonly reachedEnd: boolean }
  | { readonly kind: "ended"; readonly instrument: string; readonly summary: AttemptSummary };

/**
 * The practice surface.
 *
 * Everything it says about the music, and about how it was played, comes
 * from Rust — the engraved score, the marks, and every `PracticeEvent`. This
 * component turns them into a `Marked` map and a phase; it judges nothing
 * (`004 §8.8`). In particular `correct`/`wrong` on a `Played` event is
 * computed in `hogwash-app`'s practice thread from `Cursor::expecting()`,
 * never derived here from whether the Cursor moved (`003 §7.16`).
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
  const [marked, setMarked] = useState<Marked>(BEFORE_STARTING);
  const [lost, setLost] = useState<{ instrument: string; secondsRemaining: number } | null>(
    null,
  );

  // A piece switch (or leaving and reopening the same one) starts fresh.
  const pieceId = piece.id;
  useEffect(() => {
    setPhase({ kind: "choosing", refusal: null });
    setMarked(BEFORE_STARTING);
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
      const next = markFor(event);
      if (next) setMarked(next);

      switch (event.kind) {
        case "played":
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
          setMarked(BEFORE_STARTING);
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

      {phase.kind === "choosing" && (
        <InstrumentPicker
          available={available}
          refusal={phase.refusal}
          onChoose={begin}
        />
      )}

      {lost && phase.kind === "practicing" && (
        <p className="shrink-0 border-b border-rule bg-paper-raised px-8 py-2 text-sm text-ink-muted">
          {lost.instrument} stopped reporting. Waiting {lost.secondsRemaining}s for it to
          reconnect before ending the attempt — what you have played is kept either way.
        </p>
      )}

      {phase.kind === "ended" && (
        <SummaryPanel
          summary={phase.summary}
          onAgain={() => again(phase.instrument)}
          onChooseAnother={chooseAnother}
        />
      )}

      <div
        className="hw-score min-h-0 flex-1 px-8 py-6"
        data-surface={surface}
      >
        <Score pages={piece.pages} marks={piece.marks} marked={marked} />
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
  const chosen = useRef<string | null>(available[0]?.name ?? null);

  const options = useMemo(() => available, [available]);

  if (options.length === 0) {
    return (
      <div className="shrink-0 border-b border-rule px-8 py-4 text-sm text-ink-muted">
        No MIDI keyboard is connected. Plug one in — Hogwash will notice once you try
        again.
        {refusal && <p className="mt-1 text-ink-muted">{refusal}</p>}
      </div>
    );
  }

  return (
    <form
      className="flex shrink-0 flex-wrap items-center gap-3 border-b border-rule px-8 py-4"
      onSubmit={(event) => {
        event.preventDefault();
        if (chosen.current) onChoose(chosen.current);
      }}
    >
      <label className="text-sm text-ink-muted" htmlFor="instrument">
        Play on
      </label>
      <select
        id="instrument"
        defaultValue={options[0]?.name}
        onChange={(event) => {
          chosen.current = event.currentTarget.value;
        }}
        className="rounded-md border border-rule bg-paper px-2 py-1 text-sm"
      >
        {options.map((instrument) => (
          <option key={instrument.name} value={instrument.name}>
            {instrument.name}
          </option>
        ))}
      </select>
      <button
        type="submit"
        className="cursor-pointer rounded-md bg-ink px-3 py-1 text-sm font-medium text-paper transition-opacity hover:opacity-90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
      >
        Start
      </button>
      {refusal && <p className="text-sm text-ink-muted">{refusal}</p>}
    </form>
  );
}

/**
 * What an ended attempt has to say.
 *
 * Every line comes from `Feedback::lines()`, verbatim (`004 §8.8`). This is
 * Stage D's minimum to close the loop — the finished screen is Stage E's.
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
    <div className="shrink-0 border-b border-rule bg-paper-raised px-8 py-5">
      {summary.autoEnded && summary.reason && (
        <p className="mb-2 text-sm text-ink-muted">{summary.reason}</p>
      )}
      <ul className="space-y-1 text-sm">
        {summary.lines.map((line, index) => (
          // The lines are an ordered, fixed-shape report from Rust, not a
          // list a key could meaningfully be drawn from.
          // eslint-disable-next-line react/no-array-index-key
          <li key={index}>{line}</li>
        ))}
      </ul>
      <div className="mt-4 flex gap-3">
        <button
          type="button"
          onClick={onAgain}
          className="cursor-pointer rounded-md bg-ink px-3 py-1 text-sm font-medium text-paper transition-opacity hover:opacity-90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
        >
          Practice again
        </button>
        <button
          type="button"
          onClick={onChooseAnother}
          className="cursor-pointer rounded-md border border-rule px-3 py-1 text-sm transition-colors hover:border-ink hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
        >
          Choose a different instrument
        </button>
      </div>
    </div>
  );
}
