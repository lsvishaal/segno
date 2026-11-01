import { useEffect, useState } from "react";
import { progress, type ProgressView } from "../bridge";
import { LineGraph } from "../components/LineGraph";
import { practisedWhen } from "./Library";

/** Where a Practise button on this page loops; null plays from bar 1, no loop. */
export type PractiseRange = { readonly from: number; readonly to: number } | null;

/** A bar's level 0–4 in words; the map shows the word, never the number. */
export const LEVEL_WORDS = ["Secure", "Small slips", "Uneven", "Shaky", "Needs work"];

/** Each level's fill: pale green when secure, then amber deeper as it gets harder; never red. */
const FILL = [
  "color-mix(in srgb, var(--color-correct) 14%, transparent)",
  "color-mix(in srgb, var(--color-pause) 22%, transparent)",
  "color-mix(in srgb, var(--color-pause) 42%, transparent)",
  "color-mix(in srgb, var(--color-pause) 66%, transparent)",
  "color-mix(in srgb, var(--color-pause) 92%, transparent)",
];

/** One session in which a bar was played: its date, and what happened each time through. */
export interface BarDay {
  readonly startedMs: number;
  readonly hesitationS: number;
  readonly wrong: number;
}

/** Bar `measure` in every session that played it, oldest first, per time through. */
export function barHistory(view: ProgressView, measure: number): BarDay[] {
  return view.sessions
    .flatMap((session) => {
      const cell = session.bars.find((bar) => bar.measure === measure);
      if (!cell) return [];
      const passes = Math.max(1, cell.passes);
      return [{ startedMs: session.startedMs, hesitationS: cell.pauseUs / passes / 1_000_000, wrong: cell.wrong / passes }];
    })
    .reverse();
}

