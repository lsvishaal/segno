# Provided practice material

`MVP §9.2` makes this a release gate, and it is not a code one:

> Every piece in the provided library is verified as lawfully
> redistributable, with provenance, licence, and any attribution obligations
> recorded alongside it (C-200).

## Every piece here was encoded by hand, for this project

That is the whole strategy, and it is worth stating why rather than assuming
it reads as laziness or as caution.

Public-domain *music* is not the same thing as a public-domain *file*. A
downloaded MusicXML of a Beethoven melody carries whatever licence the person
who typed it in attached — and very often carries none at all, which is worse,
because the absence of a statement is not permission. Verifying the provenance
of a downloaded score means tracing an editorial chain that frequently cannot
be traced.

Encoding the melodies here by hand removes the question instead of answering
it. The music is public domain; the file is ours.

The cost is that the library is small and the pieces are simple. For a first
session that is arguably right anyway — `MVP §9.1` asks that a new user
complete a full attempt without obtaining any file — but it is a real limit,
and growing the library means either encoding more or doing the verification
work on someone else's file.

## The pieces

| File | Work | Composer | Status of the music | Status of this file |
|---|---|---|---|---|
| `ode-to-joy.musicxml` | Ode to Joy (theme, Symphony No. 9) | Ludwig van Beethoven (d. 1827) | Public domain worldwide | Written for Hogwash, Apache-2.0 |
| `lightly-row.musicxml` | Lightly Row | Traditional (German folk melody) | Public domain; no known author | Written for Hogwash, Apache-2.0 |
| `a-little-chord-study.musicxml` | A Little Chord Study | Hogwash | Original exercise | Apache-2.0 |
| `fur-elise.musicxml` | Für Elise (opening, simplified) | Ludwig van Beethoven (d. 1827) | Public domain worldwide | Written for Hogwash, Apache-2.0 |

**Attribution obligations: none.** No piece here derives from a third party's
edition, engraving, arrangement or typesetting.

**`fur-elise.musicxml` is explicitly a simplified arrangement, not a
transcription of a published edition.** Its first two bars set the melody's
famous opening motif (E D♯ E D♯ E B D C) over a plain alternating A3/E3
bass, which is this project's own reduction of the harmony rather than a copy
of any edition's engraving; bars three and four are an original continuation
written for this exercise, not the rest of Beethoven's piece. Naming it after
the real work rather than disguising the source, while being explicit that
the arrangement and its rhythmic simplification (no eighth-note figuration,
straight quarters and halves rather than the original's compound metre) are
Hogwash's own — the same standard `a-little-chord-study.musicxml` is held to
for being original.

## What each is for

Not decoration — each exercises something the practice loop has to get right.

- **Lightly Row** — single notes, stepwise, four bars. The shortest thing that
  is recognisably a piece rather than an exercise.
- **Ode to Joy** — single notes with repeated pitches and a dotted rhythm.
  Repeated pitches matter: they are where a cursor that advanced on the wrong
  thing would be caught.
- **A Little Chord Study** — three triads. A chord is one Position with
  several Notes (`003 §7.4`), and it is the shape most likely to be got wrong
  by an implementation that only ever saw single notes.
- **Für Elise** — the first piece with a second staff. A grand staff's right-
  and left-hand notes that sound together are still one Position (`003 §7.4`)
  even though they come from different staves in the written score; every
  other provided piece is single-staff, so this is the only one that
  exercises that merge at all.

## Adding a piece

1. Encode it by hand, or verify the provenance of the file properly — which
   means an explicit licence from the encoder, not merely public-domain music.
2. Add a row above, with both statuses filled in. Both columns exist because
   they are genuinely different questions.
3. Record any attribution obligation, or state that there is none.

A piece with an empty cell in that table is not verified, and `MVP §9.2` is
not satisfied while one is here.
