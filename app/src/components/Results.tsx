import { useCallback, useState } from "react";

import { motion } from "motion/react";

import type { Numbers, Review, ReviewBar, ReviewCard, ReviewPass } from "../bridge";
import { BarCutout } from "./BarCutout";
import type { CutOutRequest } from "./scoreCutout";
import { barLabel, barName } from "../bars";

/**
 * The results after a run, where the score was: what to work on (at most
 * three cards, each with the bars cut out of your score, what happened, why,
 * and how to fix it), what went well, and what to do next. The numbers and
 * the chart wait under Details. Every word and number comes from Rust; this
 * only lays them out.
 */
/** The results come in top to bottom, 50 ms apart. */
const STAGGER = { hidden: {}, shown: { transition: { staggerChildren: 0.05 } } };
const ITEM = {
  hidden: { opacity: 0, y: 6 },
  shown: { opacity: 1, y: 0, transition: { duration: 0.2, ease: "easeOut" } },
} as const;

export function Results({
  review,
  reason,
  cutOut,
  onCard,
  onNoteNames,
  onAgain,
  onShowScore,
  onProgress,
  onLibrary,
}: {
  readonly review: Review;
  /** Why the run ended by itself, if it did. */
  readonly reason: string | null;
  /** Bars cut out of the score, laid out behind the results. */
  readonly cutOut: (request: CutOutRequest) => SVGSVGElement | null;
  /** A card's button: practise what it is about. */
  readonly onCard: (card: ReviewCard) => void;
  /** Turn note names on, from a card about reading. */
  readonly onNoteNames: () => void;
  readonly onAgain: () => void;
  /** Back to the score, its marks left on; at a bar, if one was picked. */
  readonly onShowScore: (measure?: number) => void;
  /** The piece's Progress page: every session, and what to work on next. */
  readonly onProgress: () => void;
  readonly onLibrary: () => void;
}) {
  const { numbers } = review;
  const [main, ...more] = review.cards;
  const toFix = review.cards.some((card) => card.kind !== "clean");

  return (
    <motion.section
      aria-label="Results"
      variants={STAGGER}
      initial="hidden"
      animate="shown"
      className="mx-auto flex w-full max-w-[1040px] flex-col gap-8 px-8 py-10"
    >
      <motion.header variants={ITEM} className="flex flex-wrap items-baseline justify-between gap-x-8 gap-y-2">
        <p className="text-subtitle text-ink">{review.headline}</p>
        <p className="font-mono text-xs text-ink-muted tabular-nums">
          {[
            numbers.accuracy !== null && `accuracy ${numbers.accuracy}%`,
            numbers.tempo !== null && (numbers.rhythm ? `♩ ${numbers.tempo}` : `${numbers.tempo} notes/min`),
            numbers.steadiness !== null && `steady ${numbers.steadiness}%`,
            numbers.restarts > 0 && `restarts ${numbers.restarts}`,
          ]
            .filter(Boolean)
            .join("   ")}
        </p>
      </motion.header>

      {reason && <p className="text-sm text-ink-muted">{reason}</p>}
      {review.note && <p className="text-sm text-ink-muted">{review.note}</p>}

      {main && (
        <motion.div variants={ITEM} className="flex flex-col gap-4">
          {toFix && <h2 className="text-label text-ink-muted uppercase">Work on these</h2>}
          <CoachCard
            card={main}
            number={toFix ? 1 : null}
            large
            marks={review.marks}
            cutOut={cutOut}
            onCard={onCard}
            onNoteNames={onNoteNames}
            onShowScore={onShowScore}
          />
          {more.length > 0 && (
            <div className="grid gap-4 md:grid-cols-2">
              {more.map((card, index) => (
                <CoachCard
                  key={`${card.kind}-${card.first}`}
                  card={card}
                  number={index + 2}
                  large={false}
                  marks={review.marks}
                  cutOut={cutOut}
                  onCard={onCard}
                  onNoteNames={onNoteNames}
                  onShowScore={onShowScore}
                />
              ))}
            </div>
          )}
        </motion.div>
      )}

      {review.wentWell.length > 0 && (
        <motion.div variants={ITEM} className="flex flex-wrap items-center gap-2" aria-label="Went well">
          <h2 className="mr-2 text-label text-ink-muted uppercase">Went well</h2>
          {review.wentWell.map((line) => (
            <span key={line} className="rounded-full border border-rule px-3 py-1 text-sm text-ink">
              <span aria-hidden="true" className="mr-1.5 text-correct">✓</span>
              {line}
            </span>
          ))}
        </motion.div>
      )}

      <motion.div variants={ITEM} className="flex flex-wrap items-center gap-3">
        <motion.button
          type="button"
          onClick={onAgain}
          whileTap={{ scale: 0.96 }}
          title={toFix ? "R" : "Enter or R"}
          className={toFix ? SECONDARY : PRIMARY}
        >
          ↻ Practice again
        </motion.button>
        <button type="button" onClick={() => onShowScore()} title="Esc" className={SECONDARY}>
          ♪ Show on the score
        </button>
        <button type="button" onClick={onProgress} className={SECONDARY}>
          Progress →
        </button>
        <button type="button" onClick={onLibrary} className={SECONDARY}>
          ← Library
        </button>
      </motion.div>

      <motion.div variants={ITEM}>
        <Details review={review} onShowScore={onShowScore} />
      </motion.div>
    </motion.section>
  );
}