/** "12 s", "4 min", "1 h 12 min". */
export function playedFor(us: number): string {
  const seconds = Math.round(us / 1_000_000);
  if (seconds < 60) return `${seconds} s`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} min`;
  return `${Math.floor(minutes / 60)} h ${minutes % 60} min`;
}

/** "Bar 6", "Bars 6–7". */
function barsNamed(first: number, last: number): string {
  return first === last ? `Bar ${first}` : `Bars ${first}–${last}`;
}

/** "⟲ Practise bar 1", "⟲ Practise bars 5–7". */
function practiseLabel(from: number, to: number): string {
  return from === to ? `⟲ Practise bar ${from}` : `⟲ Practise bars ${from}–${to}`;
}

/** "31 Oct". */
function day(ms: number): string {
  return new Date(ms).toLocaleDateString("en-GB", { day: "numeric", month: "short" });
}

/** "16:52". */
function time(ms: number): string {
  return new Date(ms).toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" });
}

/**
 * A piece's Progress page: what to practise next, the piece bar by bar with
 * a word for how each is going, and one bar over time. Every sentence comes
 * from Rust; this lays it out and draws the graphs.
 */
export function ProgressScreen({
  id,
  backLabel,
  onBack,
  onPractise,
  now = Date.now(),
}: {
  readonly id: string;
  readonly backLabel: string;
  readonly onBack: () => void;
  readonly onPractise: (range: PractiseRange) => void;
  /** The time "2 days ago" counts from. */
  readonly now?: number;
}) {
  const [view, setView] = useState<ProgressView | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    progress(id).then(
      (loaded) => live && setView(loaded),
      (error: unknown) => live && setFailure(String(error)),
    );
    return () => {
      live = false;
    };
  }, [id]);

  return (
    <section
      aria-label="Progress"
      className="mx-auto w-full max-w-[1120px] px-8 py-8"
      data-progress
    >
      <button
        type="button"
        onClick={onBack}
        className="cursor-pointer rounded-md px-2 py-1 text-sm text-ink-muted transition-colors hover:bg-ink/5 hover:text-accent focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
      >
        ← {backLabel}
      </button>
      {failure && <p className="mt-6 text-sm text-ink-muted">{failure}</p>}
      {view && <Page view={view} onPractise={onPractise} now={now} />}
    </section>
  );
}

function Page({
  view,
  onPractise,
  now,
}: {
  readonly view: ProgressView;
  readonly onPractise: (range: PractiseRange) => void;
  readonly now: number;
}) {
  const sessions = view.sessions.length;
  return (
    <>
      <header className="mt-6">
        {view.composer && <p className="text-label text-ink-muted uppercase">{view.composer}</p>}
        <h1 className="mt-1 font-serif text-heading italic">{view.title}</h1>
        {sessions > 0 && (
          <p className="mt-2 text-sm text-ink-muted tabular-nums" data-progress-facts>
            Practised {sessions === 1 ? "once" : `${sessions} times`} · {playedFor(view.playedUs)} in total
            {view.lastPractisedMs !== null && ` · last practised ${practisedWhen(view.lastPractisedMs, now)}`}
          </p>
        )}
        <p className="mt-1 text-sm">Which bars need work, and whether they're getting better.</p>
      </header>

      {sessions === 0 ? (
        <div className="mt-10" data-progress-empty>
          <p className="text-subtitle">Play this piece once and your progress shows here.</p>
          <PractiseButton label="Practise" onClick={() => onPractise(null)} />
        </div>
      ) : (
        <>
          <PractiseNext view={view} onPractise={onPractise} />
          <PieceMap view={view} onPractise={onPractise} />
        </>
      )}
    </>
  );
}

function Label({ children }: { readonly children: string }) {
  return <h2 className="text-label text-ink-muted uppercase">{children}</h2>;
}

function PractiseButton({ label, onClick }: { readonly label: string; readonly onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="mt-3 cursor-pointer rounded-full bg-ink px-4 py-1.5 text-xs font-medium text-paper transition-opacity hover:opacity-90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
    >
      {label}
    </button>
  );
}

function LoopButton({ from, to, onPractise }: { readonly from: number; readonly to: number; readonly onPractise: (range: PractiseRange) => void }) {
  return (
    <button
      type="button"
      onClick={() => onPractise({ from, to })}
      className="shrink-0 cursor-pointer rounded-full border border-rule px-3 py-1 text-xs font-medium transition-colors hover:border-ink/30 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
    >
      {practiseLabel(from, to)}
    </button>
  );
}

function PractiseNext({
  view,
  onPractise,
}: {
  readonly view: ProgressView;
  readonly onPractise: (range: PractiseRange) => void;
}) {
  return (
    <section className="mt-8" aria-label="Practise next">
      <Label>Practise next</Label>
      {view.workOnNext.length === 0 ? (
        <div className="mt-3">
          <p className="text-sm">Every bar is secure in your recent sessions.</p>
          <PractiseButton label="Play it through" onClick={() => onPractise(null)} />
        </div>
      ) : (
        <ul className="mt-3 divide-y divide-rule rounded-xl border border-rule bg-paper-raised">
          {view.workOnNext.map((passage) => (
            <li key={passage.first} data-work-on-next className="flex items-center justify-between gap-6 px-5 py-3">
              <p className="min-w-0 text-sm">
                <span className="font-medium">{barsNamed(passage.first, passage.last)}</span>
                <span className="text-ink-muted"> — {passage.facts}</span>
              </p>
              <LoopButton from={passage.from} to={passage.to} onPractise={onPractise} />
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

/**
 * The piece, eight bars to a row like a page of music: each bar a box with
 * its number and a word for how it's going now. Clicking one shows it over
 * time underneath; the hardest is shown first.
 */
function PieceMap({
  view,
  onPractise,
}: {
  readonly view: ProgressView;
  readonly onPractise: (range: PractiseRange) => void;
}) {
  const [selected, setSelected] = useState<number | null>(view.workOnNext[0]?.first ?? null);
  const nowOf = new Map(view.now.map((bar) => [bar.measure, bar.level]));
  const bars = Array.from({ length: view.bars }, (_, index) => index + 1);

  return (
    <section className="mt-10" aria-label="The piece, bar by bar">
      <Label>The piece, bar by bar</Label>
      <div className="mt-3 grid grid-cols-8 gap-2 rounded-xl border border-rule bg-paper-raised p-4" data-piece-map>
        {bars.map((measure) => {
          const level = nowOf.get(measure);
          const word = level === undefined ? "Not played" : level === null ? "Not enough yet" : LEVEL_WORDS[level];
          const isSelected = selected === measure;
          return (
            <button
              key={measure}
              type="button"
              data-bar={measure}
              data-level={level === undefined ? "" : String(level)}
              aria-pressed={isSelected}
              aria-label={`Bar ${measure}: ${word}`}
              onClick={() => setSelected(measure)}
              style={level === undefined || level === null ? undefined : { background: FILL[level] }}
              className={`flex h-16 cursor-pointer flex-col items-start justify-between rounded-lg px-2.5 py-2 text-left transition-shadow hover:shadow-sm focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent ${
                level === null ? "border-2 border-dotted border-ink-muted/60" : "border border-rule"
              } ${isSelected ? "outline-2 outline-offset-2 outline-ink" : ""}`}
            >
              <span className={`text-lg leading-none font-medium tabular-nums ${level === undefined ? "text-ink-muted" : ""}`}>
                {measure}
              </span>
              {level !== undefined && <span className="text-[11px] leading-tight">{word}</span>}
            </button>
          );
        })}
      </div>
      <Legend />
      {selected === null ? (
        <p className="mt-6 text-sm text-ink-muted">Click a bar to see how it has gone over time.</p>
      ) : (
        <BarOverTime view={view} measure={selected} onPractise={onPractise} />
      )}
    </section>
  );
}

function Legend() {
  const swatch = "inline-block h-3 w-4 rounded-sm align-[-2px]";
  return (
    <ul className="mt-3 flex flex-wrap gap-x-5 gap-y-1 text-xs text-ink-muted" data-legend>
      {LEVEL_WORDS.map((word, level) => (
        <li key={word}>
          <span className={`${swatch} border border-rule`} style={{ background: FILL[level] }} /> {word}
        </li>
      ))}
      <li>
        <span className={`${swatch} border-2 border-dotted border-ink-muted/60`} /> Not enough yet: played once
      </li>
      <li>
        <span className={`${swatch} border border-rule`} /> Blank: not played yet
      </li>
    </ul>
  );
}

function BarOverTime({
  view,
  measure,
  onPractise,
}: {
  readonly view: ProgressView;
  readonly measure: number;
  readonly onPractise: (range: PractiseRange) => void;
}) {
  const detail = view.details.find((each) => each.measure === measure);
  const history = barHistory(view, measure);
  const days = history.map((each) => day(each.startedMs));
  // Two sessions on one day: the time tells them apart.
  const label = new Set(days).size < days.length ? (ms: number) => `${day(ms)} ${time(ms)}` : day;
  return (
    <div data-bar-detail className="mt-6 rounded-xl border border-rule bg-paper-raised px-5 py-4">
      <div className="flex items-start justify-between gap-6">
        <div className="min-w-0">
          <h3 className="font-medium">Bar {measure} over time</h3>
          <p className="mt-1 text-sm text-ink-muted">{detail?.line ?? "Not played yet."}</p>
        </div>
        {detail && <LoopButton from={detail.from} to={detail.to} onPractise={onPractise} />}
      </div>
      {history.length < 2 ? (
        <p className="mt-4 text-sm text-ink-muted">Play this bar on another day to see a line.</p>
      ) : (
        <div className="mt-4 grid grid-cols-2 gap-8">
          <LineGraph
            title="Hesitation (seconds, each time through)"
            points={history.map((each) => ({ label: label(each.startedMs), value: each.hesitationS }))}
          />
          <LineGraph
            title="Wrong notes (each time through)"
            points={history.map((each) => ({ label: label(each.startedMs), value: each.wrong }))}
          />
        </div>
      )}
    </div>
  );
}
