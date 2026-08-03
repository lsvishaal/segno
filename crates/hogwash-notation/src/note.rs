//! A Note in its written role (`003 §7.9`).

use hogwash_domain::{Pitch, PitchSpelling};

/// A single musical sound as a Score writes it.
///
/// `003 §7.9` models Note as **one concept in two roles** — written and
/// played. This is the written role: what the notation asks for. The played
/// role belongs to Performance Capture (`004 §8.5`), which produces sounding
/// pitches only, since a performer presses a key and a key has no spelling.
///
/// A written Note carries its spelling and nothing else. `003 §15` lists
/// duration among the things the domain does not model — Version 1 does not
/// assess it (`003 §7.4`), and modelling it would model deferred capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Note {
    spelling: PitchSpelling,
    /// Derived from `spelling` at construction, never set independently.
    ///
    /// Kept rather than recomputed so that [`sounding`](Self::sounding) is
    /// infallible without an unreachable branch. `PitchSpelling::sounding`
    /// returns an `Option`, and the only honest way to discharge it is to do
    /// so once, where the value can still be rejected.
    sounding: Pitch,
}

impl Note {
    /// Writes the note spelled `spelling`, or `None` if no instrument can
    /// sound it.
    ///
    /// Rejecting the unsoundable here rather than tolerating it is what makes
    /// [`sounding`](Self::sounding) infallible. Notation *can* write such a
    /// thing; `003 §7.14` says a malformed import becomes a Validation Issue,
    /// which is a decision taken at the boundary where the material arrives.
    /// By the time a Note exists it has passed that boundary, so Analysis
    /// never has to handle a note that cannot sound.
    #[must_use]
    pub const fn new(spelling: PitchSpelling) -> Option<Self> {
        match spelling.sounding() {
            Some(sounding) => Some(Self { spelling, sounding }),
            None => None,
        }
    }

    /// The spelling the Score wrote.
    ///
    /// Retained because `004 §8.8` requires Presentation to draw what the
    /// composer wrote, not what the key produced.
    #[must_use]
    pub const fn spelling(self) -> PitchSpelling {
        self.spelling
    }

    /// The pitch this note sounds.
    ///
    /// What Analysis compares (ADR-0001).
    #[must_use]
    pub const fn sounding(self) -> Pitch {
        self.sounding
    }
}

#[cfg(test)]
mod tests {
    use hogwash_domain::{Pitch, PitchSpelling, Step};

    use super::super::Note;

    // 003 §7.9 — "A Note has a pitch." In its written role it carries the
    // spelling the Score wrote, because Presentation must draw what the
    // composer wrote (ADR-0001).
    #[test]
    fn a_written_note_keeps_the_spelling_the_score_wrote() {
        let g_flat = PitchSpelling::new(Step::G, -1, 4);
        let note = Note::new(g_flat).unwrap();

        assert_eq!(note.spelling(), g_flat);
    }

    // ADR-0001 — correctness is decided on the sound, so a written Note must
    // be able to say what it sounds without the caller re-deriving it.
    #[test]
    fn a_written_note_sounds_the_pitch_its_spelling_names() {
        let middle_c = Note::new(PitchSpelling::new(Step::C, 0, 4)).unwrap();

        assert_eq!(middle_c.sounding(), Pitch::MIDDLE_C);
    }

    // Notation can write a pitch no instrument can produce. That is a fact
    // about the written material, and 003 §7.14 says it becomes a Validation
    // Issue at the boundary -- so it is rejected here, at construction, and
    // never becomes a Note that later has to apologise for itself.
    //
    // The payoff is that `sounding()` above is infallible, which keeps the
    // Option out of Analysis where it would be pure noise.
    #[test]
    fn a_spelling_no_instrument_can_sound_is_not_a_written_note() {
        let unplayable = PitchSpelling::new(Step::C, -2, -1);

        assert!(Note::new(unplayable).is_none());
    }

    // 003 §7.9 Domain Classification -- "two written Notes of the same pitch
    // at the same position are indistinguishable, and nothing in the domain
    // needs to tell them apart." So Note is compared by value, not identity.
    #[test]
    fn two_written_notes_of_the_same_pitch_are_indistinguishable() {
        let one = Note::new(PitchSpelling::new(Step::A, 0, 4)).unwrap();
        let other = Note::new(PitchSpelling::new(Step::A, 0, 4)).unwrap();

        assert_eq!(one, other);
    }

    // ...but enharmonics are not the same written Note, even though they
    // sound alike. 003 §7.10 requires the Score to hold the notation as
    // written; collapsing these would lose what Presentation has to draw.
    #[test]
    fn enharmonic_spellings_are_different_written_notes() {
        let f_sharp = Note::new(PitchSpelling::new(Step::F, 1, 4)).unwrap();
        let g_flat = Note::new(PitchSpelling::new(Step::G, -1, 4)).unwrap();

        assert_ne!(f_sharp, g_flat);
        assert_eq!(f_sharp.sounding(), g_flat.sounding());
    }

    // 003 §15 lists timing and duration among the things the domain does not
    // model, because Version 1 does not assess them (003 §7.4, MVP §8).
    //
    // Asserted by round-trip rather than by inspecting the API: a Note is
    // fully reconstructible from its spelling alone. Give it an independent
    // duration -- or any other attribute the Score would have to supply --
    // and this stops holding, because the round trip cannot carry it.
    #[test]
    fn a_written_note_is_fully_determined_by_its_spelling() {
        let note = Note::new(PitchSpelling::new(Step::E, 0, 5)).unwrap();

        assert_eq!(Note::new(note.spelling()), Some(note));
    }
}