/** One thing to work on: the bars, what happened, why, how to fix it, and a button. */
function CoachCard({
  card,
  number,
  large,
  marks,
  cutOut,
  onCard,
  onNoteNames,
  onShowScore,
}: {
  readonly card: ReviewCard;
  /** Its place in the list; none for a clean run. */
  readonly number: number | null;
  readonly large: boolean;
  readonly marks: Review["marks"];
  readonly cutOut: (request: CutOutRequest) => SVGSVGElement | null;
  readonly onCard: (card: ReviewCard) => void;
  readonly onNoteNames: () => void;
  readonly onShowScore: (measure?: number) => void;
}) {
  const draw = useCallback(
    () => cutOut({ first: card.first, last: card.last, marks, pauseInto: card.pauseInto }),
    [cutOut, card.first, card.last, marks, card.pauseInto],
  );
  return (
    <article
      aria-label={card.title}
      data-card={card.kind}
      className={`panel flex gap-6 rounded-2xl p-6 ${large ? "flex-col md:flex-row" : "flex-col"}`}
    >
      <div className={large ? "h-40 shrink-0 md:h-auto md:min-h-40 md:w-[44%]" : "h-28 shrink-0"}>
        <BarCutout first={card.first} last={card.last} draw={draw} onPick={onShowScore} />
      </div>

      <div className="flex min-w-0 flex-1 flex-col gap-3">
        <h3 className={`font-serif ${large ? "text-heading" : "text-title"} leading-tight`}>
          {number !== null && <span className="mr-2 font-mono text-base text-ink-muted">{number}</span>}
          {card.title}
        </h3>
        <p className="text-sm text-ink">{card.what}</p>
        {card.why && <p className="text-sm text-ink-muted">{card.why}</p>}
        {card.steps.length > 0 && (
          <ol className="list-decimal space-y-1 pl-5 text-sm text-ink marker:font-mono marker:text-ink-muted">
            {card.steps.map((step) => (
              <li key={step}>{step}</li>
            ))}
          </ol>
        )}
        {card.also && <p className="text-xs text-ink-muted">{card.also}</p>}
        {card.button && (
          <div className="mt-auto flex flex-wrap gap-2 pt-1">
            <motion.button
              type="button"
              onClick={() => onCard(card)}
              whileTap={{ scale: 0.96 }}
              title={number === 1 ? "Enter" : undefined}
              className={number === 1 ? PRIMARY : SECONDARY}
            >
              ⟲ {card.button}
            </motion.button>
            {card.kind === "wrongNotes" && (
              <button type="button" onClick={onNoteNames} className={SECONDARY}>
                Turn on note names
              </button>
            )}
          </div>
        )}
      </div>
    </article>
  );
}

