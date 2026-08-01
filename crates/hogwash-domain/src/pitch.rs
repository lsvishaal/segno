//! Pitch: what a key sounds, and how a note is written.
//!
//! These are two different things, and keeping them apart is the point of
//! this module.
//!
//! A performer plays a *key*, which produces a sound. A Score writes a
//! *spelling* — F sharp, or G flat, which are the same key. `003 §7.6`
//! scopes Analysis to *"whether the right notes were played"*, and a
//! performer cannot play a spelling, so correctness is decided on the sound
//! ([`Pitch`]). Notation keeps the spelling ([`PitchSpelling`]) because
//! Presentation must draw what the composer wrote.
//!
//! Collapsing the two would report every correctly-played G flat in a
//! sharp-spelled score as a wrong note. See ADR-0001.

/// A sounding pitch — what a key produces.
///
/// Numbered as MIDI numbers them, 0..=127, because that is the range an
/// instrument can actually sound and the form live input arrives in. The
/// numbering is a representation detail; the concept is the sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pitch(u8);

impl Pitch {
    /// The lowest sounding pitch.
    pub const LOWEST: Self = Self(0);

    /// The highest sounding pitch.
    pub const HIGHEST: Self = Self(127);

    /// Middle C — C in octave 4, and the anchor for octave numbering.
    pub const MIDDLE_C: Self = Self(60);

    /// Returns the pitch numbered `number`, or `None` if no instrument can
    /// sound it.
    #[must_use]
    pub const fn new(number: u8) -> Option<Self> {
        if number <= Self::HIGHEST.0 {
            Some(Self(number))
        } else {
            None
        }
    }
}

impl From<Pitch> for u8 {
    fn from(pitch: Pitch) -> Self {
        pitch.0
    }
}

/// The seven note names, before any accidental is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Step {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl Step {
    /// Semitones above C, within the octave.
    ///
    /// These are the gaps a keyboard has: there is no black key between E
    /// and F, or between B and C.
    const fn semitones_above_c(self) -> i32 {
        match self {
            Self::C => 0,
            Self::D => 2,
            Self::E => 4,
            Self::F => 5,
            Self::G => 7,
            Self::A => 9,
            Self::B => 11,
        }
    }
}

/// A written note name: a step, an alteration, and an octave.
///
/// This is notation. Two spellings that sound identical are still different
/// spellings, and remain distinguishable here — that is what makes it
/// possible for Presentation to draw what was written while Analysis
/// compares what was heard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PitchSpelling {
    step: Step,
    /// Semitones of alteration: -1 flat, +1 sharp, -2/+2 double, 0 natural.
    alter: i8,
    /// Scientific pitch notation, where middle C is C4.
    octave: i8,
}

impl PitchSpelling {
    /// Writes a note as `step`, altered by `alter` semitones, in `octave`.
    #[must_use]
    pub const fn new(step: Step, alter: i8, octave: i8) -> Self {
        Self {
            step,
            alter,
            octave,
        }
    }

    /// The written note name.
    #[must_use]
    pub const fn step(self) -> Step {
        self.step
    }

    /// The accidental, in semitones.
    #[must_use]
    pub const fn alter(self) -> i8 {
        self.alter
    }

    /// The octave, where middle C is octave 4.
    #[must_use]
    pub const fn octave(self) -> i8 {
        self.octave
    }

