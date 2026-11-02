//! The [`Engraver`] Segno actually uses.

use std::path::{Path, PathBuf};

use segno_engraving::{ElementId, EngravedScore, Engraver, EngravingFailure};

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
/// Zoom reflows rather than magnifies: [`fitting`](Self::fitting) makes the
/// page as wide as the window at the chosen size, so a bigger zoom means
/// fewer bars per line, never sideways scrolling. `svgViewBox` lets the page
/// then be sized in the DOM with plain CSS.
///
/// [`EngravedScore`]: segno_engraving::EngravedScore
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// Page width in Verovio's units: one pixel each at 100%.
    pub width: u32,
    /// Staff size as a percentage; 100 is Verovio's natural size.
    pub scale: u32,
}

/// The zoom range, in percent.
const ZOOM: std::ops::RangeInclusive<u32> = 50..=200;
/// The narrowest page, in Verovio's units: enough for one bar with its clefs
/// and key signature. A narrower window shrinks the page instead.
const NARROWEST: u32 = 500;

impl Default for Layout {
    /// A typical window at natural size, for engraving before a window has
    /// been measured.
    fn default() -> Self {
        Self::fitting(1200, 100)
    }
}

impl Layout {
    /// A page `width_px` pixels wide on screen at `zoom` percent: Verovio's
    /// own recipe for a responsive layout (`pageWidth = width × 100 / zoom`,
    /// `scale = zoom`). Zoom is kept within 50-200%.
    #[must_use]
    pub fn fitting(width_px: u32, zoom: u32) -> Self {
        let scale = zoom.clamp(*ZOOM.start(), *ZOOM.end());
        Self {
            width: (width_px.saturating_mul(100) / scale).max(NARROWEST),
            scale,
        }
    }

    /// The options Verovio is given, as JSON.
    ///
    /// `header` and `footer` are turned off: Verovio prints a title and page
    /// numbers by default, which belong on paper and not on a practice
    /// surface where the application already says what the piece is.
    fn as_options(self) -> String {
        format!(
            r#"{{"pageWidth":{},"scale":{},"adjustPageHeight":true,"breaks":"auto","header":"none","footer":"none","svgViewBox":true}}"#,
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
    /// application rather than inside it, which is the design: it is what
    /// makes the library replaceable, and replaceability is how section 4 of LGPL-3.0
    /// is satisfied.
    ///
    /// # Errors
    ///
    /// [`EngravingFailure::EngineUnavailable`] if the library is missing, is
    /// not Verovio, or will not start with those resources. This is reported
    /// rather than fatal because a performer who has moved a file deserves to
    /// be told which one.
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
        // why the requirement's "compressed and uncompressed" costs two lines here
        // rather than a zip dependency and a guess at which entry is the
        // score.
        //
        // This is a *format*, not a *validation*: deciding whether a file is
        // acceptable at all belongs to Acquisition & Validation,
        // which arrives at M6.
        // A damaged file never reaches Verovio: a broken zip would abort the
        // whole application, and half a MusicXML file would read as a piece.
        if let Some(reason) = crate::intact::damage(source) {
            return Err(EngravingFailure::Unreadable {
                detail: reason.to_owned(),
            });
        }

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

        // Before any of the reading below. Without it,
        // every timing query returns an empty object while merely *logging* a
        // warning — which would look like a score where nothing sounds, and
        // would produce a Score with no positions at all.
        self.toolkit.build_timemap().map_err(refused)?;

        let document = self.toolkit.mei().map_err(refused)?;
        let times = self.toolkit.timemap().map_err(refused)?;
        // After the timemap: building it is what writes the repeats out.
        let copies = self.toolkit.expansion_map().map_err(refused)?;

        // Verovio numbers pages from one.
        let pages = (1..=self.toolkit.page_count())
            .map(|page| self.toolkit.page(page))
            .collect::<Result<Vec<_>, _>>()
            .map_err(refused)?;

        let read = mei::read(&document).map_err(|failure| unreadable(&failure))?;
        let onsets = timemap::onsets(&times).map_err(|failure| unreadable(&failure))?;
        let played = timemap::measures(&times).map_err(|failure| unreadable(&failure))?;
        let beats = timemap::beats(&times).map_err(|failure| unreadable(&failure))?;
        let originals = timemap::originals(&copies).map_err(|failure| unreadable(&failure))?;
        let (score, details, placements) =
            translate::assemble(&read, &onsets, &played, &beats, &originals, &pages)
                .map_err(|failure| unreadable(&failure))?;

        let bars = read
            .measure_elements()
            .map(|(id, number)| (ElementId::new(id.to_owned()), number))
            .collect();

        Ok(EngravedScore::new(score, details, pages, placements)?.with_bars(bars))
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

#[cfg(test)]
mod tests {
    use super::Layout;

    #[test]
    fn a_fitted_page_is_the_window_width_at_the_zoom() {
        assert_eq!(
            Layout::fitting(1200, 100),
            Layout {
                width: 1200,
                scale: 100
            }
        );
        assert_eq!(
            Layout::fitting(1200, 200),
            Layout {
                width: 600,
                scale: 200
            }
        );
        assert_eq!(
            Layout::fitting(1200, 50),
            Layout {
                width: 2400,
                scale: 50
            }
        );
    }

    #[test]
    fn zoom_stays_in_range() {
        assert_eq!(Layout::fitting(1200, 400).scale, 200);
        assert_eq!(Layout::fitting(1200, 10).scale, 50);
    }

    #[test]
    fn a_page_is_never_narrower_than_one_bar() {
        assert_eq!(Layout::fitting(300, 200).width, 500);
    }
}
