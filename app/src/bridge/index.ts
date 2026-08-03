/**
 * Everything the interface may ask Rust for.
 *
 * One typed function per command, and nothing else calls `invoke`. Two
 * reasons, and the second is the one that matters:
 *
 * 1. The command names are strings, and a typo in one is a run-time failure.
 *    Confining them to this file means there is one place to check.
 * 2. It keeps the boundary visible. Every line here is a thing the interface
 *    depends on Rust for — which is nearly everything, by design, because
 *    `004 §8.8` says Presentation originates no finding of its own.
 */
import { invoke } from "@tauri-apps/api/core";

import type { Mark, OpenedPiece, PieceSummary, Unavailable } from "./types";

export type { Mark, OpenedPiece, PieceSummary, Unavailable };

/** The Pieces available to practise. Never empty (FR-207). */
export function library(): Promise<PieceSummary[]> {
  return invoke<PieceSummary[]>("library");
}

/** Engraves a Piece and returns everything needed to draw it (FR-800). */
export function openPiece(id: string): Promise<OpenedPiece> {
  return invoke<OpenedPiece>("open_piece", { id });
}

/** Why the application cannot work, or `null` if it can. */
export function startupFailure(): Promise<Unavailable | null> {
  return invoke<Unavailable | null>("startup_failure");
}