    /// The pitch this spelling sounds, if an instrument can sound it.
    ///
    /// Returns `None` for a spelling outside the sounding range. Notation
    /// can write such a thing; no keyboard can produce it, and clamping it
    /// to the nearest playable pitch would invent a note the composer did
    /// not write.
    ///
    /// This returns an `Option` rather than offering a panicking companion,
    /// because spellings arrive from imported files and a malformed one must
    /// become a Validation Issue (`003 §7.14`), never a crash.
    #[must_use]
    pub const fn sounding(self) -> Option<Pitch> {
        // Middle C is C4 = 60, so C0 = 12 and the octave offset is
        // (octave + 1) * 12.
        let number =
            (self.octave as i32 + 1) * 12 + self.step.semitones_above_c() + self.alter as i32;

        if number < 0 || number > Pitch::HIGHEST.0 as i32 {
            return None;
        }

        // The bounds check above puts `number` inside 0..=127.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the check above bounds `number` within u8 before the cast"
        )]
        Some(Pitch(number as u8))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 003 §7.9 — a Note "has a pitch". A piano keyboard produces a bounded
    // set of them, and MIDI numbers them 0..=127. Anything outside that is
    // not a pitch an instrument can produce, so it is not constructible.
    #[test]
    fn a_pitch_above_the_midi_range_does_not_exist() {
        assert!(Pitch::new(128).is_none());
    }

    #[test]
    fn the_extremes_of_the_midi_range_are_pitches() {
        assert!(Pitch::new(0).is_some());
        assert!(Pitch::new(127).is_some());
    }

    // The finding that motivates separating spelling from sound.
    //
    // A performer plays a key; a score writes a spelling. F sharp and G flat
    // are different spellings of one key. 003 §7.6 scopes Analysis to
    // "whether the right notes were played", and the performer cannot play a
    // spelling -- so correctness has to be decided on the sound.
    //
    // Without this, every correctly-played G flat in a sharp-spelled score
    // would be reported as a wrong note. See ADR-0001.
    #[test]
    fn f_sharp_and_g_flat_sound_the_same_pitch() {
        let f_sharp = PitchSpelling::new(Step::F, 1, 4);
        let g_flat = PitchSpelling::new(Step::G, -1, 4);

        assert_eq!(f_sharp.sounding(), g_flat.sounding());
    }

    // ...but they remain distinguishable as notation, because Presentation
    // draws what the composer wrote, not what the key produced.
    #[test]
    fn f_sharp_and_g_flat_are_written_differently() {
        let f_sharp = PitchSpelling::new(Step::F, 1, 4);
        let g_flat = PitchSpelling::new(Step::G, -1, 4);

        assert_ne!(f_sharp, g_flat);
    }

    // Middle C is MIDI 60 in the convention where it is C4. This anchors the
    // octave numbering so the rest of the mapping cannot drift by an octave
    // without a test noticing.
    #[test]
    fn middle_c_is_midi_sixty() {
        assert_eq!(
            PitchSpelling::new(Step::C, 0, 4).sounding(),
            Some(Pitch::MIDDLE_C)
        );
    }

    #[test]
    fn a_double_sharp_sounds_two_semitones_above_the_natural() {
        let c = PitchSpelling::new(Step::C, 0, 4).sounding().unwrap();
        let c_double_sharp = PitchSpelling::new(Step::C, 2, 4).sounding().unwrap();

        assert_eq!(u8::from(c_double_sharp), u8::from(c) + 2);
    }

    // B sharp in octave 4 sounds as C in octave 5 -- the alteration carries
    // across the octave boundary. A naive implementation that clamps or
    // wraps within the octave gets this wrong.
    #[test]
    fn an_alteration_may_cross_an_octave_boundary() {
        let b_sharp_4 = PitchSpelling::new(Step::B, 1, 4);
        let c_5 = PitchSpelling::new(Step::C, 0, 5);

        assert_eq!(b_sharp_4.sounding(), c_5.sounding());
    }

    // Spellings can be written that no instrument can sound -- a heavily
    // flattened note in the lowest octave falls off the bottom of the range.
    // That is a fact about the written note, so it is reported rather than
    // silently clamped to the nearest playable pitch.
    #[test]
    fn a_spelling_outside_the_sounding_range_has_no_pitch() {
        let far_below = PitchSpelling::new(Step::C, -2, -1);

        assert!(far_below.sounding().is_none());
    }
}
