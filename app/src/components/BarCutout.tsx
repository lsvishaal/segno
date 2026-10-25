import { useLayoutEffect, useRef, useState } from "react";

/** Draws a cut-out of some bars, or `null` when the score can't. */
export type DrawCutOut = () => SVGSVGElement | null;

/**
 * Some bars of the score as a picture (the score draws it: see `Score`'s
 * `cutOut`), as a button that shows them on the score. Without a picture, it
 * shows the bar numbers instead.
 */
export function BarCutout({
  first,
  last,
  draw,
  onPick,
}: {
  readonly first: number;
  readonly last: number;
  readonly draw: DrawCutOut;
  /** The cut-out was clicked: show bar `first` on the score. */
  readonly onPick: (measure: number) => void;
}) {
  const holder = useRef<HTMLDivElement>(null);
  const [drawn, setDrawn] = useState(false);

  useLayoutEffect(() => {
    const target = holder.current;
    if (!target) return;
    const cut = draw();
    if (cut) target.replaceChildren(cut);
    else target.replaceChildren();
    setDrawn(cut !== null);
  }, [draw]);

  const label = first === last ? `Bar ${first}` : `Bars ${first}–${last}`;

  return (
    <button
      type="button"
      onClick={() => onPick(first)}
      title={`${label} · show on the score`}
      aria-label={`${label} · show on the score`}
      data-bar-cutout
      className="relative block h-full w-full cursor-pointer overflow-hidden rounded-xl border border-rule bg-paper-raised text-ink transition-colors duration-[120ms] hover:border-accent focus-visible:outline-2 focus-visible:outline-accent"
    >
      <div ref={holder} className="h-full w-full px-2 py-1" />
      {!drawn && (
        <span className="absolute inset-0 flex items-center justify-center font-serif text-title text-ink-muted">
          {label}
        </span>
      )}
      <span className="absolute bottom-1 left-2 font-mono text-[10px] text-ink-muted">{label}</span>
    </button>
  );
}
