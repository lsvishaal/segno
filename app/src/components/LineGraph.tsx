/** One dot: its date label and its value. */
export interface Point {
  readonly label: string;
  readonly value: number;
}

const W = 420;
const H = 170;
const LEFT = 36;
const RIGHT = 12;
const TOP = 12;
const BOTTOM = 26;

/** "3.1", "1", "0". */
function tick(value: number): string {
  return Number.isInteger(value) ? String(value) : value.toFixed(1);
}

/**
 * A small line graph: 0 at the bottom, the highest value at the top, one dot
 * per point left to right, the first and last labelled underneath (all of
 * them when there are six or fewer).
 */
export function LineGraph({ title, points }: { readonly title: string; readonly points: readonly Point[] }) {
  const max = Math.max(...points.map((point) => point.value), 0);
  // The top tick rounded up to a tenth, so it never reads "0.0".
  const top = max > 0 ? Math.ceil(max * 10) / 10 : 1;
  const x = (index: number) =>
    LEFT + (points.length === 1 ? (W - LEFT - RIGHT) / 2 : (index * (W - LEFT - RIGHT)) / (points.length - 1));
  const y = (value: number) => TOP + (1 - value / top) * (H - TOP - BOTTOM);
  const labelled = (index: number) => points.length <= 6 || index === 0 || index === points.length - 1;

  return (
    <figure data-line-graph className="min-w-0">
      <figcaption className="text-xs text-ink-muted">{title}</figcaption>
      <svg viewBox={`0 0 ${W} ${H}`} className="mt-1 w-full" role="img" aria-label={title}>
        <line x1={LEFT} x2={LEFT} y1={TOP} y2={H - BOTTOM} className="stroke-rule-strong" />
        <line x1={LEFT} x2={W - RIGHT} y1={H - BOTTOM} y2={H - BOTTOM} className="stroke-rule-strong" />
        {[0, top].map((value) => (
          <text
            key={value}
            x={LEFT - 6}
            y={y(value) + 4}
            textAnchor="end"
            className="fill-ink-muted font-mono text-[11px]"
          >
            {tick(value)}
          </text>
        ))}
        <polyline
          points={points.map((point, index) => `${x(index)},${y(point.value)}`).join(" ")}
          fill="none"
          className="stroke-ink"
          strokeWidth={1.5}
        />
        {points.map((point, index) => (
          <g key={index}>
            <circle data-point cx={x(index)} cy={y(point.value)} r={3.5} className="fill-ink" />
            {labelled(index) && (
              <text
                x={x(index)}
                y={H - 8}
                textAnchor={points.length > 1 && index === 0 ? "start" : index === points.length - 1 && points.length > 1 ? "end" : "middle"}
                className="fill-ink-muted text-[11px]"
              >
                {point.label}
              </text>
            )}
          </g>
        ))}
      </svg>
    </figure>
  );
}
