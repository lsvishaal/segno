# Provided practice material

Every piece shipped here must be lawfully redistributable, with its source,
licence and any attribution obligations recorded below.

Public-domain *music* is not the same thing as a public-domain *file*: a
downloaded MusicXML carries whatever licence the person who typed it in
attached. So each file has two statuses, recorded separately.

## Encoded by hand by Claude (Anthropic's model), for Segno when it was named Hogwash

These three files were written by Claude, not by the owner. Their header
comments still say "written for Hogwash"; the owner asked for them to be left
as written (3 Nov). A bundled piece's ID is its fixed tag, so an edit would now
make a new version, not a new piece.

| File | Work | Composer | Status of the music | Status of this file |
|---|---|---|---|---|
| `ode-to-joy.musicxml` | Ode to Joy (theme, Symphony No. 9) | Ludwig van Beethoven (d. 1827) | Public domain worldwide | Written by Claude, Apache-2.0 |
| `lightly-row.musicxml` | Lightly Row | Traditional (German folk melody) | Public domain; no known author | Written by Claude, Apache-2.0 |
| `fur-elise.musicxml` | Für Elise (opening, simplified) | Ludwig van Beethoven (d. 1827) | Public domain worldwide | Written by Claude, Apache-2.0 |

**`fur-elise.musicxml` is a simplified arrangement, not a transcription of a
published edition.** Its first two bars set the opening motif (E D♯ E D♯ E B D
C) over a plain alternating A3/E3 bass, this project's own reduction; bars
three and four are an original continuation, not the rest of Beethoven's
piece. The arrangement and its rhythmic simplification are Claude's, for this project.

## Downloaded from MuseScore

| File | Work | Composer | Source | Status of the music | Status of this file |
|---|---|---|---|---|---|
| `minuet-in-g.musicxml` | Minuet in G Major, BWV Anh. 114 (Notebook for Anna Magdalena Bach) | Christian Petzold (d. 1733) | musescore.com/score/2086106 | Public domain worldwide | Public domain — the file states "Public Domain (PianoXML typeset)" |
| `prelude-in-c.musicxml` | Prelude in C Major, BWV 846 (The Well-Tempered Clavier, Book I) | Johann Sebastian Bach (d. 1750) | musescore.com/score/117279 | Public domain worldwide | Public domain, as confirmed by the project owner (the file itself carries no rights statement) |

Both were downloaded as compressed `.mxl` (MuseScore 2 exports) and changed
only by: unpacking the score from the container, and adding a `<work-title>`
and a `<creator type="composer">` so the library can name them (the originals
carry the title and composer only as page text). The notes are untouched.

**Attribution obligations: none.**

## Test fixtures (not bundled)

Used only by tests; never shipped in the app.

| File | Work | Composer | Source | Status of the music | Status of this file |
|---|---|---|---|---|---|
| `tests/fixtures/schlusslied-des-narren.musicxml` | Schlusslied des Narren, Op. 127 No. 5 (voice and piano) | Robert Schumann (d. 1856) | OpenScore Lieder, CC0 corpus: `scores/Schumann,_Robert/5_Lieder_und_Gesänge,_Op.127/5_Schlusslied_des_Narren/lc6828018.mxl` at commit `40d212cea9f858c4e83e8bfa46ce729ef939606a` of https://github.com/OpenScore/Lieder | Public domain worldwide | CC0 1.0: the repository's licence, and the file's own `<rights>OpenScore (CC0)</rights>`; transcribed from IMSLP #271937 |

Changed only by unpacking `score.xml` from the `.mxl` (blob `1a0b076`, checked against
the repository). It opens with a pickup (bar `0`, `implicit="yes"`) and ends on a
short bar, which is why it's here (Phase 2.6).

## What each is for

Each exercises something the practice loop has to get right.

- **Lightly Row** — single notes, stepwise, four bars.
- **Ode to Joy** — single notes with repeated pitches, where a cursor that
  advanced on the wrong thing would be caught.
- **Für Elise** — a second staff: right- and left-hand notes that sound
  together are one position.
- **Minuet in G** — repeat signs (`|: 1–16 :| |: 17–32 :|`): the cursor
  follows the performed order, so each half is played twice.
- **Prelude in C** — 68 tied notes held in the left hand: a held note is
  struck once and never waited for again.

## Adding a piece

1. Encode it by hand, or record the file's own licence — not merely that the
   music is public domain.
2. Add a row above with every column filled in.
3. Record any attribution obligation, or state that there is none.
