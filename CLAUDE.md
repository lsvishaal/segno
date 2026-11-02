# Hogwash — rules for Claude Code

## What this is
A free, open-source, local-first desktop app for practising piano from standard
sheet music with a MIDI keyboard. The cursor waits for the correct notes. Goal:
"the practice tool that remembers which bars you struggle with and helps you fix
them." Dates and the active milestone: `ROADMAP.md`.

## Where things live
| What | Where |
|---|---|
| The plan: gated milestones (M-Base, M-Win, M-Test, M-Public), task IDs, Parking lot | `ROADMAP.md` |
| Why things are as they are; open decisions (D-numbers) | `DECISIONS.md` |
| User-facing changes | `CHANGELOG.md` (Unreleased on top) |
| Who it's for, product rules, Never list, competitors | `docs/PRODUCT.md` |
| Visual system, screens as built, words, shortcuts, budgets | `docs/DESIGN.md` |
| Test levels, the e2e kit, the manual checklist | `docs/TESTING.md` |
| Release checklist, launch material | `docs/RELEASE.md` |
| The summary pasted into Claude chat | `docs/BRIEFING.md` |
| Historical, not maintained | `docs/archive/` |
| Domain definitions (pause, break, passes, levels, coach thresholds) | doc comments in `crates/segno-analysis/src/bars.rs`, `coach.rs`, `crates/segno-practice` — the source of truth |
| The owner's practice log | `annoyances.md` |

Stack:
- **Rust workspace** in `crates/` (domain, notation, capture, practice, analysis,
  feedback, engraving, engraving-verovio, library, acquisition, midi, records, cli).
- **Tauri v2 app** in `app/` (React 19, TypeScript, Vite, Tailwind v4, motion).
  Rust commands live in `app/src-tauri/src/commands.rs`.
- **Verovio** is a separate shared library loaded at run time from a sibling
  checkout (`../verovio-build`, `../verovio/data`). Bundling it is the main
  release risk.
- **Only `Score.tsx`** (and helpers it alone imports) touches the score's SVG;
  everything else asks through `ScoreHandle`.

Commands:
- `just run`: the app.
- `just ci`: the gate. Its cargo-deny step writes to `~/.cargo`, so run it outside
  a sandbox.
- `just engraving-test` and `just provided-test`: real-engine tests.
- `just e2e`: the real app driven end to end.
- `just install-hooks`: once per clone.

## Hard rules
1. Work ONLY on the phase given. When it's done, stop and report; don't start the
   next one.
2. Before writing code, read the relevant code and give a short plan (files,
   approach, risks). Wait for approval.
3. `just ci` passes before every commit. Commit after each completed task, cite
   task IDs, and push after each completed part (`git push`; `origin` tracks
   GitHub over HTTPS).
4. New domain logic gets unit tests. Engine crates stay pure (no I/O, no clock
   reads); `scripts/check-arch.sh` must keep passing. Timestamps enter only as
   data from the MIDI adapter.
5. **Tests use real pieces only:**
   - the Minuet in G or the Prelude in C (`app/src-tauri/library/`)
   - the fixtures in `app/src/components/__fixtures__`
   - pieces the owner supplied (`test-scores/`, ignored; tests skip when absent)
   - Never invented notes.
   - **Each test at the cheapest tier that fails when the behaviour breaks**
     (Rust unit → real engine → ALSA → UI → e2e; tiers in `docs/TESTING.md`).
     e2e only for real MIDI, engraving and layout, files the running app
     writes, and timing.
6. **Never write to the real data folder** (`~/.local/share/com.segno.practice`)
   unless the owner explicitly asks. Use `target/e2e/data` or copies (e.g.
   `target/m21-data`).
7. No new crates without approval. No features outside the current phase; ideas
   become one dated line in the ROADMAP Parking lot.
8. Docs describe what **is** and what is **decided**. No specs, and no speculative
   designs.
9. Visual work follows `docs/DESIGN.md`: one screen per task; describe the layout
   in words before changing styles.
10. You can't see the app or play a keyboard. End every behaviour change with a
    manual checklist ("open X, play Y, expect Z").
11. A decision that changes user-visible behaviour and isn't specified: ask. Don't
    guess.

## Docs protocol
1. **Docs change in the same commit** as the behaviour they describe. Never "docs
   later".
2. Per commit, update whichever applies:
   - **`ROADMAP.md`:** tick the task or change its status; done tasks are one line.
   - **`CHANGELOG.md`** Unreleased: one line for any change a user could notice.
   - **`DECISIONS.md`:** a dated log line for any decision made or changed (in chat
     or in an approved plan), with what it replaces; update the status table if it
     has a D-number.
   - **`README.md`:** if install, usage, status or the feature list changed.
   - **`docs/DESIGN.md`:** if a visual rule, screen, shortcut, copy or budget
     changed.
   - **Code doc comments:** if a domain definition changed. They are the source of
     truth; docs link to them rather than restating numbers.
3. **Every commit message ends with a trailer:** `Docs: <files updated>`, or
   `Docs: none — <reason>` (e.g. "refactor, no behaviour change"; for a config-only
   commit, `Docs: none — config` is enough).
   `scripts/hooks/commit-msg` rejects a commit without one (`just install-hooks`).
4. **Decisions from Claude chat** arrive as a pasted `DECISIONS — …` block. The first
   step is to record them in `DECISIONS.md` (and apply ROADMAP status changes),
   commit that alone, then plan the work.
5. **Plans in `~/.claude/plans` are scratch.** Anything decided in a plan that
   matters later lands in `DECISIONS.md` or `ROADMAP.md` before the phase ends.
6. **At the end of every milestone or phase**, and before a release, regenerate
   `docs/BRIEFING.md` from the other docs (≤ 250 lines, no claim they don't make),
   and say it's ready to paste.
7. **Weekly, or on "docs check":** look for contradictions between the docs and the
   code, and report them. References to documents that no longer exist must return
   nothing:
   ```sh
   git grep -nE '(00[34] §|MVP §|ADR-[0-9]|\bN?FR-[0-9]|\bC-[0-9]{3}\b|\bS0\b|Amendment [A-Z])' \
     -- ':!docs/archive' ':!*.svg' ':!CLAUDE.md'
   ```
   (SVG path data such as `C-432` would match, hence `:!*.svg`.)
8. **An idea mid-task:** one dated line in the ROADMAP Parking lot. Nothing else.

## Product principles (detail in `docs/PRODUCT.md`)
- Local-first: no accounts, no network calls, no telemetry.
- No gamification: no XP, streaks or badges. Accuracy % and tempo may appear, never
  as the headline; show facts, never the combined difficulty number.
- Helping the user decide what to practise is the product.
- Feedback is factual and specific ("bar 9: paused 4 s, 2 wrong notes"), never
  judgmental.
