//! The [`Engraver`] Hogwash actually uses.

use std::path::{Path, PathBuf};

use hogwash_engraving::{EngravedScore, Engraver, EngravingFailure};

use crate::failure::TranslationFailure;
use crate::toolkit::{Toolkit, ToolkitError};
use crate::{mei, timemap, translate};

/// How the music should sit on the page.
///
/// # A practice score is scrolled, not turned
///
/// Verovio's defaults produce A4 sheets, which is right for its usual job of
/// printing and wrong here: a performer scrolls, and has no reason to inherit
/// the shape of a piece of paper.
///
/// `adjustPageHeight` is what this asks for. It shrinks a page to the music
/// actually on it, so a four-bar exercise is a four-bar strip rather than a
/// mostly-empty sheet.
///
/// **It does not make the page unbounded**, which was worth learning by
/// measurement rather than by assumption: a 400-bar score still comes back as
/// six pages, because `adjustPageHeight` trims a page rather than abolishing
/// pagination. So pages remain real and [`EngravedScore`] keeps carrying
/// several — the view stacks them. Fighting that would mean asking for one
/// enormous page, and an enormous page is an enormous texture.
///
/// The width is **not** the window's width. Re-engraving on every resize
/// would mean re-parsing the file to answer a question about a window, and
/// `MVP §6.4` only asks that all of the music be reachable. Pages are
/// rasterised to fit the window instead, at whatever size the view wants.
///
/// [`EngravedScore`]: hogwash_engraving::EngravedScore
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// Page width, in tenths of a millimetre. Verovio's own unit.
    pub width: u32,
    /// Staff size as a percentage. Larger is easier to read from a distance,
    /// which is where a performer sits.
    pub scale: u32,
}

impl Default for Layout {
    fn default() -> Self {
        // 2100 is A4's width in Verovio's units, and is wide enough to hold
        // several bars per system without crowding. 110% because the
        // performer is at arm's length from the screen rather than holding
        // the page.
        Self {
            width: 2100,
            scale: 110,
        }
    }
}

impl Layout {
    /// The options Verovio is given, as JSON.
    ///
    /// `header` and `footer` are turned off: Verovio prints a title and page
    /// numbers by default, which belong on paper and not on a practice
    /// surface where the application already says what the piece is.
    fn as_options(self) -> String {
        format!(
            r#"{{"pageWidth":{},"scale":{},"adjustPageHeight":true,"breaks":"auto","header":"none","footer":"none"}}"#,
            self.width, self.scale
        )
    }
}

/// Verovio, ready to engrave.
///
/// Holds one live toolkit. Opening the shared library is the expensive part
/// and it happens once, in [`open`](Self::open); after that each score is a
/// load and a render.
#[derive(Debug)]
pub struct VerovioEngraver {
    toolkit: Toolkit,
    layout: Layout,
}

impl VerovioEngraver {
    /// Opens the Verovio shared library and starts a toolkit.
    ///
    /// `library` is `libverovio.so` (or `.dll`, or `.dylib`) and `resources`
    /// is Verovio's `data/` directory. Both are shipped **beside** the
    /// application rather than inside it, which is `ADR-0007`: it is what
    /// makes the library replaceable, and replaceability is how LGPL-3.0 §4
    /// is satisfied.
    ///
    /// # Errors
    ///
    /// [`EngravingFailure::EngineUnavailable`] if the library is missing, is
    /// not Verovio, or will not start with those resources. This is reported
    /// rather than fatal because a performer who has moved a file deserves to
    /// be told which one (`004 §8.2`).
    pub fn open(library: &Path, resources: &Path) -> Result<Self, EngravingFailure> {
        Toolkit::open(library, resources)
            .map(|toolkit| Self {
                toolkit,
                layout: Layout::default(),
            })
            .map_err(unavailable)
    }

    /// Lays scores out as `layout` describes, from the next one onward.
    pub const fn lay_out(&mut self, layout: Layout) {
        self.layout = layout;
    }