/** The numbers and the chart, for whoever wants them. */
function Details({
  review,
  onShowScore,
}: {
  readonly review: Review;
  readonly onShowScore: (measure?: number) => void;
}) {
  const { numbers } = review;
  const points = review.passes.length > 0 ? passPoints(review.passes) : barPoints(review.bars);
  const charted = points.filter((point) => point.tempo !== null || point.wrong > 0).length >= 2;

  return (
    <details className="group">
      <summary className="cursor-pointer text-sm text-ink-muted select-none hover:text-accent">Details</summary>
      <div className="hw-details-body mt-6 flex flex-col gap-8">
        <div className="flex flex-col gap-8 sm:flex-row sm:items-stretch">
          <div className="flex shrink-0 flex-col gap-6 sm:w-48">
            <Big label="accuracy" value={numbers.accuracy === null ? "—" : `${numbers.accuracy}%`} />
            <Big
              label={numbers.rhythm ? "tempo · ♩ / min" : "notes / min"}
              value={numbers.tempo === null ? "—" : String(numbers.tempo)}
            />
          </div>
          {charted && (
            <div className="min-w-0 flex-1">
              <Chart
                points={points}
                byPass={review.passes.length > 0}
                tempoLabel={numbers.rhythm ? "♩ / min" : "notes / min"}
                onPick={onShowScore}
              />
            </div>
          )}
        </div>
        <Small numbers={numbers} />
      </div>
    </details>
  );
}

const PRIMARY =
  "cursor-pointer rounded-full bg-ink px-4 py-1.5 text-sm font-medium text-paper transition-opacity duration-[120ms] hover:opacity-90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent";
const SECONDARY =
  "cursor-pointer rounded-full border border-rule px-4 py-1.5 text-sm transition-colors duration-[120ms] hover:border-accent hover:text-accent focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent";

/** A label in small caps over a big number. */
function Big({
  label,
  value,
}: {
  readonly label: string;
  readonly value: string;
}) {
  return (
    <div>
      <p className="text-label text-ink-muted uppercase">{label}</p>
      <p className="font-mono text-display text-accent tabular-nums">{value}</p>
    </div>
  );
}

/** The row of smaller numbers under the chart. */
function Small({ numbers }: { readonly numbers: Numbers }) {
  const items: {
    label: string;
    value: string;
    note?: string;
    title?: string;
  }[] = [
    {
      label: "notes",
      value: `${numbers.right}/${numbers.wrong}/${numbers.missed}`,
      title: "right / wrong / not reached",
    },
    {
      label: "steadiness",
      value: numbers.steadiness === null ? "—" : `${numbers.steadiness}%`,
    },
    {
      label: "longest pause",
      value:
        numbers.longestPauseBar === null
          ? "—"
          : seconds(numbers.longestPauseMs),
      ...(numbers.longestPauseBar === null
        ? {}
        : { note: barName(numbers.longestPauseBar) }),
    },
    {
      label: "time",
      value: numbers.timeMs === null ? "—" : duration(numbers.timeMs),
    },
  ];
  if (numbers.restarts > 0)
    items.push({ label: "restarts", value: String(numbers.restarts) });

  return (
    <dl className="grid grid-cols-2 gap-x-8 gap-y-4 sm:grid-cols-5">
      {items.map((item) => (
        <div key={item.label} title={item.title}>
          <dt className="text-label text-ink-muted uppercase">{item.label}</dt>
          <dd className="font-mono text-title text-ink tabular-nums">
            {item.value}
            {item.note && (
              <span className="ml-2 text-xs text-ink-muted">{item.note}</span>
            )}
          </dd>
        </div>
      ))}
    </dl>
  );
}

