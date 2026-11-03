//! What an engraver returns, and what it must implement.

use std::collections::BTreeMap;
use std::collections::btree_map::Iter;

use segno_domain::{Location, MeasureNumber};
use segno_notation::Score;

/// A drawn element's identifier, as the engraving engine names it.
///
/// **Ephemeral.** Measured: Verovio regenerates these on every
/// render — they differ between two renders in the same process, and an
/// engine upgrade may renumber them all. One is meaningful only for the
/// [`EngravedScore`] it came from.
///
/// So this is deliberately opaque: it can be created, compared and handed to
/// the renderer that issued it, and it has no structure to reason about.
/// Nothing persists one. `Location` is the durable anchor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElementId(String);

impl ElementId {
    /// The element the engine calls `id`.
    #[must_use]
    pub const fn new(id: String) -> Self {
        Self(id)
    }

    /// The identifier as the engine wrote it, for handing back to it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Which rendered page, counting from zero.
///
/// From zero rather than one, unlike [`MeasureNumber`], because a page is an
/// artefact of rendering rather than something the Score writes: a performer
/// says "bar 12", never "page 2 of the engraving I happen to be looking at".
///
/// [`MeasureNumber`]: segno_domain::MeasureNumber
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageIndex(usize);

impl PageIndex {
    /// The first page. Every engraving has one, or it drew nothing.
    pub const FIRST: Self = Self(0);

    /// The `index`-th page, counting from zero.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    /// The index, for reaching into the pages of an [`EngravedScore`].
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

/// Where the notes of one Position ended up on the page.
///
/// Several elements, not one, because a Position is *the notes required
/// together* and a chord is three or four glyphs. The design
/// shows correctness on the score, and a chord the performer got half right
/// has to be shown half right.
///
/// They share one page: notes that sound together are drawn on one system,
/// and a system does not straddle a page break.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Placement {
    page: PageIndex,
    elements: Vec<ElementId>,
}

impl Placement {
    /// The `elements` on `page`, in the order the Position writes its notes.
    ///
    /// Returns `None` if `elements` is empty: a placement that places nothing
    /// is not a placement, and permitting one would push the emptiness check
    /// out to every caller that draws a highlight.
    #[must_use]
    pub fn new(page: PageIndex, elements: Vec<ElementId>) -> Option<Self> {
        if elements.is_empty() {
            return None;
        }

        Some(Self { page, elements })
    }

    /// Which page they were drawn on.
    #[must_use]
    pub const fn page(&self) -> PageIndex {
        self.page
    }

    /// The drawn elements, in the Position's order. Never empty.
    #[must_use]
    pub fn elements(&self) -> &[ElementId] {
        &self.elements
    }
}

/// What a source file says about the work itself.
///
/// The requirement requires a Piece to carry *"at minimum a title and, where
/// available, a composer"*. Both are optional here and neither is optional
/// there: a source file may simply not say, and the Piece supplies what is
/// missing rather than the engraver inventing it.
///
/// It rides along with the engraving because it came from the same reading of
/// the same file. Asking for it separately would mean a second
/// parse to answer a question the first one already passed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Details {
    /// What the source calls the work.
    pub title: Option<String>,
    /// Who the source says wrote it.
    pub composer: Option<String>,
}

/// A Score, and how it was drawn.
///
/// The two arrive together because they came from one reading of one file
///. Separating them would mean correlating two parses, and the
/// symptom of them disagreeing is the cursor highlighting the wrong note.
///
/// Immutable once built, like the notation it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngravedScore {
    score: Score,
    details: Details,
    pages: Vec<String>,
    // A BTreeMap, not a HashMap: the iteration order is the Score's order on
    // every run, which the requirement requires and which the architecture
    // guard enforces for every pure crate.
    placements: BTreeMap<Location, Placement>,
    /// Each bar's drawn element, so a click on the page can say which bar.
    bars: Vec<(ElementId, MeasureNumber)>,
}

impl EngravedScore {
    /// Assembles an engraving, rejecting one that does not hang together.
    ///
    /// Validated at construction rather than trusted, because an adapter is
    /// the least reviewable code in the system and the damage from a bad
    /// mapping — a highlight drawn on the wrong note, or nowhere — surfaces
    /// far from its cause.
    ///
    /// # Errors
    ///
    /// [`MalformedEngraving::NothingWasDrawn`] if there are no pages, and
    /// [`MalformedEngraving::PlacementOnAMissingPage`] if a placement names a
    /// page that was not rendered.
    pub fn new(
        score: Score,
        details: Details,
        pages: Vec<String>,
        placements: Vec<(Location, Placement)>,
    ) -> Result<Self, MalformedEngraving> {
        if pages.is_empty() {
            return Err(MalformedEngraving::NothingWasDrawn);
        }

        for (at, placement) in &placements {
            if placement.page().get() >= pages.len() {
                return Err(MalformedEngraving::PlacementOnAMissingPage {
                    at: *at,
                    page: placement.page(),
                });
            }
        }

        Ok(Self {
            score,
            details,
            pages,
            placements: placements.into_iter().collect(),
            bars: Vec::new(),
        })
    }

