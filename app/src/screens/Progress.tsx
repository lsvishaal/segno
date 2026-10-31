import { useEffect, useState } from "react";
import { progress, type ProgressView } from "../bridge";
import { practisedWhen } from "./Library";

/** Where a Practise button on this page loops; null plays from bar 1, no loop. */
export type PractiseRange = { readonly from: number; readonly to: number } | null;

/** How many sessions the grid shows before "Show all". */
const RECENT_ROWS = 12;

/** Amber by level 1–4, the same hue deeper as it gets harder; never red. */
const AMBER = ["", "0.22", "0.42", "0.66", "0.92"];

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

/** "31 Oct 16:52". */
function when(ms: number): string {
  const date = new Date(ms);
  return `${date.toLocaleDateString("en-GB", { day: "numeric", month: "short" })} ${date.toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" })}`;
}

/**
 * A piece's Progress page: what to work on next, every session bar by bar,
 * and each bar now against its earliest sessions. Every sentence comes from
 * Rust; this only lays it out.
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
          <p className="mt-2 font-mono text-xs text-ink-muted tabular-nums" data-progress-facts>
            {sessions} {sessions === 1 ? "session" : "sessions"} · {playedFor(view.playedUs)}
            {view.lastPractisedMs !== null && ` · last practised ${practisedWhen(view.lastPractisedMs, now)}`}
          </p>
        )}
      </header>

      {sessions === 0 ? (
        <div className="mt-10" data-progress-empty>
          <p className="text-subtitle">Play this piece once and your progress shows here.</p>
          <PractiseButton label="Practise" onClick={() => onPractise(null)} />
        </div>
      ) : (
        <>
          <WorkOnNext view={view} onPractise={onPractise} />
          <Grid view={view} onPractise={onPractise} />
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

function WorkOnNext({
  view,
  onPractise,
}: {
  readonly view: ProgressView;
  readonly onPractise: (range: PractiseRange) => void;
}) {
  return (
    <section className="mt-8" aria-label="Work on next">
      <Label>Work on next</Label>
      {view.workOnNext.length === 0 ? (
        <div className="mt-3">
          <p className="text-sm">Every bar is clean in your recent sessions.</p>
          <PractiseButton label="Play it through" onClick={() => onPractise(null)} />
        </div>
      ) : (
        <ul className="mt-3 divide-y divide-rule rounded-xl border border-rule bg-paper-raised">
          {view.workOnNext.map((passage) => (
            <li
              key={passage.first}
              data-work-on-next
              className="flex items-center justify-between gap-6 px-5 py-3"
            >
              <p className="min-w-0 text-sm">
                <span className="font-medium">{barsNamed(passage.first, passage.last)}</span>
                {passage.facts && <span className="text-ink-muted"> · {passage.facts}</span>}
              </p>
              <button
                type="button"
                onClick={() => onPractise({ from: passage.from, to: passage.to })}
                className="shrink-0 cursor-pointer rounded-full border border-rule px-3 py-1 text-xs font-medium transition-colors hover:border-ink/30 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
              >
                {practiseLabel(passage.from, passage.to)}
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

/**
 * Bars across, sessions down (newest on top), and a "Now" row of heights
 * above them. Each column is a button that opens the bar's line.
 */
