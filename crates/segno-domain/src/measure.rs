//! Where in the music something happened.

use core::num::NonZeroU32;

/// The number of a Measure as the Score writes it.
///
/// *"Measures are numbered from one, following the convention
/// performers use."* That convention is carried by the type rather than left
/// to call sites, because the failure it prevents is user-visible: Feedback
/// exists to tell a performer **where** something happened, and
/// "bar 0" is not a place they can look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MeasureNumber(NonZeroU32);

impl MeasureNumber {
    /// The first measure of any Score.
    pub const FIRST: Self = Self(NonZeroU32::MIN);

    /// Returns the measure numbered `number`, or `None` if it is zero.
    #[must_use]
    pub const fn new(number: u32) -> Option<Self> {
        match NonZeroU32::new(number) {
            Some(number) => Some(Self(number)),
            None => None,
        }
    }

    /// The number as a performer would say it.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

/// A place in the written music: a Measure, and which position within it.
///
/// The design requires Feedback to locate a finding *"by Measure and Note"*,
/// and this is that pair given a name. It introduces no concept — a Measure
/// and the ordered positions within it both already
/// exist; what was missing was a single value carrying them together.
///
/// It is also the project's **only durable anchor into a score**. The Verovio
/// spike found that engraving identifiers are regenerated on every
/// render and may be renumbered by an upgrade, so nothing outside the
/// engraving adapter may hold one. A bar number and a position survive both.
///
/// `position` is the index of the Position within the Piece's **Reference
/// Performance** — it counts from zero and runs across the whole piece rather
/// than restarting each bar, because that is the sequence the Cursor walks
///. Carrying the Measure alongside it means a finding can say
/// which bar to look at without consulting the Reference Performance again.
///
/// It is an index into a sequence, not something a performer says aloud, so
/// it carries no from-one convention the way `MeasureNumber` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Location {
    // Field order is the ordering: `PartialOrd` derives lexicographically, so
    // declaring `measure` first is what makes bar 2 sort before bar 10.
    // Reordering these fields would silently change how findings are listed.
    measure: MeasureNumber,
    position: usize,
}

impl Location {
    /// The `position`-th position of `measure`.
    #[must_use]
    pub const fn new(measure: MeasureNumber, position: usize) -> Self {
        Self { measure, position }
    }

    /// The bar this is in.
    #[must_use]
    pub const fn measure(self) -> MeasureNumber {
        self.measure
    }

    /// Which position within that bar, counting from zero.
    #[must_use]
    pub const fn position(self) -> usize {
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // "Measures are numbered from one, following the convention
    // performers use." Carrying that in the type means an off-by-one cannot
    // reach the performer as "bar 0", which is not a thing they can look at.
    #[test]
    fn measures_are_numbered_from_one() {
        assert_eq!(MeasureNumber::FIRST.get(), 1);
    }

    #[test]
    fn there_is_no_measure_zero() {
        assert!(MeasureNumber::new(0).is_none());
    }

    #[test]
    fn measures_order_as_performers_count_them() {
        let first = MeasureNumber::new(1).unwrap();
        let fourth = MeasureNumber::new(4).unwrap();

        assert!(first < fourth);
    }

    fn bar(number: u32) -> MeasureNumber {
        MeasureNumber::new(number).unwrap()
    }

    // Feedback locates a finding "by Measure and Note". A place in
    // the music is that pair, and naming it once means the bar number and the
    // index cannot drift apart on their way through the system.
    #[test]
    fn a_location_is_a_measure_and_a_position_within_it() {
        let location = Location::new(bar(4), 2);

        assert_eq!(location.measure(), bar(4));
        assert_eq!(location.position(), 2);
    }

    // The order is reading order, and it must be, because Location is the key
    // the engraving adapter maps to a drawn glyph and the anchor a
    // Practice Record stores. A set of Locations iterated in any other order
    // would report bar 10 before bar 2.
    #[test]
    fn locations_order_by_measure_first_then_by_position() {
        let mut places = [
            Location::new(bar(10), 0),
            Location::new(bar(2), 1),
            Location::new(bar(2), 0),
        ];
        places.sort();

        assert_eq!(
            places,
            [
                Location::new(bar(2), 0),
                Location::new(bar(2), 1),
                Location::new(bar(10), 0),
            ]
        );
    }
}
