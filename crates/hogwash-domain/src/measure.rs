//! Where in the music something happened.

use core::num::NonZeroU32;

/// The number of a Measure as the Score writes it.
///
/// `003 §7.8` — *"Measures are numbered from one, following the convention
/// performers use."* That convention is carried by the type rather than left
/// to call sites, because the failure it prevents is user-visible: Feedback
/// exists to tell a performer **where** something happened (`003 §7.7`), and
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

#[cfg(test)]
mod tests {
    use super::*;

    // 003 §7.8 — "Measures are numbered from one, following the convention
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
}
