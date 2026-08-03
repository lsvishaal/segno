/**
 * A decorative accent for one piece on the shelf.
 *
 * Deterministic from the piece's id, and nothing else — never from bar
 * count, note count, or anything that could read as a difficulty signal
 * (`001 §11` rules grading out). Purely visual variety, the same piece
 * always gets the same colour, and no two adjacent pieces are guaranteed
 * anything about each other.
 */
const MOTIFS = [
  "var(--color-accent)",
  "var(--color-motif-sage)",
  "var(--color-motif-plum)",
] as const;

export function motifFor(pieceId: string): string {
  let hash = 0;

  for (const char of pieceId) {
    hash = (hash * 31 + char.codePointAt(0)!) | 0;
  }

  const index = Math.abs(hash) % MOTIFS.length;
  return MOTIFS[index]!;
}
