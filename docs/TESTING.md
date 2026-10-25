# Hogwash — testing

How Hogwash is tested.

---

## 22. Testing and quality

### 22.1 Automated (every task)

- `just ci` green: fmt, clippy `-D warnings`, tests, cargo-deny, native
  licences, architecture guard, determinism (two processes, byte-diff),
  frontend tests.
- Every engine feature (seek, loop, hands, per-bar stats) gets:
  - unit tests in its crate;
  - at least one `just replay` fixture under `crates/hogwash-cli/examples/`
    named `<piece>.<behaviour>.json` (e.g. `minuet-in-g.loop-9-12.json`).
- Old record formats keep loading: a test with a pre-M1.1 session file.

### 22.2 Manual checklist per behaviour change

Claude Code writes it; you run it with the FP-10 before ticking the box.
Format: *open X → play Y → expect Z.* Store the latest ones at the bottom of
`annoyances.md` or in the commit message.

### 22.3 Regression pieces

Before each release, play through these by hand:
1. Lightly Row (single line)
2. A Little Chord Study (chords)
3. Minuet in G (repeats, two hands) — with a loop across the repeat
4. Prelude in C (68 ties) — start from bar 20
5. One imported piece with a pickup bar

### 22.4 The real test

Seven days of self-use (§12), then one outside pianist (M4.9), then three
before v1.0. Their hesitations are the backlog.