    /// This engraving, knowing which drawn element is which bar.
    #[must_use]
    pub fn with_bars(mut self, bars: Vec<(ElementId, MeasureNumber)>) -> Self {
        self.bars = bars;
        self
    }

    /// Each bar's drawn element and its number.
    #[must_use]
    pub fn bars(&self) -> &[(ElementId, MeasureNumber)] {
        &self.bars
    }

    /// The written music this was engraved from.
    #[must_use]
    pub const fn score(&self) -> &Score {
        &self.score
    }

    /// What the source said about the work.
    #[must_use]
    pub const fn details(&self) -> &Details {
        &self.details
    }

    /// The rendered pages, in order.
    #[must_use]
    pub fn pages(&self) -> &[String] {
        &self.pages
    }

    /// Which drawn element stands for the note at `at`, if one does.
    ///
    /// `None` is not an error. A Reference Performance position always has a
    /// written note, but an engraver may legitimately fail to attribute a
    /// glyph to it — and the design degrades to no highlight rather than to a
    /// highlight in the wrong place.
    #[must_use]
    pub fn placement_of(&self, at: Location) -> Option<&Placement> {
        self.placements.get(&at)
    }

    /// Every placement, in reading order.
    pub fn placements(&self) -> Iter<'_, Location, Placement> {
        self.placements.iter()
    }
}

/// An engraving that does not hang together.
///
/// Distinct from [`EngravingFailure`]: this is a bad *result*, which means an
/// adapter is wrong, whereas an [`EngravingFailure`] means the engraving
/// could not be produced, which is usually the input's doing or the engine's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MalformedEngraving {
    /// No pages were rendered, so there is nothing to look at.
    NothingWasDrawn,

    /// A note was placed on a page that was never rendered.
    PlacementOnAMissingPage {
        /// The place in the music whose placement is wrong.
        at: Location,
        /// The page it claimed to be on.
        page: PageIndex,
    },
}

/// Turns written music into a Score and something drawable.
///
/// One method, because reading and drawing must come from one parse
/// (the design — see the crate documentation).
///
/// Takes bytes rather than a path: the design points dependencies inward, and
/// a trait that named a file would put the filesystem in a pure crate.
/// Whoever reads the file is the adapter's caller.
pub trait Engraver {
    /// Reads `source` and engraves it.
    ///
    /// Takes `&mut self` because an engraving engine holds the document it is
    /// working on. Saying so in the signature is more honest than hiding a
    /// cell inside an implementation, and it costs nothing: the design keeps
    /// everything on one thread with one owner.
    ///
    /// # Errors
    ///
    /// [`EngravingFailure`] if the engine is unavailable, or the source is
    /// not music it can read.
    fn engrave(&mut self, source: &[u8]) -> Result<EngravedScore, EngravingFailure>;
}

/// Why an engraving could not be produced.
///
/// Deliberately few and coarse. The design requires a failure to resolve into
/// something explainable to the performer, and a taxonomy finer than this
/// would be describing one engine's error messages — which is exactly the
/// detail a port exists to keep out.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EngravingFailure {
    /// The engine could not be used at all — not installed, not loadable.
    ///
    /// Nothing to do with the source. The design ships the engraving library
    /// as a separate replaceable file precisely so a performer can be told
    /// this rather than shown a crash.
    EngineUnavailable {
        /// What went wrong, for the log. Not for the performer.
        detail: String,
    },

    /// The engine read the source and did not find music it could engrave.
    Unreadable {
        /// What the engine said, to be turned into a Validation Issue by
        /// Acquisition & Validation.
        detail: String,
    },

    /// The engraving came out incoherent. An adapter defect.
    Malformed(MalformedEngraving),

    /// A bar is numbered with letters ("12a", or "X1" as `MuseScore` writes), which bar
    /// numbers can't carry yet (DECISIONS, 3 Nov, E3).
    LetteredBar {
        /// The number as written.
        written: String,
    },
}

impl From<MalformedEngraving> for EngravingFailure {
    fn from(malformed: MalformedEngraving) -> Self {
        Self::Malformed(malformed)
    }
}

