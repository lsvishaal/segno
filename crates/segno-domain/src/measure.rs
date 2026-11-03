//! Where in the music something happened.

/// The number of a Measure as the Score prints it.
///
/// Bars are numbered from one, as performers count them, except a **pickup**:
/// the incomplete bar some pieces open with, which scores number 0. It is
/// shown as "Pickup", never "bar 0" (DECISIONS, 3 Nov), so only
/// [`MeasureNumber::PICKUP`] and [`MeasureNumber::printed`] can make one;
/// [`MeasureNumber::new`] still refuses 0, so arithmetic that slips below
/// bar 1 can't land on it by accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MeasureNumber(u32);

impl MeasureNumber {
    /// The first measure after any pickup.
    pub const FIRST: Self = Self(1);

    /// The pickup: an incomplete first bar, printed as bar 0.
    pub const PICKUP: Self = Self(0);

    /// Returns the measure numbered `number`, or `None` if it is zero (a
    /// pickup is made only on purpose: [`MeasureNumber::printed`]).
    #[must_use]
    pub const fn new(number: u32) -> Option<Self> {
        if number == 0 {
            None
        } else {
            Some(Self(number))
        }
    }

    /// The measure a score prints as `number`: 0 is the pickup.
    #[must_use]
    pub const fn printed(number: u32) -> Self {
        Self(number)
    }

    /// The number as printed; 0 for the pickup.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Whether this is the pickup.
    #[must_use]
    pub const fn is_pickup(self) -> bool {
        self.0 == 0
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
    fn bar_zero_is_only_ever_the_pickup_made_on_purpose() {
        assert!(MeasureNumber::new(0).is_none());
        assert_eq!(MeasureNumber::printed(0), MeasureNumber::PICKUP);
        assert!(MeasureNumber::PICKUP.is_pickup());
        assert!(!MeasureNumber::FIRST.is_pickup());
        assert!(MeasureNumber::PICKUP < MeasureNumber::FIRST);
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
