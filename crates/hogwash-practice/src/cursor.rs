//! The Cursor — where the performer is expected to be (`003 §7.16`).

use std::collections::BTreeSet;

use hogwash_domain::{MeasureNumber, Pitch};
use hogwash_notation::{ExpectedNotes, ReferencePerformance};

/// Whether observing a note moved the Cursor.
///
/// **This is not a verdict.** `003 §7.16` — *"The Cursor does not judge what
/// was played."* A wrong note and a correct-but-incomplete one both yield
/// [`Held`](Self::Held), and the Cursor has no way to tell them apart.
/// Correctness is decided by Analysis (`004 §8.6`) after the Attempt ends,
/// over the whole performance.
///
/// Adding a `Correct` or `Incorrect` variant here would move a
/// responsibility across a component boundary `004 §14` makes governance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CursorMovement {
    /// The position was satisfied; the Cursor moved on.
    Advanced,
    /// The Cursor stayed where it was.
    Held,
}

/// The position in a Piece the performer is expected to play next
/// (`003 §7.16`).
///
/// It advances only when the Notes expected at its position have been played,
/// which means **it can never be ahead of the performer**. That single
/// property is what `MVP §5` calls the decision shaping Version 1: a bar
/// cannot be skipped, a pause needs no handling, and a system that has lost
/// its place cannot keep confidently scoring.
///
/// It borrows the Reference Performance rather than owning one. `003 §7.16`
/// says a Cursor's position "is always a position the Reference Performance
/// contains", and borrowing makes that a fact about the type: there is no
/// Cursor without the sequence it indexes into.
#[derive(Debug, Clone)]
pub struct Cursor<'a> {
    reference: &'a ReferencePerformance,
    position: usize,
    /// Which of the currently expected pitches have been played.
    ///
    /// Held as sounding pitches rather than as note indices because a chord
    /// may write one key twice — F♯ against G♭ — and a keyboard has one key
    /// for both (ADR-0001). Requiring two presses would ask for something no
    /// instrument can do.
    satisfied: BTreeSet<Pitch>,
}

impl<'a> Cursor<'a> {
    /// A Cursor resting on the first thing `reference` expects.
    ///
    /// `003 §7.16` Lifecycle — *"Begins at the first expected position when
    /// an Attempt begins."*
    #[must_use]
    pub fn at_the_beginning(reference: &'a ReferencePerformance) -> Self {
        Self {
            reference,
            position: 0,
            satisfied: BTreeSet::new(),
        }
    }

    /// How far through the Reference Performance the Cursor has come.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }

    /// What the performer is expected to play next, or `None` at the end.
    #[must_use]
    pub fn expecting(&self) -> Option<&'a ExpectedNotes> {
        self.reference.at(self.position)
    }

    /// The Measure the Cursor is standing in, or `None` at the end.
    ///
    /// `003 §7.16` — *"It occupies a position within a Measure, which is how
    /// a performer sees where it is."*
    #[must_use]
    pub fn measure(&self) -> Option<MeasureNumber> {
        self.expecting().map(ExpectedNotes::measure)
    }

    /// Whether every expected Note has been played.
    #[must_use]
    pub fn has_reached_the_end(&self) -> bool {
        self.position >= self.reference.len()
    }

    /// Whether `pitch` has already been played **at the current position**.
    ///
    /// A chord is satisfied one note at a time, and two components need to
    /// know which of them are down: Presentation, to show the performer what
    /// the chord still wants (`MVP §6.6`), and Analysis, so that a note which
    /// *was* played is not reported as one that never arrived (FR-402).
    ///
    /// Scoped to the current position. Progress belongs to the position being
    /// played, not to the piece, and is forgotten on advancing.
    #[must_use]
    pub fn has_been_played(&self, pitch: Pitch) -> bool {
        self.satisfied.contains(&pitch)
    }

    /// Observes that the performer played `pitch`, and reports whether the
    /// Cursor moved.
    ///
    /// An unexpected note is **ignored entirely** — it does not advance the
    /// Cursor, and it does not discard progress already made on a chord
    /// (ADR-0002). Two reasons, both from the documents:
    ///
    /// - `003 §7.16` says the Cursor does not judge what was played, and
    ///   discarding progress because a note was wrong is a judgement.
    /// - `MVP §5` requires that noodling and tuning up are not "silently
    ///   recorded as failure", and a reset is precisely a penalty for them.
    ///
    /// The note is not lost: `003 §7.5` records notes the Score never asked
    /// for, so it still reaches the Attempt's summary. Only the Cursor
    /// ignores it.
    pub fn observe(&mut self, pitch: Pitch) -> CursorMovement {
        let Some(expected) = self.expecting() else {
            // Past the end there is no position to satisfy. Playing on is
            // not an error, it simply moves nothing.
            return CursorMovement::Held;
        };

        if !expected.sounding().any(|expected| expected == pitch) {
            return CursorMovement::Held;
        }

        self.satisfied.insert(pitch);

        // A rolled chord satisfies its position: "all of them have been
        // played" is the only test available, because distinguishing
        // together from one-after-another needs a clock (003 §15, 004 §6.7).
        if expected
            .sounding()
            .all(|note| self.satisfied.contains(&note))
        {
            self.position += 1;
            self.satisfied.clear();
            CursorMovement::Advanced
        } else {
            CursorMovement::Held
        }
    }
}