/** One point on the chart: a bar, or a pass of a loop. */
interface Point {
  readonly label: string;
  /** The bar to show when picked; none for a pass. */
  readonly measure: number | null;
  readonly tempo: number | null;
  readonly wrong: number;
  readonly reached: boolean;
  readonly breaks: number;
  readonly clean: boolean;
  readonly line: string;
}

function barPoints(bars: readonly ReviewBar[]): Point[] {
  return bars.map((bar) => ({
    label: bar.measure === 0 ? barLabel(0) : String(bar.measure),
    measure: bar.measure,
    tempo: bar.tempo,
    wrong: bar.wrong,
    reached: bar.state !== "notReached",
    breaks: bar.breaks,
    clean: false,
    line:
      bar.state === "notReached"
        ? `${barLabel(bar.measure)} · not reached`
        : bar.line,
  }));
}

function passPoints(passes: readonly ReviewPass[]): Point[] {
  return passes.map((pass) => ({
    label: String(pass.number),
    measure: null,
    tempo: pass.tempo,
    wrong: pass.wrong,
    reached: true,
    breaks: 0,
    clean: pass.clean,
    line: pass.line,
  }));
}

/** Chart geometry, in the SVG's own units; it stretches to the width. */
const WIDTH = 1000;
const HEIGHT = 200;

/**
 * Tempo across the run, one point per bar (or per pass): dips are where you
 * slowed or stopped. Red ✕ along the top are wrong notes; the grey band is
 * what was not reached. Each point is a button, labelled with its line.
 */
