# Hogwash — Roadmap

A checklist, not a narrative. `STATUS.md` is the prose handoff;
`docs/Backlog_From_Reference.md` is the record of what was deliberately
deferred and why. This file is just: what's left, and what's next.

One task in flight at a time.

**Now: UI / visual rehaul — verify against `ideal-pics/` and close it out.**

---

## Done

- [x] Rust core (14 crates, domain → notation → capture → practice →
      analysis → feedback, plus MIDI/Verovio/records adapters)
- [x] Practice loop end to end (MIDI in, wait-for-me cursor, live
      correctness marks, attempt summary, history kept per session)
- [x] MusicXML import (`import_musicxml`, wired to `hogwash-acquisition`
      + `hogwash-records`, joins the library at runtime)
- [x] MIDI Thru/output routing (`hogwash-midi::output`) — a second,
      independent connection to the input port forwarding raw bytes to a
      real synth/hardware output, alongside the existing correctness-focused
      capture path
- [x] UI / visual rehaul, first three passes: a real `SettingsPanel` with a
      custom `Dropdown` replacing the bare native `<select>`s for MIDI I/O,
      one green accent color, a spacing pass, a slow breathing cursor cap,
      default notation size doubled (`Layout::default().scale` 150→300).
      **Not yet checked side-by-side against `ideal-pics/`** — that visual
      QA pass is the one thing still open before this is fully closed out.

## Not started — build next

- [ ] **History screen.** `hogwash-records` is done and tested; there is no
      screen to browse past sessions. Small and well-scoped relative to the
      UI rehaul above — a candidate for right after, or to fold into it if
      the rehaul redesigns navigation anyway.
- [ ] **Packaged release build.** No `tauri build` recipe has ever been run;
      `just dev` only. Also untested on Windows/macOS — Linux only so far.
- [ ] **Housekeeping.** ADR-0012 (Tauri over egui — decided, not written
      down) superseding ADR-0011; IF-6 (the `usvg` `<use>` geometry bug that
      killed the egui approach) not written up in
      `docs/Implementation_Findings.md`.

## Needs a decision before it's built

Not defaulted to "yes, build it" — each is a real scope or architecture
call, not just unscheduled work.

- [ ] **Accept, revise, or drop Amendment A** (`docs/002_Amendment_A_Capability_Gaps.md`)
      — still Proposed. The code is already built to it; this is a
      paperwork call, not blocking.
- [ ] **Tempo/transpose** (BFR-2) — architectural, not just a slider.
      Hogwash has no timing model anywhere in the domain (`003 §15`
      excludes one on purpose); adding tempo means deciding whether that
      changes.
- [ ] **Fingering hints** (BFR-3) — MusicXML may already carry
      `<fingering>` data Verovio can render "as written"; an "Auto" mode
      would be a real feature of its own.
- [ ] **Game / retry mode** (BFR-6) — changes what "an Attempt" means
      (`003 §7.3`'s current model is simpler: play to the end or stop).
- [ ] **Combo / hit-miss counters** (BFR-7) — flagged explicitly as in
      tension with `001 §11`'s no-grading, no-difficulty-rating rule. Needs
      an explicit yes, not a quiet addition.
- [ ] **PDF / ABC / MIDI export** (BFR-1) — MusicXML export is done; the
      others need either a Verovio PDF render path (unverified whether the
      build exposes it) or a format-conversion crate Hogwash doesn't have.

## Stretch / open questions, not scheduled

- Larger piece library — 4 bundled pieces today; growing it means either
  hand-encoding more (the current approach, for licensing reasons — see
  `app/src-tauri/library/PROVENANCE.md`) or reviewing someone else's file.
- Windows build friction for Verovio — untested, flagged since `ADR-0005`.