#[cfg(test)]
mod tests {
    use hogwash_domain::{MeasureNumber, Step};

    use crate::cursor::{Cursor, CursorMovement};
    use crate::fixtures::{
        one_bar, one_chord, one_enharmonic_unison, pitch, two_bars, unexpected, unison_pitch,
    };

    // MVP §5, pause row -- "Long pause to work something out: the cursor
    // waits. No special handling needed."
    //
    // The pause cannot be expressed as elapsed time, because 004 §6.7 forbids
    // a clock here. It does not need to be: waiting is what the Cursor does
    // when nothing arrives, so "indefinitely" is the absence of any advance.
    #[test]
    fn the_cursor_waits_indefinitely_for_the_expected_note() {
        let reference = one_bar(&[Step::C, Step::D]);
        let cursor = Cursor::at_the_beginning(&reference);

        assert_eq!(cursor.position(), 0);
        assert_eq!(cursor.expecting(), reference.at(0));
    }

    // MVP §5 -- "Skipped bar: cannot occur -- the cursor will not move past
    // unplayed notes." Playing bar two's notes while bar one is unplayed
    // achieves nothing.
    #[test]
    fn a_bar_cannot_be_skipped() {
        let reference = two_bars(&[Step::C, Step::D], &[Step::E, Step::F]);
        let mut cursor = Cursor::at_the_beginning(&reference);

        assert_eq!(cursor.observe(pitch(Step::E)), CursorMovement::Held);
        assert_eq!(cursor.observe(pitch(Step::F)), CursorMovement::Held);

        assert_eq!(cursor.position(), 0);
        assert_eq!(cursor.measure(), Some(MeasureNumber::FIRST));
    }

    // MVP §5 -- "Wrong note played, user carries on: the cursor stays put.
    // Nothing downstream is mis-scored."
    #[test]
    fn a_wrong_note_does_not_advance_the_cursor() {
        let reference = one_bar(&[Step::C, Step::D]);
        let mut cursor = Cursor::at_the_beginning(&reference);

        assert_eq!(cursor.observe(pitch(Step::G)), CursorMovement::Held);

        assert_eq!(cursor.position(), 0);
    }

    // MVP §5 -- "Noodling, tuning up, playing something else: the cursor
    // holds. Nothing is silently recorded as failure."
    #[test]
    fn noodling_does_not_advance_the_cursor() {
        let reference = one_bar(&[Step::C]);
        let mut cursor = Cursor::at_the_beginning(&reference);

        for step in [Step::A, Step::B, Step::G, Step::A, Step::F] {
            assert_eq!(cursor.observe(pitch(step)), CursorMovement::Held);
        }

        assert_eq!(cursor.position(), 0);
    }

    #[test]
    fn the_cursor_advances_when_the_expected_note_is_played() {
        let reference = one_bar(&[Step::C, Step::D]);
        let mut cursor = Cursor::at_the_beginning(&reference);

        assert_eq!(cursor.observe(pitch(Step::C)), CursorMovement::Advanced);

        assert_eq!(cursor.position(), 1);
    }

