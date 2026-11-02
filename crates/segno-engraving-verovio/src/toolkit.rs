//! The C boundary. **The only `unsafe` in the workspace.**
//!
//! Verovio exposes 60 plain `extern "C"` functions over an opaque handle
//! (`tools/c_wrapper.h`). No C++ ABI crosses the boundary, so no `bindgen`
//! and no `cxx` — the integration really is a declaration list, as the
//! Verovio spike measured. Ten of the sixty are used.
//!
//! # Why the library is opened at run time
//!
//! `libloading` rather than build-time linking means:
//!
//! - `cargo build` and the whole test suite work with no C++ toolchain and no
//!   Verovio present, so contributors and CI pay nothing for it.
//! - section 4 of LGPL-3.0's replaceable-library obligation is discharged by
//!   construction rather than argued in a paragraph.
//! - A missing library is a message rather than a link error, which is what
//!   The design requires of every failure.
//!
//! # What makes the `unsafe` here sound
//!
//! Three facts, and every block below leans on one of them:
//!
//! 1. **The symbols have the signatures declared.** They are transcribed from
//!    `tools/c_wrapper.h` at the pinned commit, and [`Toolkit::open`] resolves
//!    every one up front, so a mismatched or missing library fails at open
//!    rather than at an arbitrary later call.
//! 2. **The handle is used only by the toolkit that made it.** It is created
//!    in `open`, private, and destroyed in `Drop`.
//! 3. **Returned strings belong to Verovio and are copied immediately.** The
//!    toolkit owns that memory and reuses the buffer on the next call, so
//!    nothing borrowed from it may outlive the call that produced it. Every
//!    accessor here returns an owned `String`.
//!
//! [`Toolkit`] is deliberately **not** `Send` or `Sync` — it holds a raw
//! pointer, and nothing here establishes that Verovio's toolkit is safe to
//! use from more than one thread. The design keeps everything on one thread
//! anyway.

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::Path;

use libloading::{Library, Symbol};

/// What can go wrong at the boundary itself.
#[derive(Debug)]
pub(crate) enum ToolkitError {
    /// The shared library could not be opened, or lacks an expected symbol.
    Unavailable(String),
    /// A string Verovio returned was not valid UTF-8, or a string we were
    /// asked to pass in contained a NUL byte.
    NotText(String),
    /// Verovio declined the input.
    Refused(String),
}

type Constructor = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type Destructor = unsafe extern "C" fn(*mut c_void);
type LoadData = unsafe extern "C" fn(*mut c_void, *const c_char) -> bool;
type LoadZipBuffer = unsafe extern "C" fn(*mut c_void, *const u8, c_int) -> bool;
type ResetSeed = unsafe extern "C" fn(*mut c_void, c_int);
type SetOptions = unsafe extern "C" fn(*mut c_void, *const c_char) -> bool;
type GetString = unsafe extern "C" fn(*mut c_void) -> *const c_char;
type GetStringWithOptions = unsafe extern "C" fn(*mut c_void, *const c_char) -> *const c_char;
type GetPageCount = unsafe extern "C" fn(*mut c_void) -> c_int;
type RenderToSvg = unsafe extern "C" fn(*mut c_void, c_int, bool) -> *const c_char;

/// Every Verovio function this crate uses, resolved once.
///
/// Resolving up front is what makes fact 1 above hold: a library that is the
/// wrong thing entirely fails at [`Toolkit::open`], where the message can say
/// so, rather than at whichever call first happens to need the missing
/// symbol.
struct Functions {
    destructor: Destructor,
    load_data: LoadData,
    load_zip_buffer: LoadZipBuffer,
    reset_xml_id_seed: ResetSeed,
    set_options: SetOptions,
    get_mei: GetStringWithOptions,
    get_log: GetString,
    get_page_count: GetPageCount,
    render_to_svg: RenderToSvg,
    render_to_timemap: GetStringWithOptions,
    render_to_expansion_map: GetString,
}

/// A live Verovio toolkit.
pub(crate) struct Toolkit {
    // Declared before `library`: fields drop in declaration order, so the
    // handle is destroyed while the code that destroys it is still mapped.
    // Reversing these two lines would call a function in an unloaded library.
    handle: Handle,
    functions: Functions,
    #[expect(
        dead_code,
        reason = "held so the library outlives every symbol resolved from it"
    )]
    library: Library,
}