#[cfg(test)]
mod tests {
    use segno_domain::{Location, MeasureNumber, PitchSpelling, Step};
    use segno_notation::{Measure, Note, Position, Score};

    use crate::engraved::{
        Details, ElementId, EngravedScore, MalformedEngraving, PageIndex, Placement,
    };

    fn bar(number: u32) -> MeasureNumber {
        MeasureNumber::new(number).expect("fixture measures are numbered from one")
    }

    /// A one-bar, two-note Score. The music is irrelevant here — these tests
    /// are about the mapping from a place in the music to a drawn glyph.
    fn score() -> Score {
        let note = |step| {
            Position::new(vec![
                Note::new(PitchSpelling::new(step, 0, 4)).expect("fixture notes sound"),
            ])
            .expect("a position has at least one note")
        };

        Score::new(vec![Measure::new(
            bar(1),
            vec![note(Step::C), note(Step::D)],
        )])
        .expect("a one-measure score is well formed")
    }

    fn placed(id: &str) -> Placement {
        Placement::new(PageIndex::FIRST, vec![ElementId::new(id.to_owned())])
            .expect("one element is not none")
    }

    // The point of the whole port: given a place in the music, which drawn
    // thing is it? The design shows correctness "on the score", which is
    // impossible without this lookup.
    #[test]
    fn a_place_in_the_music_resolves_to_a_drawn_element() {
        let engraved = EngravedScore::new(
            score(),
            Details::default(),
            vec!["<svg/>".to_owned()],
            vec![(Location::new(bar(1), 0), placed("note-1"))],
        )
        .expect("the placement names page one, which exists");

        assert_eq!(
            engraved.placement_of(Location::new(bar(1), 0)),
            Some(&placed("note-1"))
        );
    }

    #[test]
    fn a_place_that_was_not_drawn_resolves_to_nothing() {
        let engraved = EngravedScore::new(
            score(),
            Details::default(),
            vec!["<svg/>".to_owned()],
            vec![],
        )
        .expect("an engraving with no placements is structurally fine");

        assert_eq!(engraved.placement_of(Location::new(bar(1), 0)), None);
    }

    // A placement pointing at a page that was not rendered is a broken
    // engraving, and the failure it causes downstream — a highlight drawn
    // nowhere, or an index out of range — is far from the mistake. Rejecting
    // it at construction means an EngravedScore that exists is coherent.
    #[test]
    fn a_placement_on_a_page_that_does_not_exist_is_rejected() {
        let broken = EngravedScore::new(
            score(),
            Details::default(),
            vec!["<svg/>".to_owned()],
            vec![(
                Location::new(bar(1), 0),
                Placement::new(PageIndex::new(4), vec![ElementId::new("note-1".to_owned())])
                    .expect("one element is not none"),
            )],
        );

        assert_eq!(
            broken,
            Err(MalformedEngraving::PlacementOnAMissingPage {
                at: Location::new(bar(1), 0),
                page: PageIndex::new(4),
            })
        );
    }

    // An engraving with no pages has drawn nothing, whatever else it claims.
    #[test]
    fn an_engraving_with_no_pages_is_rejected() {
        assert_eq!(
            EngravedScore::new(score(), Details::default(), vec![], vec![]),
            Err(MalformedEngraving::NothingWasDrawn)
        );
    }

    // Placements are iterated in reading order on every
    // run, because a BTreeMap orders by its key and Location orders by
    // measure then position. A hash-ordered map would give a different order
    // per process, and the arch guard bans one in a pure crate for exactly
    // this reason.
    #[test]
    fn placements_are_iterated_in_reading_order() {
        let engraved = EngravedScore::new(
            score(),
            Details::default(),
            vec!["<svg/>".to_owned()],
            vec![
                (Location::new(bar(1), 1), placed("second")),
                (Location::new(bar(1), 0), placed("first")),
            ],
        )
        .expect("both placements name page one");

        let order: Vec<&str> = engraved
            .placements()
            .flat_map(|(_, placement)| placement.elements())
            .map(ElementId::as_str)
            .collect();

        assert_eq!(order, ["first", "second"]);
    }

    // - notation does not change once it exists. The Score an
    // engraving carries is the one that was read, and there is no way to
    // swap it for another.
    #[test]
    fn an_engraving_carries_the_score_it_was_read_from() {
        let engraved = EngravedScore::new(
            score(),
            Details::default(),
            vec!["<svg/>".to_owned()],
            vec![],
        )
        .expect("well formed");

        assert_eq!(engraved.score().measures().len(), 1);
    }
}