    // 003 §7.16 -- the Cursor "advances only when those expected Notes have
    // been played". For a chord that means all of them; two of three is not
    // the position satisfied.
    #[test]
    fn a_chord_advances_only_when_all_its_notes_are_played() {
        let reference = one_chord(&[Step::C, Step::E, Step::G]);
        let mut cursor = Cursor::at_the_beginning(&reference);

        assert_eq!(cursor.observe(pitch(Step::C)), CursorMovement::Held);
        assert_eq!(cursor.observe(pitch(Step::E)), CursorMovement::Held);
        assert_eq!(cursor.position(), 0);

        assert_eq!(cursor.observe(pitch(Step::G)), CursorMovement::Advanced);
        assert_eq!(cursor.position(), 1);
    }

    // ADR-0002, stated consequence. A rolled chord satisfies its position.
    //
    // This is forced rather than chosen: telling "together" from "one after
    // another" requires measuring time, which 003 §15 does not model. 003
    // §7.16 says the Cursor advances when the expected Notes "have been
    // played" -- played, not held -- so this reading is also the literal one.
    #[test]
    fn a_rolled_chord_satisfies_its_position() {
        let reference = one_chord(&[Step::C, Step::E, Step::G]);
        let mut cursor = Cursor::at_the_beginning(&reference);

        // Deliberately in an order the score did not write them in.
        cursor.observe(pitch(Step::G));
        cursor.observe(pitch(Step::C));

        assert_eq!(cursor.observe(pitch(Step::E)), CursorMovement::Advanced);
    }

    // ADR-0002. The Cursor ignores unexpected notes entirely, so partial
    // progress on a chord survives a fumble.
    //
    // 003 §7.16 says the Cursor "does not judge what was played"; discarding
    // progress because a note was wrong is a judgement. MVP §5 also requires
    // that noodling not be "silently recorded as failure", and resetting is
    // precisely a penalty for it.
    //
    // The note is still recorded -- 003 §7.5 records notes the Score never
    // asked for -- so it still reaches the attempt summary. Only the Cursor
    // ignores it.
    #[test]
    fn an_unexpected_note_does_not_undo_progress_on_a_chord() {
        let reference = one_chord(&[Step::C, Step::E, Step::G]);
        let mut cursor = Cursor::at_the_beginning(&reference);

        cursor.observe(pitch(Step::C));
        cursor.observe(pitch(Step::E));
        cursor.observe(unexpected());

        assert_eq!(cursor.observe(pitch(Step::G)), CursorMovement::Advanced);
    }

    // 003 §7.16 Non-Responsibilities -- "The Cursor does not judge what was
    // played -- playing a wrong note is Analysis's concern, not the
    // Cursor's."
    //
    // Asserted by indistinguishability rather than by inspecting the API: a
    // plainly wrong note and a correct-but-incomplete one must produce the
    // same answer. If a Correct/Incorrect variant is ever added to
    // CursorMovement, these two stop matching and this fails.
    #[test]
    fn the_cursor_reports_no_verdict() {
        let reference = one_chord(&[Step::C, Step::E]);

        let mut after_wrong_note = Cursor::at_the_beginning(&reference);
        let wrong = after_wrong_note.observe(unexpected());

        let mut after_partial_chord = Cursor::at_the_beginning(&reference);
        let incomplete = after_partial_chord.observe(pitch(Step::C));

        assert_eq!(wrong, incomplete);
    }

    // 003 §7.16 -- "it can never be ahead of the performer". The position is
    // bounded by how many expected notes have actually been played, so no
    // sequence of input can outrun it.
    #[test]
    fn the_cursor_is_never_ahead_of_the_performer() {
        let reference = one_bar(&[Step::C, Step::D, Step::E]);
        let mut cursor = Cursor::at_the_beginning(&reference);

        for (played, step) in [Step::C, Step::D].into_iter().enumerate() {
            cursor.observe(pitch(step));
            assert_eq!(cursor.position(), played + 1);
        }

        assert_eq!(cursor.position(), 2);
    }