/// The opaque toolkit pointer, wrapped so it is never bare in a struct field.
struct Handle(*mut c_void);

impl std::fmt::Debug for Toolkit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Toolkit").finish_non_exhaustive()
    }
}

impl Toolkit {
    /// Opens `library` and creates a toolkit that finds its fonts in
    /// `resources`.
    ///
    /// `resources` is Verovio's `data/` directory. It is not
    /// compiled in: without it the engine loads but draws nothing legible.
    pub(crate) fn open(library: &Path, resources: &Path) -> Result<Self, ToolkitError> {
        // SAFETY (fact 1): opening a shared library runs its initialisers,
        // which is unsound only if the file is not a library we expect.
        // The path comes from configuration, not from a score file, and a
        // wrong one fails here rather than corrupting anything.
        #[expect(
            unsafe_code,
            reason = "dlopen has no safe equivalent; the path is configuration, not input"
        )]
        let handle = unsafe { Library::new(library) }.map_err(|failure| {
            ToolkitError::Unavailable(format!("{}: {failure}", library.display()))
        })?;

        // SAFETY (fact 1): each signature is transcribed from
        // tools/c_wrapper.h. Resolving all of them here means a library with
        // a different ABI is rejected before any of it is called.
        #[expect(
            unsafe_code,
            reason = "symbol signatures are transcribed from tools/c_wrapper.h and resolved up front"
        )]
        let (functions, constructor) = unsafe {
            let constructor: Symbol<'_, Constructor> =
                resolve(&handle, b"vrvToolkit_constructorResourcePath")?;

            let functions = Functions {
                destructor: *resolve::<Destructor>(&handle, b"vrvToolkit_destructor")?,
                load_data: *resolve::<LoadData>(&handle, b"vrvToolkit_loadData")?,
                load_zip_buffer: *resolve::<LoadZipBuffer>(
                    &handle,
                    b"vrvToolkit_loadZipDataBuffer",
                )?,
                reset_xml_id_seed: *resolve::<ResetSeed>(&handle, b"vrvToolkit_resetXmlIdSeed")?,
                set_options: *resolve::<SetOptions>(&handle, b"vrvToolkit_setOptions")?,
                get_mei: *resolve::<GetStringWithOptions>(&handle, b"vrvToolkit_getMEI")?,
                get_log: *resolve::<GetString>(&handle, b"vrvToolkit_getLog")?,
                get_page_count: *resolve::<GetPageCount>(&handle, b"vrvToolkit_getPageCount")?,
                render_to_svg: *resolve::<RenderToSvg>(&handle, b"vrvToolkit_renderToSVG")?,
                render_to_timemap: *resolve::<GetStringWithOptions>(
                    &handle,
                    b"vrvToolkit_renderToTimemap",
                )?,
                render_to_expansion_map: *resolve::<GetString>(
                    &handle,
                    b"vrvToolkit_renderToExpansionMap",
                )?,
            };

            (functions, *constructor)
        };

        let path = text_in(resources.to_string_lossy().as_ref())?;

        // SAFETY (facts 1 and 3): `constructor` has the declared signature,
        // and `path` is a NUL-terminated C string that outlives the call.
        #[expect(
            unsafe_code,
            reason = "calling a resolved C function with a live NUL-terminated string"
        )]
        let pointer = unsafe { constructor(path.as_ptr()) };

        if pointer.is_null() {
            return Err(ToolkitError::Unavailable(format!(
                "verovio would not start with resources at {}",
                resources.display()
            )));
        }

        Ok(Self {
            handle: Handle(pointer),
            functions,
            library: handle,
        })
    }

    /// Loads uncompressed `source` — `MusicXML` or MEI.
    ///
    /// Seeds the identifier generator **first**, and that ordering is not
    /// stylistic. Measured: unseeded identifiers differ between two loads in
    /// one process, and `resetXmlIdSeed` after `loadData` is too late because
    /// identifiers are assigned during parsing. The requirement is identical
    /// results across restarts, and the design makes that release-blocking, so an unseeded load would fail the
    /// release criteria on the first restart.
    pub(crate) fn load(&mut self, source: &str) -> Result<(), ToolkitError> {
        self.seed();

        let data = text_in(source)?;

        // SAFETY (facts 1, 2 and 3): `data` outlives the call.
        #[expect(unsafe_code, reason = "resolved signature, own handle, live string")]
        let loaded = unsafe { (self.functions.load_data)(self.handle.0, data.as_ptr()) };

        if loaded {
            Ok(())
        } else {
            Err(ToolkitError::Refused(self.log()))
        }
    }

    /// Loads compressed `MusicXML` — a `.mxl` container.
    ///
    /// The requirement requires `MusicXML` *"in both its uncompressed and compressed
    /// forms"*. Verovio unpacks the container itself, so this exists rather
    /// than a zip dependency and a guess at which entry inside is the score.
    pub(crate) fn load_compressed(&mut self, source: &[u8]) -> Result<(), ToolkitError> {
        self.seed();

        let length = c_int::try_from(source.len()).map_err(|_| {
            ToolkitError::Refused("the file is too large for the engraver".to_owned())
        })?;

        // SAFETY (facts 1, 2 and 3): `source` is a live slice of `length`
        // bytes, and Verovio only reads it during the call.
        #[expect(unsafe_code, reason = "resolved signature, own handle, live buffer")]
        let loaded =
            unsafe { (self.functions.load_zip_buffer)(self.handle.0, source.as_ptr(), length) };

        if loaded {
            Ok(())
        } else {
            Err(ToolkitError::Refused(self.log()))
        }
    }

    /// Applies Verovio options, given as JSON.
    ///
    /// Must be called **before** loading: the options that matter here change
    /// how the music is laid out, and layout happens during parsing.
    pub(crate) fn set_options(&mut self, options: &str) -> Result<(), ToolkitError> {
        let json = text_in(options)?;

        // SAFETY (facts 1, 2 and 3): `json` outlives the call.
        #[expect(unsafe_code, reason = "resolved signature, own handle, live string")]
        let accepted = unsafe { (self.functions.set_options)(self.handle.0, json.as_ptr()) };

        if accepted {
            Ok(())
        } else {
            Err(ToolkitError::Refused(self.log()))
        }
    }

    /// Fixes the identifier generator before parsing. See [`Self::load`].
    fn seed(&mut self) {
        // SAFETY (facts 1 and 2): resolved signature, own handle.
        #[expect(unsafe_code, reason = "seeds before parsing assigns identifiers")]
        unsafe {
            (self.functions.reset_xml_id_seed)(self.handle.0, ID_SEED);
        }
    }

    /// The loaded music as MEI, which is where the written spellings are.
    ///
    /// `getElementAttr` does **not** report accidentals,
    /// because MEI models an accidental as a child element rather than an
    /// attribute of the note. So the spelling a performer reads is only
    /// available from the document itself, and this is how it is obtained.
    pub(crate) fn mei(&mut self) -> Result<String, ToolkitError> {
        self.string_with_options(self.functions.get_mei, r#"{"noLayout": true}"#)
    }

    /// Builds the MIDI timemap.
    ///
    /// Called for its side effect. Without it every MIDI
    /// query returns `{}` while logging a warning — a failure quiet enough to
    /// look like "this note has no pitch".
    pub(crate) fn build_timemap(&mut self) -> Result<(), ToolkitError> {
        self.timemap().map(|_| ())
    }

    /// The timemap as JSON: which notes sound, and when, and which bar
    /// begins at each moment (`includeMeasures`) — the order bars are played
    /// in, repeats included.
    pub(crate) fn timemap(&mut self) -> Result<String, ToolkitError> {
        self.string_with_options(
            self.functions.render_to_timemap,
            r#"{"includeMeasures": true}"#,
        )
    }

    /// Which identifiers are copies of which, as JSON.
    ///
    /// The timemap is built from a copy of the document with repeats written
    /// out, and every repeated element there gets a new identifier
    /// (`<id>-rend2`) that the MEI does not contain. This map is how a note in
    /// a second pass is traced back to the note that was drawn. Call after
    /// [`Self::timemap`], which is what creates the expanded copy.
    pub(crate) fn expansion_map(&mut self) -> Result<String, ToolkitError> {
        // SAFETY (facts 1, 2 and 3).
        #[expect(
            unsafe_code,
            reason = "resolved signature, own handle, string copied at once"
        )]
        let returned = unsafe { (self.functions.render_to_expansion_map)(self.handle.0) };

        text_out(returned)
    }

    /// How many pages the current layout produced.
    pub(crate) fn page_count(&self) -> usize {
        // SAFETY (facts 1 and 2).
        #[expect(unsafe_code, reason = "resolved signature, own handle")]
        let count = unsafe { (self.functions.get_page_count)(self.handle.0) };

        usize::try_from(count).unwrap_or(0)
    }

    /// One rendered page as SVG. Pages are numbered from one by Verovio.
    pub(crate) fn page(&mut self, page: usize) -> Result<String, ToolkitError> {
        let number = c_int::try_from(page)
            .map_err(|_| ToolkitError::Refused(format!("page {page} is out of range")))?;

        // SAFETY (facts 1, 2 and 3): the returned pointer is Verovio's and is
        // copied before this function returns.
        #[expect(
            unsafe_code,
            reason = "resolved signature, own handle, string copied at once"
        )]
        let rendered = unsafe { (self.functions.render_to_svg)(self.handle.0, number, false) };

        text_out(rendered)
    }

    /// Whatever Verovio has said since the last call.
    ///
    /// Verovio reports most trouble by logging rather than by returning
    /// false, so this is the raw material for the Validation Report that
    /// The design requires.
    pub(crate) fn log(&self) -> String {
        // SAFETY (facts 1, 2 and 3).
        #[expect(
            unsafe_code,
            reason = "resolved signature, own handle, string copied at once"
        )]
        let logged = unsafe { (self.functions.get_log)(self.handle.0) };

        text_out(logged).unwrap_or_else(|_| String::new())
    }

    fn string_with_options(
        &self,
        function: GetStringWithOptions,
        options: &str,
    ) -> Result<String, ToolkitError> {
        let json = text_in(options)?;

        // SAFETY (facts 1, 2 and 3): `json` outlives the call, and the
        // returned pointer is copied before this function returns.
        #[expect(
            unsafe_code,
            reason = "resolved signature, own handle, string copied at once"
        )]
        let returned = unsafe { function(self.handle.0, json.as_ptr()) };

        text_out(returned)
    }
}

