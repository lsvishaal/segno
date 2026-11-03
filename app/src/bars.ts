/**
 * Bar names on screen: the printed numbers, with the pickup (printed 0) in
 * words. Never "bar 0" (DECISIONS, 3 Nov, E2). Bar 0 is a real bar here, so
 * nothing may test a bar number for truthiness.
 */

export const PICKUP = 0;

/** A label: "Bar 7", "Pickup". */
export function barLabel(bar: number): string {
  return bar === PICKUP ? "Pickup" : `Bar ${bar}`;
}

/** In a sentence: "bar 7", "the pickup". */
export function barName(bar: number): string {
  return bar === PICKUP ? "the pickup" : `bar ${bar}`;
}

/** A label for a run of bars: "Bar 7", "Bars 6–7", "Pickup", "Pickup–bar 2". */
export function barsLabel(first: number, last: number): string {
  if (first === last) return barLabel(first);
  return first === PICKUP ? `Pickup–bar ${last}` : `Bars ${first}–${last}`;
}

/** The same in running text and on buttons: "bar 7", "bars 6–7", "the pickup", "pickup–bar 2". */
export function barsInline(first: number, last: number): string {
  if (first === last) return barName(first);
  return first === PICKUP ? `pickup–bar ${last}` : `bars ${first}–${last}`;
}