function Grid({
  view,
  onPractise,
}: {
  readonly view: ProgressView;
  readonly onPractise: (range: PractiseRange) => void;
}) {
  const [all, setAll] = useState(false);
  const [selected, setSelected] = useState<number | null>(null);
  const rows = all ? view.sessions : view.sessions.slice(0, RECENT_ROWS);
  const nowOf = new Map(view.now.map((bar) => [bar.measure, bar.level]));
  const detailOf = new Map(view.details.map((detail) => [detail.measure, detail]));
  const bars = Array.from({ length: view.bars }, (_, index) => index + 1);
  const detail = selected === null ? undefined : detailOf.get(selected);

  return (
    <section className="mt-10" aria-label="Every session">
      <Label>Every session</Label>
      <div className="mt-3 overflow-x-auto rounded-xl border border-rule bg-paper-raised px-5 py-4">
        <div className="flex w-max gap-[3px]">
          <div className="mr-3 flex flex-col gap-[3px] font-mono text-[11px] text-ink-muted">
            <span className="flex h-10 items-end">Now</span>
            {rows.map((session) => (
              <span key={session.startedMs} className="flex h-4 items-center whitespace-nowrap">
                {when(session.startedMs)}
              </span>
            ))}
            <span className="h-4" />
          </div>

          {bars.map((measure) => {
            const level = nowOf.get(measure);
            const isSelected = selected === measure;
            return (
              <button
                key={measure}
                type="button"
                data-bar-column={measure}
                aria-pressed={isSelected}
                aria-label={`Bar ${measure}: ${detailOf.get(measure)?.line ?? "Not played yet."}`}
                onClick={() => setSelected(isSelected ? null : measure)}
                className={`flex w-6 cursor-pointer flex-col gap-[3px] rounded-sm outline-offset-2 hover:bg-ink/5 focus-visible:outline-2 focus-visible:outline-accent ${
                  isSelected ? "outline-2 outline-ink" : ""
                }`}
              >
                <span className="flex h-10 w-full items-end" data-now={level === undefined ? "" : String(level)}>
                  {level === null ? (
                    // Played, but once: not enough to say where it stands.
                    <span className="h-2 w-full border-t-2 border-dotted border-ink-muted" />
                  ) : level === undefined ? null : level === 0 ? (
                    <span className="h-[3px] w-full rounded-sm bg-ink-muted/40" />
                  ) : (
                    <span
                      className="w-full rounded-t-sm bg-pause"
                      style={{ height: `${level * 25}%`, opacity: AMBER[level] }}
                    />
                  )}
                </span>
                {rows.map((session) => {
                  const cell = session.bars.find((bar) => bar.measure === measure);
                  return (
                    <span
                      key={session.startedMs}
                      data-cell={cell ? String(cell.level) : ""}
                      className={`h-4 w-full rounded-sm ${
                        !cell ? "" : cell.level === 0 ? "border border-rule-strong" : "bg-pause"
                      }`}
                      style={cell && cell.level > 0 ? { opacity: AMBER[cell.level] } : undefined}
                    />
                  );
                })}
                <span
                  className={`h-4 text-center font-mono text-[11px] leading-4 tabular-nums ${
                    isSelected ? "font-bold text-ink" : "text-ink-muted"
                  }`}
                >
                  {isSelected || measure === 1 || measure % 4 === 0 ? measure : ""}
                </span>
              </button>
            );
          })}
        </div>
      </div>
      <p className="mt-2 text-xs text-ink-muted">
        Taller and deeper is harder now · dotted: played once, play it again to judge · click a bar
        for its story
      </p>
      {view.sessions.length > RECENT_ROWS && (
        <button
          type="button"
          onClick={() => setAll((shown) => !shown)}
          className="mt-2 cursor-pointer text-xs text-ink-muted underline decoration-rule underline-offset-4 hover:text-ink"
        >
          {all ? "Show the last 12" : `Show all ${view.sessions.length} sessions`}
        </button>
      )}

      {detail && (
        <div
          data-bar-detail
          className="mt-4 flex items-center justify-between gap-6 rounded-xl border border-rule bg-paper-raised px-5 py-3"
        >
          <p className="text-sm">
            <span className="font-medium">Bar {detail.measure}</span>
            <span className="text-ink-muted"> · {detail.line}</span>
          </p>
          <button
            type="button"
            onClick={() => onPractise({ from: detail.from, to: detail.to })}
            className="shrink-0 cursor-pointer rounded-full border border-rule px-3 py-1 text-xs font-medium transition-colors hover:border-ink/30 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
          >
            {practiseLabel(detail.from, detail.to)}
          </button>
        </div>
      )}
    </section>
  );
}