impl Drop for Toolkit {
    fn drop(&mut self) {
        // SAFETY (fact 2): this handle was made by `open`, has not been
        // destroyed, and cannot be — nothing else can reach it, and `Drop`
        // runs once. The library is still mapped because `library` is
        // declared after `handle` and so drops after it.
        #[expect(unsafe_code, reason = "destroying a handle this type exclusively owns")]
        unsafe {
            (self.functions.destructor)(self.handle.0);
        }
    }
}

/// The seed handed to Verovio before every load.
///
/// Any fixed value would do; what matters is that it is fixed.
const ID_SEED: c_int = 1;

/// Resolves one symbol, turning a missing one into a stated failure.
///
/// # Safety
///
/// The caller states that `T` is the symbol's real signature. Every call site
/// here transcribes it from `tools/c_wrapper.h`.
#[expect(
    unsafe_code,
    reason = "declaring the obligation this helper passes to its callers"
)]
unsafe fn resolve<'library, T>(
    library: &'library Library,
    name: &[u8],
) -> Result<Symbol<'library, T>, ToolkitError> {
    // SAFETY: delegated to the caller by this function's own contract.
    #[expect(unsafe_code, reason = "obligation discharged by the caller")]
    unsafe { library.get(name) }.map_err(|failure| {
        ToolkitError::Unavailable(format!(
            "{} is not the verovio library: {failure}",
            String::from_utf8_lossy(name)
        ))
    })
}

/// A Rust string as a C string.
fn text_in(text: &str) -> Result<CString, ToolkitError> {
    CString::new(text).map_err(|_| {
        ToolkitError::NotText("the text contains a NUL byte, which C cannot carry".to_owned())
    })
}

/// A C string as an owned Rust string.
///
/// **Copies immediately** (fact 3). Verovio owns the buffer and reuses it on
/// the next call, so borrowing from it would be a use-after-free the moment
/// anything else was asked of the toolkit.
fn text_out(text: *const c_char) -> Result<String, ToolkitError> {
    if text.is_null() {
        return Err(ToolkitError::NotText("verovio returned nothing".to_owned()));
    }

    // SAFETY (fact 3): non-null, and Verovio NUL-terminates everything it
    // returns from these accessors. The borrow ends inside this function.
    #[expect(
        unsafe_code,
        reason = "reading a NUL-terminated string Verovio just returned"
    )]
    let borrowed = unsafe { CStr::from_ptr(text) };

    borrowed
        .to_str()
        .map(str::to_owned)
        .map_err(|failure| ToolkitError::NotText(format!("verovio returned non-UTF-8: {failure}")))
}