function Chart({
  points,
  byPass,
  tempoLabel,
  onPick,
}: {
  readonly points: readonly Point[];
  readonly byPass: boolean;
  readonly tempoLabel: string;
  readonly onPick: (measure?: number) => void;
}) {
  const [hovered, setHovered] = useState<number | null>(null);
  const top = niceCeiling(
    Math.max(1, ...points.map((point) => point.tempo ?? 0)) * 1.15,
  );
  const step = WIDTH / points.length;
  const x = (index: number) => step * (index + 0.5);
  const y = (tempo: number) => HEIGHT - (tempo / top) * HEIGHT;
  const every = points.length <= 16 ? 1 : points.length <= 64 ? 4 : 8;

  // The line breaks where a bar has no tempo, so it never draws through
  // what wasn't played.
  const runs: string[] = [];
  let current: string[] = [];
  points.forEach((point, index) => {
    if (point.tempo === null) {
      if (current.length) runs.push(current.join(" "));
      current = [];
    } else {
      current.push(`${x(index)},${y(point.tempo)}`);
    }
  });
  if (current.length) runs.push(current.join(" "));

  const firstUnreached = points.findIndex((point) => !point.reached);
  const hoveredPoint = hovered === null ? null : points[hovered];

  return (
    <figure
      aria-label={
        byPass
          ? "Tempo and wrong notes, pass by pass"
          : "Tempo and wrong notes, bar by bar"
      }
    >
      <div className="flex gap-2">
        <div
          className="flex w-10 shrink-0 flex-col justify-between py-0 text-right font-mono text-[10px] text-ink-muted tabular-nums"
          aria-hidden="true"
        >
          <span>{top}</span>
          <span>{Math.round(top / 2)}</span>
          <span>0</span>
        </div>

        <div className="relative h-48 min-w-0 flex-1">
          <svg
            viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
            preserveAspectRatio="none"
            className="absolute inset-0 h-full w-full overflow-visible"
            aria-hidden="true"
          >
            {[0, 0.5, 1].map((at) => (
              <line
                key={at}
                x1={0}
                x2={WIDTH}
                y1={HEIGHT * at}
                y2={HEIGHT * at}
                className="stroke-rule"
                vectorEffect="non-scaling-stroke"
              />
            ))}
            {firstUnreached >= 0 && (
              <rect
                data-not-reached-band
                x={step * firstUnreached}
                y={0}
                width={WIDTH - step * firstUnreached}
                height={HEIGHT}
                className="fill-ink/5"
              />
            )}
            {runs.map((run) => (
              <polyline
                key={run}
                points={run}
                fill="none"
                className="stroke-accent"
                strokeWidth={2}
                strokeLinejoin="round"
                vectorEffect="non-scaling-stroke"
              />
            ))}
          </svg>

          {points.map((point, index) => (
            <div key={`${point.label}-${index}`} aria-hidden="true">
              {point.tempo !== null && (
                <span
                  className={`absolute size-2 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-accent ${
                    point.clean || !byPass ? "bg-accent" : "bg-paper-raised"
                  }`}
                  style={{
                    left: `${(x(index) / WIDTH) * 100}%`,
                    top: `${(y(point.tempo) / HEIGHT) * 100}%`,
                  }}
                />
              )}
              {point.wrong > 0 && (
                <span
                  data-chart-wrong
                  className="absolute -top-4 -translate-x-1/2 font-mono text-[11px] leading-none text-wrong"
                  style={{ left: `${(x(index) / WIDTH) * 100}%` }}
                >
                  ✕{point.wrong > 1 ? point.wrong : ""}
                </span>
              )}
              {point.breaks > 0 && (
                <span
                  className="absolute bottom-1 -translate-x-1/2 text-[10px] text-ink-muted"
                  style={{ left: `${(x(index) / WIDTH) * 100}%` }}
                >
                  ‖
                </span>
              )}
            </div>
          ))}

          <div className="absolute inset-0 flex" role="list">
            {points.map((point, index) => (
              <button
                key={`${point.label}-${index}`}
                type="button"
                role="listitem"
                aria-label={point.line}
                data-reached={point.reached}
                onPointerEnter={() => setHovered(index)}
                onPointerLeave={() =>
                  setHovered((current) => (current === index ? null : current))
                }
                onFocus={() => setHovered(index)}
                onBlur={() =>
                  setHovered((current) => (current === index ? null : current))
                }
                onClick={() => onPick(point.measure ?? undefined)}
                className="h-full min-w-0 flex-1 cursor-pointer hover:bg-ink/5 focus-visible:bg-ink/5 focus-visible:outline-none"
              />
            ))}
          </div>

          {hoveredPoint && hovered !== null && (
            <div
              role="tooltip"
              className="pointer-events-none absolute -top-10 z-10 -translate-x-1/2 rounded-lg border border-rule bg-paper-raised px-3 py-1.5 text-xs whitespace-nowrap text-ink shadow-sm"
              style={{
                left: `${Math.min(90, Math.max(10, (x(hovered) / WIDTH) * 100))}%`,
              }}
            >
              {hoveredPoint.line}
            </div>
          )}
        </div>
      </div>

      <div className="mt-1 flex gap-2" aria-hidden="true">
        <span className="w-10 shrink-0 text-right text-[10px] text-ink-muted">
          {tempoLabel}
        </span>
        <div className="flex min-w-0 flex-1">
          {points.map((point, index) => (
            <span
              key={`${point.label}-${index}`}
              className="min-w-0 flex-1 text-center font-mono text-[10px] text-ink-muted tabular-nums"
            >
              {index === 0 || (index + 1) % every === 0 ? point.label : ""}
            </span>
          ))}
        </div>
      </div>
      <figcaption className="mt-1 text-center text-label text-ink-muted uppercase">
        {byPass ? "pass" : "bar"}
      </figcaption>
    </figure>
  );
}

/** A round number at or above `value` for the chart's top: 20, 40, 60, … */
function niceCeiling(value: number): number {
  return Math.max(20, Math.ceil(value / 20) * 20);
}

function seconds(ms: number): string {
  return `${(ms / 1000).toFixed(1)} s`;
}

function duration(ms: number): string {
  const total = Math.round(ms / 1000);
  return total < 60
    ? `${total} s`
    : `${Math.floor(total / 60)} min ${total % 60} s`;
}