    /// Opens Verovio from `HOGWASH_VEROVIO_LIB` and `HOGWASH_VEROVIO_DATA`.
    ///
    /// A development convenience. Where an installed application looks for
    /// these files is a packaging question, not an architectural one, and it
    /// is answered when there is something to package.
    ///
    /// # Errors
    ///
    /// As [`open`](Self::open), and also if either variable is unset.
    pub fn from_environment() -> Result<Self, EngravingFailure> {
        let named = |variable: &str| {
            std::env::var(variable).map(PathBuf::from).map_err(|_| {
                EngravingFailure::EngineUnavailable {
                    detail: format!("{variable} is not set"),
                }
            })
        };

        Self::open(
            &named("HOGWASH_VEROVIO_LIB")?,
            &named("HOGWASH_VEROVIO_DATA")?,
        )
    }
}

impl Engraver for VerovioEngraver {
    fn engrave(&mut self, source: &[u8]) -> Result<EngravedScore, EngravingFailure> {
        // A `.mxl` is a zip container. Its first two bytes are the one thing
        // every zip file agrees on, and Verovio unpacks the rest — which is
        // why FR-100's "compressed and uncompressed" costs two lines here
        // rather than a zip dependency and a guess at which entry is the
        // score.
        //
        // This is a *format*, not a *validation*: deciding whether a file is
        // acceptable at all belongs to Acquisition & Validation (`004 §8.2`),
        // which arrives at M6.
        // Before loading, not after: these options change how the music is
        // laid out, and Verovio lays it out while parsing.
        self.toolkit
            .set_options(&self.layout.as_options())
            .map_err(refused)?;

        if source.starts_with(b"PK") {
            self.toolkit.load_compressed(source).map_err(refused)?;
        } else {
            let text =
                std::str::from_utf8(source).map_err(|failure| EngravingFailure::Unreadable {
                    detail: format!("the file is not text: {failure}"),
                })?;

            self.toolkit.load(text).map_err(refused)?;
        }

        // Before any of the reading below. `S0` Q2a found that without it
        // every timing query returns an empty object while merely *logging* a
        // warning — which would look like a score where nothing sounds, and
        // would produce a Score with no positions at all.
        self.toolkit.build_timemap().map_err(refused)?;

        let document = self.toolkit.mei().map_err(refused)?;
        let times = self.toolkit.timemap().map_err(refused)?;

        // Verovio numbers pages from one.
        let pages = (1..=self.toolkit.page_count())
            .map(|page| self.toolkit.page(page))
            .collect::<Result<Vec<_>, _>>()
            .map_err(refused)?;

        let read = mei::read(&document).map_err(|failure| unreadable(&failure))?;
        let onsets = timemap::onsets(&times).map_err(|failure| unreadable(&failure))?;
        let (score, details, placements) =
            translate::assemble(&read, &onsets, &pages).map_err(|failure| unreadable(&failure))?;

        Ok(EngravedScore::new(score, details, pages, placements)?)
    }
}

fn unavailable(error: ToolkitError) -> EngravingFailure {
    EngravingFailure::EngineUnavailable {
        detail: describe(error),
    }
}

fn refused(error: ToolkitError) -> EngravingFailure {
    match error {
        // The engine failing to answer at all is not the file's fault, and
        // telling a performer their score is broken when the library is
        // missing would send them to fix the wrong thing.
        unusable @ ToolkitError::Unavailable(_) => unavailable(unusable),
        other => EngravingFailure::Unreadable {
            detail: describe(other),
        },
    }
}

fn unreadable(failure: &TranslationFailure) -> EngravingFailure {
    EngravingFailure::Unreadable {
        detail: failure.to_string(),
    }
}

fn describe(error: ToolkitError) -> String {
    match error {
        ToolkitError::Unavailable(detail)
        | ToolkitError::NotText(detail)
        | ToolkitError::Refused(detail) => detail,
    }
}
