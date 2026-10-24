import type { ReviewBar } from "../bridge";

/** Below this, a column's height means nothing: two seconds is the tallest at least. */
const LEAST_SCALE_MS = 2000;
/** Wrong-note dots drawn on a column before the rest are counted. */
const MOST_DOTS = 5;

/**
 * One column per bar, after an attempt: how long you paused there (the
 * darker part is the pause going into the bar), red dots for wrong notes,
 * a stub for bars not reached. Every word and number comes from Rust.
 */
export function BarStrip({
  bars,
  onPick,
}: {
  readonly bars: readonly ReviewBar[];
  /** A column was clicked: show that bar. */
  readonly onPick: (measure: number) => void;
}) {
  const scale = Math.max(LEAST_SCALE_MS, ...bars.map((bar) => bar.pauseMs));
  // Every bar numbered when there are few; otherwise every fourth.
  const labelled = (measure: number, index: number) =>
    bars.length <= 16 || index === 0 || measure % 4 === 0;

  return (
    <div data-bar-strip className="flex items-end gap-px" role="list" aria-label="Bars played">
      {bars.map((bar, index) => (
        <button
          key={bar.measure}
          type="button"
          role="listitem"
          title={bar.line}
          aria-label={bar.line}
          data-state={bar.state}
          onClick={() => onPick(bar.measure)}
          className="group flex min-w-1 flex-1 cursor-pointer flex-col items-center focus-visible:outline-2 focus-visible:outline-accent"
        >
          <span className="flex h-3 items-end gap-px" aria-hidden="true">
            {Array.from({ length: Math.min(bar.wrong, MOST_DOTS) }, (_, dot) => (
              <span key={dot} className="size-1 rounded-full bg-wrong" />
            ))}
            {bar.wrong > MOST_DOTS && (
              <span className="text-[9px] leading-none text-wrong">+{bar.wrong - MOST_DOTS}</span>
            )}
          </span>
          <span className="flex h-16 w-full flex-col justify-end" aria-hidden="true">
            {bar.state === "played" && (
              <span
                className="flex w-full flex-col overflow-hidden rounded-t-sm bg-ink/20 group-hover:bg-ink/30"
                style={{ height: `max(2px, ${(bar.pauseMs / scale) * 100}%)` }}
              >
                <span
                  className="w-full bg-ink/45"
                  style={{ height: `${bar.pauseMs > 0 ? (bar.intoMs / bar.pauseMs) * 100 : 0}%` }}
                />
              </span>
            )}
            {bar.state === "notReached" && <span className="h-px w-full bg-rule-strong" />}
          </span>
          <span className="mt-1 h-3 text-[10px] leading-none text-ink-muted tabular-nums" aria-hidden="true">
            {bar.breaks > 0 ? "‖" : labelled(bar.measure, index) ? bar.measure : ""}
          </span>
        </button>
      ))}
    </div>
  );
}
