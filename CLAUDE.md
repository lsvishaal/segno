# Hogwash — project rules for Claude Code

## What this is
Hogwash is a free, open-source, local-first desktop app for practising piano from
standard sheet music with a MIDI keyboard. The cursor waits for the correct notes.

Goal: "The practice tool that remembers which bars you struggle with and helps
you fix them." v0.1 preview on 8 November, v1.0 on 13 December (see ROADMAP.md).

## Stack
- Rust workspace in `crates/` (domain, notation, capture, practice, analysis,
  feedback, engraving, engraving-verovio, library, acquisition, midi, records, cli).
- Tauri v2 app in `app/` (React 19, TypeScript, Vite, Tailwind v4). Rust commands
  in `app/src-tauri/src/commands.rs`.
- Verovio is loaded at run time from a sibling checkout (`../verovio-build`,
  `../verovio/data`); see `justfile`.
- `just run` launches the app. `just ci` is the full gate.

## Hard rules
1. Work ONLY on the phase I give you. When the phase is done, stop and report.
   Do not start the next phase.
2. Before writing code, read the relevant code and give me a short plan
   (files to touch, approach, risks). Wait for my approval.
3. `just ci` must pass at the end of every task. Commit after each completed task
   with a clear message.
4. New domain logic gets unit tests. Keep engine crates pure (no I/O, no clock
   reads) — the architecture check in `scripts/check-arch.sh` must keep passing.
   Timestamps enter only as data from the MIDI adapter.
5. Do NOT add: new crates (unless you justify it and I approve), ADRs, spec
   documents, or features not in the current phase. If you notice something
   worth doing later, add one line under "Later" in `ROADMAP.md`.
6. The `docs/` folder has been deleted. Code comments still reference it
   (e.g. `003 §7.4`, `ADR-0006`, `FR-501`). Do not recreate docs or chase those
   references. When you edit a file for other reasons, remove dead doc references
   in the lines you touch, and leave a plain-English comment only if it adds value.
7. Visual work follows §16–§17 of ROADMAP.md. One screen per task. Show a plan
   with the layout described in words before changing styles.
8. You cannot see the running app or play a MIDI keyboard. At the end of each
   task that changes behaviour, give me a short manual test checklist
   ("open X, play Y, expect Z").
9. If a decision changes user-visible behaviour and isn't specified, ask me.
   Don't guess.

## Task IDs
Task IDs come from ROADMAP.md (e.g. M1.5). Reference them in commit messages.

## Product principles (these still hold)
- Local-first: no accounts, no network calls, no telemetry.
- No gamification: no scores, combos, streaks, or difficulty badges.
- Helping the user decide what to practise IS allowed and wanted
  (the old "never advisory" rule is removed).
- Feedback should be factual and specific ("bar 9: 14s stalled, 3 wrong notes"),
  never judgmental.
