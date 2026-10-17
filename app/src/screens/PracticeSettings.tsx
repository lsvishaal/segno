import { useEffect, useRef, useState } from "react";
import type { RefObject } from "react";

import {
  instruments as fetchInstruments,
  outputPorts as fetchOutputPorts,
  setThru,
} from "../bridge";
import type { InstrumentSummary, OutputPortSummary } from "../bridge";
import { MAX_SCALE, MIN_SCALE, SCALE_STEP } from "./zoom";

/**
 * Everything about how the score is shown and heard, gathered behind one
 * gear trigger instead of five separate floating controls (`BFR-8`).
 * A pure props-in/callbacks-out panel — `scale` and
 * `showNoteNames` stay owned by `Practice.tsx`, since it also drives the
 * rescale effect and the Ctrl+scroll handler from `scale`.
 */
export function SettingsPanel({
  open,
  onClose,
  triggerRef,
  showNoteNames,
  onShowNoteNamesChange,
  showGeometry,
  onShowGeometryChange,
  scale,
  onScaleChange,
  zoomError,
  onExport,
  exportError,
}: {
  readonly open: boolean;
  readonly onClose: () => void;
  readonly triggerRef: RefObject<HTMLButtonElement | null>;
  readonly showNoteNames: boolean;
  readonly onShowNoteNamesChange: (value: boolean) => void;
  readonly showGeometry: boolean;
  readonly onShowGeometryChange: (value: boolean) => void;
  readonly scale: number;
  readonly onScaleChange: (scale: number) => void;
  readonly zoomError: string | null;
  readonly onExport: () => void;
  readonly exportError: string | null;
}) {
  // Closes on an outside click or Escape. No focus trap: a handful of
  // simple controls in a non-modal popover (`aria-modal="false"`) don't
  // warrant one — Tab is free to leave it, only focus *return* on close
  // matters.
  useEffect(() => {
    if (!open) return;

    const onPointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (triggerRef.current?.contains(target)) return;
      if ((event.target as HTMLElement).closest('[data-settings-panel="true"]')) return;
      onClose();
    };

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
        triggerRef.current?.focus();
      }
    };

    document.addEventListener("mousedown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open, onClose, triggerRef]);

  if (!open) return null;

  return (
    <div
      data-settings-panel="true"
      role="dialog"
      aria-label="Settings"
      aria-modal="false"
      className="panel absolute top-full right-0 z-10 mt-2 w-80 rounded-2xl p-5 text-sm"
    >
      <section>
        <h3 className="text-label font-medium text-ink-muted uppercase">
          Appearance
        </h3>

        <div className="mt-4 flex items-center justify-between">
          <span className="text-ink-muted">Note names</span>
          <button
            type="button"
            aria-pressed={showNoteNames}
            onClick={() => onShowNoteNamesChange(!showNoteNames)}
            className={`cursor-pointer rounded-full border border-rule-strong px-3 py-1 text-xs font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent ${
              showNoteNames ? "bg-ink text-paper" : "text-ink-muted hover:text-ink"
            }`}
          >
            {showNoteNames ? "On" : "Off"}
          </button>
        </div>

        <div className="mt-3 flex items-center justify-between">
          <span className="text-ink-muted">Show geometry</span>
          <button
            type="button"
            aria-pressed={showGeometry}
            aria-label="Show geometry"
            onClick={() => onShowGeometryChange(!showGeometry)}
            className={`cursor-pointer rounded-full border border-rule-strong px-3 py-1 text-xs font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent ${
              showGeometry ? "bg-ink text-paper" : "text-ink-muted hover:text-ink"
            }`}
          >
            {showGeometry ? "On" : "Off"}
          </button>
        </div>
      </section>

      <div className="staff-rule mt-5" />

      <section className="pt-5">
        <h3 className="text-label font-medium text-ink-muted uppercase">Sound</h3>
        <div className="mt-4">
          <SoundRouting />
        </div>
      </section>

      <div className="staff-rule mt-5" />

      <section className="pt-5">
        <h3 className="text-label font-medium text-ink-muted uppercase">
          View &amp; export
        </h3>

        <div className="mt-4 flex items-center gap-2">
          <label htmlFor="zoom" className="text-ink-muted">
            Zoom
          </label>
          <input
            id="zoom"
            type="range"
            min={MIN_SCALE}
            max={MAX_SCALE}
            step={SCALE_STEP}
            value={scale}
            onChange={(event) => onScaleChange(Number(event.target.value))}
            className="w-full cursor-pointer accent-accent"
          />
          <span className="w-10 shrink-0 text-right text-xs tabular-nums text-ink-muted">
            {scale}%
          </span>
        </div>
        {zoomError && <p className="mt-1 text-xs text-wrong">{zoomError}</p>}

        <button
          type="button"
          onClick={onExport}
          className="mt-4 w-full cursor-pointer rounded-md border border-rule-strong px-3 py-2 text-xs font-medium transition-colors hover:border-accent hover:text-accent focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
        >
          Export MusicXML
        </button>
        {exportError && <p className="mt-1 text-xs text-wrong">{exportError}</p>}
      </section>
    </div>
  );
}

