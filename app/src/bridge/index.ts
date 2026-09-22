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
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";

import type {
  AttemptSummary,
  DrawnNote,
  ImportedPiece,
  InstrumentSummary,
  Mark,
  OpenedPiece,
  OutputPortSummary,
  PieceSummary,
  PracticeEvent,
  StaffPosition,
  Step,
  Unavailable,
} from "./types";

export type {
  AttemptSummary,
  DrawnNote,
  ImportedPiece,
  InstrumentSummary,
  Mark,
  OpenedPiece,
  OutputPortSummary,
  PieceSummary,
  PracticeEvent,
  StaffPosition,
  Step,
  Unavailable,
};

/** The Pieces available to practise. Never empty (FR-207). */
export function library(): Promise<PieceSummary[]> {
  return invoke<PieceSummary[]>("library");
}

/** Engraves a Piece and returns everything needed to draw it (FR-800). */
export function openPiece(id: string): Promise<OpenedPiece> {
  return invoke<OpenedPiece>("open_piece", { id });
}

/** Re-engraves a Piece at a different staff size (`BFR-4`). */
export function rescalePiece(id: string, scale: number): Promise<OpenedPiece> {
  return invoke<OpenedPiece>("rescale_piece", { id, scale });
}

/** Why the application cannot work, or `null` if it can. */
export function startupFailure(): Promise<Unavailable | null> {
  return invoke<Unavailable | null>("startup_failure");
}

/** The MIDI instruments currently connected (FR-307). */
export function instruments(): Promise<InstrumentSummary[]> {
  return invoke<InstrumentSummary[]>("instruments");
}

/** Begins an attempt at `pieceId`, on `instrument` (FR-801). */
export function startAttempt(pieceId: string, instrument: string): Promise<void> {
  return invoke<void>("start_attempt", { pieceId, instrument });
}

/** Ends whatever attempt is running, and returns its summary (FR-408). */
export function endAttempt(): Promise<AttemptSummary> {
  return invoke<AttemptSummary>("end_attempt");
}

/**
 * Saves `pieceId`'s original source to a location the performer chooses.
 * Resolves whether they save it or cancel the dialog — cancelling is not an
 * error.
 */
export function exportMusicXml(pieceId: string): Promise<void> {
  return invoke<void>("export_musicxml", { id: pieceId });
}

/**
 * Opens a file picker and imports the `MusicXML` chosen, joining it to the
 * library. Resolves `null` if the performer cancels the dialog — cancelling
 * is not an error.
 */
export function importMusicXml(): Promise<ImportedPiece | null> {
  return invoke<ImportedPiece | null>("import_musicxml");
}

/** The MIDI output devices currently available, for routing Thru. */
export function outputPorts(): Promise<OutputPortSummary[]> {
  return invoke<OutputPortSummary[]>("output_ports");
}

/**
 * Sets or clears the MIDI Thru route: everything `input` reports gets
 * forwarded straight to `output`, instead of Hogwash making its own sound.
 * `null` for either side clears whatever route was set. Independent of
 * `startAttempt` — routing works with or without a practice attempt running.
 */
export function setThru(input: string | null, output: string | null): Promise<void> {
  return invoke<void>("set_thru", { input, output });
}

/**
 * Listens for the practice loop's events, until the returned function is
 * called.
 *
 * The one place `@tauri-apps/api/event` is imported — same reason every
 * other command has exactly one wrapper here.
 */
export function listenToPractice(
  onEvent: (event: PracticeEvent) => void,
): Promise<UnlistenFn> {
  return listen<PracticeEvent>("practice", (message) => onEvent(message.payload));
}