    // 003 §7.16 Business Invariant -- "A Cursor's position is always a
    // position the Reference Performance contains."
    #[test]
    fn a_cursor_position_is_always_one_the_reference_contains() {
        let reference = one_bar(&[Step::C, Step::D]);
        let mut cursor = Cursor::at_the_beginning(&reference);

        while !cursor.has_reached_the_end() {
            assert!(cursor.expecting().is_some());
            let expected = cursor.expecting().expect("checked just above");
            let next = expected.notes()[0].sounding();
            cursor.observe(next);
        }

        // Past the last position there is nothing to expect, and the Cursor
        // says so rather than pointing at a position that does not exist.
        assert_eq!(cursor.expecting(), None);
        assert_eq!(cursor.position(), reference.len());
    }

    #[test]
    fn the_cursor_reaches_the_end_when_every_note_has_been_played() {
        let reference = one_bar(&[Step::C, Step::D]);
        let mut cursor = Cursor::at_the_beginning(&reference);

        assert!(!cursor.has_reached_the_end());
        cursor.observe(pitch(Step::C));
        cursor.observe(pitch(Step::D));

        assert!(cursor.has_reached_the_end());
    }

    // Once the piece is finished there is nothing left to advance to, and
    // further playing must not move a position that no longer exists.
    #[test]
    fn playing_on_past_the_end_does_not_move_the_cursor() {
        let reference = one_bar(&[Step::C]);
        let mut cursor = Cursor::at_the_beginning(&reference);

        cursor.observe(pitch(Step::C));
        let end = cursor.position();

        assert_eq!(cursor.observe(pitch(Step::C)), CursorMovement::Held);
        assert_eq!(cursor.position(), end);
    }

    // A chord may write one key twice -- F sharp in one voice against G flat
    // in another. ADR-0001 establishes those are one sounding pitch, and a
    // keyboard has one key for them, so one press satisfies both. Requiring
    // two would ask for something no instrument can do.
    #[test]
    fn a_unison_written_twice_needs_only_one_key_press() {
        let reference = one_enharmonic_unison();
        let mut cursor = Cursor::at_the_beginning(&reference);

        assert_eq!(cursor.observe(unison_pitch()), CursorMovement::Advanced);
    }

    // A chord is satisfied one note at a time, and both Presentation and
    // Analysis need to know which of them are already down: Presentation to
    // show the performer (MVP §6.6), Analysis so it does not report a note
    // that was played as one that never arrived (FR-402).
    #[test]
    fn the_cursor_knows_which_of_the_expected_notes_are_already_played() {
        let reference = one_chord(&[Step::C, Step::E, Step::G]);
        let mut cursor = Cursor::at_the_beginning(&reference);

        assert!(!cursor.has_been_played(pitch(Step::C)));

        cursor.observe(pitch(Step::C));

        assert!(cursor.has_been_played(pitch(Step::C)));
        assert!(!cursor.has_been_played(pitch(Step::E)));
    }

    // Progress belongs to the position being played, not to the piece. Once
    // the Cursor moves on, the previous position's notes are history and the
    // new position starts from nothing.
    #[test]
    fn advancing_forgets_which_notes_satisfied_the_previous_position() {
        let reference = one_bar(&[Step::C, Step::D]);
        let mut cursor = Cursor::at_the_beginning(&reference);

        cursor.observe(pitch(Step::C));

        assert!(!cursor.has_been_played(pitch(Step::C)));
    }

    // 003 §7.16 -- "It occupies a position within a Measure, which is how a
    // performer sees where it is." Feedback locates findings by Measure
    // (003 §7.7), so the Cursor must be able to say which one it is in.
    #[test]
    fn the_cursor_knows_which_measure_it_is_in() {
        let reference = two_bars(&[Step::C], &[Step::D]);
        let mut cursor = Cursor::at_the_beginning(&reference);

        assert_eq!(cursor.measure(), Some(MeasureNumber::FIRST));

        cursor.observe(pitch(Step::C));

        assert_eq!(cursor.measure(), MeasureNumber::new(2));
    }
}