/** A small down chevron for a dropdown trigger. */
function ChevronIcon() {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className="shrink-0"
    >
      <path d="m6 9 6 6 6-6" />
    </svg>
  );
}

/** A checkmark beside a dropdown's selected option. */
function CheckIcon() {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className="shrink-0 text-accent"
    >
      <path d="M20 6 9 17l-5-5" />
    </svg>
  );
}

/**
 * A trigger showing the current choice, opening a real list of options —
 * not a native `<select>`. Closes on selecting an option or clicking
 * outside it.
 */
function Dropdown({
  label,
  value,
  placeholder,
  options,
  onChange,
}: {
  readonly label: string;
  readonly value: string | null;
  readonly placeholder: string;
  readonly options: readonly string[];
  readonly onChange: (value: string | null) => void;
}) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;

    const onPointerDown = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };

    document.addEventListener("mousedown", onPointerDown);
    return () => document.removeEventListener("mousedown", onPointerDown);
  }, [open]);

  const choose = (next: string | null) => {
    onChange(next);
    setOpen(false);
  };

  return (
    <div ref={rootRef} className="relative min-w-0 flex-1">
      <button
        type="button"
        onClick={() => setOpen((current) => !current)}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={label}
        className="flex w-full cursor-pointer items-center justify-between gap-2 rounded-md border border-rule-strong bg-paper-raised px-3 py-1.5 text-left text-ink focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
      >
        <span className="truncate">{value ?? placeholder}</span>
        <ChevronIcon />
      </button>

      {open && (
        <ul
          role="listbox"
          aria-label={label}
          className="panel absolute z-20 mt-1 max-h-48 w-full min-w-40 overflow-auto rounded-lg py-1"
        >
          <li>
            <button
              type="button"
              role="option"
              aria-selected={value === null}
              onClick={() => choose(null)}
              className={optionClassName(value === null)}
            >
              {placeholder}
            </button>
          </li>

          {options.map((name) => (
            <li key={name}>
              <button
                type="button"
                role="option"
                aria-selected={value === name}
                onClick={() => choose(name)}
                className={optionClassName(value === name)}
              >
                <span className="truncate">{name}</span>
                {value === name && <CheckIcon />}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function optionClassName(selected: boolean): string {
  return `flex w-full cursor-pointer items-center justify-between gap-2 px-3 py-1.5 text-left transition-colors ${
    selected ? "text-accent" : "text-ink hover:bg-ink/5"
  }`;
}

/**
 * Routes MIDI Thru: an input device's raw messages, forwarded straight to an
 * output device — real hardware or a software synth, in place of Hogwash
 * making its own sound. Deliberately independent of `InstrumentPicker`:
 * this owns its own input/output selection and works whether or not a
 * practice attempt is running, because Thru has nothing to do with
 * correctness (`hogwash-midi`'s `Thru`).
 */
function SoundRouting() {
  const [inputs, setInputs] = useState<readonly InstrumentSummary[]>([]);
  const [outputs, setOutputs] = useState<readonly OutputPortSummary[]>([]);
  const [input, setInput] = useState<string | null>(null);
  const [output, setOutput] = useState<string | null>(null);

  useEffect(() => {
    let live = true;

    void (async () => {
      try {
        const found = await fetchInstruments();
        if (live) setInputs(found);
      } catch {
        if (live) setInputs([]);
      }
    })();

    return () => {
      live = false;
    };
  }, []);

  useEffect(() => {
    let live = true;

    void (async () => {
      try {
        const found = await fetchOutputPorts();
        if (live) setOutputs(found);
      } catch {
        if (live) setOutputs([]);
      }
    })();

    return () => {
      live = false;
    };
  }, []);

  useEffect(() => {
    void setThru(input, output);
  }, [input, output]);

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-3">
        <span className="w-14 shrink-0 text-ink-muted">Input</span>
        <Dropdown
          label="MIDI input"
          value={input}
          placeholder="None"
          options={inputs.map((instrument) => instrument.name)}
          onChange={setInput}
        />
      </div>

      <div className="flex items-center gap-3">
        <span className="w-14 shrink-0 text-ink-muted">Output</span>
        <Dropdown
          label="MIDI output"
          value={output}
          placeholder="None"
          options={outputs.map((port) => port.name)}
          onChange={setOutput}
        />
      </div>
    </div>
  );
}
