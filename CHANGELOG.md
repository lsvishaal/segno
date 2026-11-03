# Changelog

All notable changes a user could notice. Format:
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow
[SemVer](https://semver.org/).

## [Unreleased]

Everything so far, before the first preview (v0.1).

### Changed
- **Renamed to Segno.** The window, About and library say Segno; your practice lives in
  `~/.local/share/com.segno.practice`. History and settings from before the rename aren't
  carried over.
- **A Little Chord Study is no longer bundled:** five starter pieces remain.
- **History survives score changes:** each session is judged against the version of the
  score it was played on, so an edited score never changes past results. When a piece's
  bars change, its Progress page says "This piece was updated on …" and shows bar by bar
  only what was played since; the totals still count everything.

### Added
- **Settings are kept with your history:** marks, the remembered keyboard and
  zoom move to `settings.json` in the data folder.
- **A log file:** warnings, errors and crashes go to `segno.log` in the data
  folder (at most about 2 MB). If the system gives no data folder, Segno says
  so instead of saving in the current folder.
- **Library:**
  - bundled pieces on Beginner and Intermediate shelves
  - MusicXML and MXL import, with clear refusals for MIDI, PDF and damaged files
  - rename and delete
  - a Continue card for the piece played last
  - "last practised" and the weakest bars on each card
- **Practice:**
  - a cursor that waits for the right notes (chords, ties, repeats, grace notes)
  - start from any bar
  - loop a range of bars, with a clean-passes counter and an offer to widen the
    loop
  - restart with R
  - zoom
  - note names
  - MIDI Thru
  - a minute's grace when the keyboard is unplugged
  - auto-finish at the end of a piece
- **Results after a run:**
  - accuracy, tempo and steadiness
  - up to three cards on what to work on (the bars cut out of the score, what
    happened, why, how to practise it, and a loop button)
  - what went well
  - a per-bar chart
- **History:**
  - every note saved with its timing and velocity
  - **a Progress page per piece** (from each card, Continue, Results and the
    practice screen): what to practise next with a button that loops it, a map
    of the piece with a word for each bar (Secure, Small slips, Uneven, Shaky,
    Needs work) and a legend, and the bar you pick graphed over time (hesitation
    and wrong notes, each time through), all in plain sentences. It replaces the
    amber tint on the score, its legend and the W key.
  - Practise on the Progress page starts at once on the keyboard you last
    started on, which Segno now remembers
- **Marks while playing:** each right key's note turns green and stays; a wrong key stays as a red note with a ring (at most 3 in one place). They clear on a new attempt and on each pass of a loop, and stay on the score after Results until the next Start. Settings › Marks while playing turns them back into a brief flash. The ✕ counts the score used to show after a run are gone.
- **Speed:** a key reaches the screen in about 15 ms (it was about 27): the keyboard is read every 5 ms while playing, not every 25 ms.
- **Coach cards:** every Practise button loops from one bar before the trouble; "You keep stopping at bar N" now loops bars N−1–N instead of starting at bar N.
- **Weak bars:** each bar is judged on one fixed scale, the same for every piece, from its own last 3 sessions, so a piece played well has no weakest bars. A bar needs 2 passes before it's judged.
- **Speed:** switching screens is a 120 ms cross-fade instead of about 0.4 s of fades.
- **Settings:** MIDI input and output, note names, zoom, MusicXML export, About.
