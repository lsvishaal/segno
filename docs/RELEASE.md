# Hogwash — release

How a release is made and launched.

---

## 24. Release and launch playbook

### 24.1 README outline (< 150 lines)

1. One-sentence pitch + one screenshot (the Progress page grid, once M2.10 is built).
2. A 20-second GIF: loop a bar, the counter reaches 3, then the Progress page.
3. **What it does** — five bullets: waits for you · start anywhere · loop bars ·
   remembers your weak bars · free and local.
4. **Install (Linux)** — AppImage: download, `chmod +x`, run (include the
   lowercase-extension and FUSE notes from your own CachyOS experience).
   `.deb` for Debian/Ubuntu.
5. **Getting music** — bundled pieces; MusicXML from MuseScore Studio
   (File → Export), OpenScore, Mutopia; why MIDI and PDF aren't read directly.
6. **Honest comparison** — a short table: Hogwash vs MidiSlayer vs
   midipiano.app vs Synthesia (price, open source, platforms, weak-bar
   memory, MIDI import). Be fair; say where others are ahead.
7. **Principles** — local-first, no gamification, factual feedback.
8. **Building from source** — `just`, Verovio, Node.
9. **Licence and credits** — Apache-2.0, Verovio (LGPL), bundled piece sources.

### 24.2 Demo video shot list (60–90 s, no voice-over needed, captions)

1. (0–5 s) Library → click Prelude in C. Caption: *"Open a piece."*
2. (5–20 s) Hands on the FP-10; cursor waits on a chord until all notes are
   pressed. *"It waits for you."*
3. (20–30 s) A wrong note: the red mark on the staff, then the right note.
   *"It shows exactly what you played."*
4. (30–45 s) Click bar 20 → Shift-click bar 23 → loop; counter 1, 2, 3 clean.
   *"Loop the hard bars."*
5. (45–60 s) Stop; Results, then the Progress page shows the weak bars. *"It remembers where
   you struggle."*
6. (60–70 s) "Practise weak bars." *"And tells you what to practise next."*
7. End card: *"Hogwash — free and open source. Linux."* + repo URL.

### 24.3 Launch posts (v1.0 week)

- **r/pianolearning** and **r/piano** — read the self-promotion rules first;
  lead with the problem ("I kept replaying whole pieces to fix two bars"),
  not the app. Answer every comment.
- **Show HN** — title: *"Show HN: Hogwash – a free piano practice app that
  remembers which bars you struggle with."* First comment: why, stack
  (Rust + Tauri + Verovio), what's not done.
- **LinkedIn** — the build story (pairs with your 29 Sep post: "I rested it
  because of the content problem; here's what changed").
- **Lemmy / Mastodon (#piano, #rust, #linux)** — short version with the GIF.
- **AlternativeTo** — list it as an alternative to MidiSlayer and Synthesia.

### 24.4 After launch: feedback intake

- GitHub issue templates: Bug (steps, piece, keyboard model, AppImage
  version), Idea (what were you trying to do?).
- A pinned "Known limits" issue linking `docs/PRODUCT.md`, Known limits.
- Weekly: read everything, add one line per real signal to the Parking lot,
  reply to everyone. Don't promise dates.

### 24.5 Release checklist (each release)

- [ ] `just ci` green on main, locally and on GitHub
- [ ] Version bumped in `tauri.conf.json` and `Cargo.toml`
- [ ] Regression pieces played by hand (`docs/TESTING.md`, regression pieces)
- [ ] AppImage tested on a clean system
- [ ] `THIRD_PARTY.md` and `PROVENANCE.md` current
- [ ] `CHANGELOG.md`: "Unreleased" becomes the version, dated
- [ ] Release notes: what's new, known limits, how to report bugs
- [ ] Checksums attached
- [ ] Tag pushed

### 24.6 How a release is built (decided 25 Oct)

Releases are built by **CI from a version tag** (`v0.1.0`), not on this machine:
pushing the tag runs a workflow that builds the AppImage (and `.deb`), attaches
the files and their checksums to a GitHub release, and fails if the build isn't
reproducible from the tag. Until that workflow exists (M4.1), `just release`
builds the same AppImage locally for testing only.

