//! Looping a range of bars, and counting clean passes through it.

use core::ops::Range;

use hogwash_domain::Pitch;

use crate::cursor::Cursor;

/// A range of steps played over and over, and how many passes in a row
/// through it were clean.
///
/// A pass is clean when nothing wrong or unexpected was played in it. It
/// cannot miss a note: the Cursor only reaches the range's end once every
/// note in it has been played. Grace notes and ornaments do not count
/// against it. How long it took does not matter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loop {
    range: Range<usize>,
    target: Option<u32>,
    clean_in_a_row: u32,
    spoiled: bool,
}

/// What happens at the end of a pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pass {
    /// Round again, with this many clean passes in a row so far.
    Again { clean_in_a_row: u32 },
    /// The target was reached, this many clean passes in a row: the loop is
    /// over.
    Done { passes: u32 },
}

impl Loop {
    /// A loop over `range`, ending after `target` clean passes in a row, or
    /// never if `None`.
    #[must_use]
    pub const fn new(range: Range<usize>, target: Option<u32>) -> Self {
        Self {
            range,
            target,
            clean_in_a_row: 0,
            spoiled: false,
        }
    }

    /// The steps looped over.
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    /// How many clean passes in a row so far.
    #[must_use]
    pub const fn clean_in_a_row(&self) -> u32 {
        self.clean_in_a_row
    }

    /// Whether the Cursor has come to the end of the pass.
    #[must_use]
    pub fn is_at_the_end(&self, cursor: &Cursor) -> bool {
        cursor.position() >= self.range.end
    }

    /// The performer is about to play `pitch` where `cursor` stands. A note
    /// neither expected nor tolerated there spoils this pass. Returns
    /// whether that dropped the count of clean passes to zero.
    pub fn heard(&mut self, cursor: &Cursor, pitch: Pitch) -> bool {
        let expected = cursor
            .expecting()
            .is_some_and(|expected| expected.sounding().any(|note| note == pitch));
        if expected || cursor.tolerates(pitch) || self.spoiled {
            return false;
        }

        self.spoiled = true;
        let dropped = self.clean_in_a_row > 0;
        self.clean_in_a_row = 0;
        dropped
    }

    /// The pass was begun again from the range's start (Restart) before
    /// reaching its end: what was spoiled in it no longer counts. The count
    /// so far stands.
    pub const fn again(&mut self) {
        self.spoiled = false;
    }

    /// Ends the loop after `target` clean passes in a row from now on, or
    /// never if `None`.
    pub const fn aim_for(&mut self, target: Option<u32>) {
        self.target = target;
    }

    /// The pass reached the end of the range.
    pub fn wrapped(&mut self) -> Pass {
        if !self.spoiled {
            self.clean_in_a_row += 1;
        }
        self.spoiled = false;

        match self.target {
            Some(target) if self.clean_in_a_row >= target => Pass::Done {
                passes: self.clean_in_a_row,
            },
            _ => Pass::Again {
                clean_in_a_row: self.clean_in_a_row,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use hogwash_domain::Step;

    use crate::cursor::Cursor;
    use crate::fixtures::{pitch, two_bars, unexpected};
    use crate::looping::{Loop, Pass};

    /// Plays bar 2 (A, B) once through, `wrong` first if given, and ends
    /// the pass.
    fn pass(looping: &mut Loop, wrong: bool) -> Pass {
        let reference = two_bars(&[Step::D, Step::G], &[Step::A, Step::B]);
        let mut cursor = Cursor::starting_at(Arc::clone(&reference), 2);
        if wrong {
            looping.heard(&cursor, unexpected());
        }
        for step in [Step::A, Step::B] {
            looping.heard(&cursor, pitch(step));
            cursor.observe(pitch(step));
        }
        assert!(looping.is_at_the_end(&cursor));
        looping.wrapped()
    }

    #[test]
    fn clean_passes_count_up() {
        let mut looping = Loop::new(2..4, None);

        assert_eq!(pass(&mut looping, false), Pass::Again { clean_in_a_row: 1 });
        assert_eq!(pass(&mut looping, false), Pass::Again { clean_in_a_row: 2 });
    }

    #[test]
    fn a_wrong_note_drops_the_count_at_once_and_the_pass_does_not_count() {
        let reference = two_bars(&[Step::D, Step::G], &[Step::A, Step::B]);
        let mut looping = Loop::new(2..4, None);
        pass(&mut looping, false);

        let cursor = Cursor::starting_at(reference, 2);
        assert!(looping.heard(&cursor, unexpected()));
        assert_eq!(looping.clean_in_a_row(), 0);
        assert!(!looping.heard(&cursor, unexpected()), "already dropped");
        assert_eq!(looping.wrapped(), Pass::Again { clean_in_a_row: 0 });

        assert_eq!(pass(&mut looping, false), Pass::Again { clean_in_a_row: 1 });
    }

    #[test]
    fn a_note_of_the_next_step_is_wrong_here() {
        let reference = two_bars(&[Step::D, Step::G], &[Step::A, Step::B]);
        let mut looping = Loop::new(2..4, None);
        let cursor = Cursor::starting_at(reference, 2);

        looping.heard(&cursor, pitch(Step::B));

        assert_eq!(looping.wrapped(), Pass::Again { clean_in_a_row: 0 });
    }

    #[test]
    fn the_loop_is_done_at_its_target() {
        let mut looping = Loop::new(2..4, Some(2));

        assert_eq!(pass(&mut looping, true), Pass::Again { clean_in_a_row: 0 });
        assert_eq!(pass(&mut looping, false), Pass::Again { clean_in_a_row: 1 });
        assert_eq!(pass(&mut looping, false), Pass::Done { passes: 2 });
    }

    #[test]
    fn a_pass_begun_again_is_not_held_against_the_next() {
        let reference = two_bars(&[Step::D, Step::G], &[Step::A, Step::B]);
        let mut looping = Loop::new(2..4, None);
        looping.heard(&Cursor::starting_at(reference, 2), unexpected());

        looping.again();

        assert_eq!(pass(&mut looping, false), Pass::Again { clean_in_a_row: 1 });
    }

    #[test]
    fn the_target_can_change_while_looping() {
        let mut looping = Loop::new(2..4, Some(5));
        pass(&mut looping, false);

        looping.aim_for(Some(2));

        assert_eq!(pass(&mut looping, false), Pass::Done { passes: 2 });
    }

    #[test]
    fn without_a_target_it_never_ends() {
        let mut looping = Loop::new(2..4, None);

        for count in 1..=12 {
            assert_eq!(
                pass(&mut looping, false),
                Pass::Again {
                    clean_in_a_row: count
                }
            );
        }
    }
}
