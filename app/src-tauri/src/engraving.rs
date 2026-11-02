//! The engraver, given a thread of its own.
//!
//! # Why this exists
//!
//! `VerovioEngraver` holds a raw pointer to a C toolkit, so it is neither
//! `Send` nor `Sync` — deliberately. `segno-engraving-verovio` says as much:
//! nothing establishes that Verovio's toolkit may be used from more than one
//! thread, and claiming otherwise would be exactly the kind of unfounded
//! `unsafe impl` that makes a raw pointer dangerous.
//!
//! Tauri's shared state must be `Send + Sync`, because commands run on a pool
//! of threads. Those two facts collide, and there are only two honest ways
//! out: keep the engraver out of shared state entirely, or **give it one
//! thread and talk to it**.
//!
//! This is the second. The toolkit is created on a thread and never leaves
//! it; every request crosses a channel and every reply crosses back. The
//! `!Send` property is not worked around, it is enforced by construction —
//! the engraver is unreachable from anywhere else.
//!
//! It is the same shape as the design: one owner, and a channel to reach it.
//! That decision was about MIDI arriving on someone else's thread; this is
//! about a toolkit that must stay on its own. The answer being the same twice
//! is a point in its favour.

use std::sync::mpsc::{Sender, SyncSender, channel, sync_channel};
use std::thread;

use segno_engraving::{EngravedScore, Engraver, EngravingFailure};
use segno_engraving_verovio::Layout;

use crate::startup::{Startup, Unavailable};

/// One thing to engrave, and where to send the result.
struct Request {
    source: Vec<u8>,
    layout: Layout,
    reply: SyncSender<Result<EngravedScore, EngravingFailure>>,
}

/// A handle to the engraver's thread.
///
/// Cloneable and `Send`, unlike the engraver itself. Dropping the last one
/// closes the channel, which ends the thread.
#[derive(Debug, Clone)]
pub struct Engraving {
    requests: Sender<Request>,
}

impl Engraving {
    /// Starts the engraver on its own thread.
    ///
    /// Blocks until the engine has been found and opened, so that a failure
    /// to start is reported here rather than at the first score — a performer
    /// should be told the engraving library is missing when they open the
    /// application, not when they open a piece.
    ///
    /// # Errors
    ///
    /// [`Unavailable`], describing what was looked for and where.
    pub fn start() -> Result<Self, Unavailable> {
        Self::start_in(None)
    }

    /// [`Self::start`], also looking in the package's resource folder.
    ///
    /// # Errors
    ///
    /// As [`Self::start`].
    pub fn start_in(resources: Option<std::path::PathBuf>) -> Result<Self, Unavailable> {
        let (requests, incoming) = channel::<Request>();
        let (started, ready) = sync_channel::<Result<(), Unavailable>>(0);

        thread::Builder::new()
            .name("segno-engraver".to_owned())
            .spawn(move || {
                // Created *on this thread*, and never moved off it. This is
                // the line the whole module exists to make true.
                let mut engraver = match Startup::engine(resources.as_deref()) {
                    Ok(engine) => {
                        drop(started.send(Ok(())));
                        engine
                    }
                    Err(unavailable) => {
                        drop(started.send(Err(unavailable)));
                        return;
                    }
                };

                // Ends when the last handle is dropped and the channel
                // closes, which is the application shutting down.
                for request in incoming {
                    engraver.lay_out(request.layout);
                    let outcome = engraver.engrave(&request.source);

                    // A failed reply means the caller gave up waiting. That
                    // is their business; this thread carries on serving the
                    // next request.
                    drop(request.reply.send(outcome));
                }
            })
            .map_err(|failure| Unavailable {
                summary: "Hogwash could not start its engraving thread.".to_owned(),
                detail: failure.to_string(),
                looked_in: Vec::new(),
            })?;

        match ready.recv() {
            Ok(Ok(())) => Ok(Self { requests }),
            Ok(Err(unavailable)) => Err(unavailable),
            Err(_) => Err(Unavailable {
                summary: "Hogwash's engraving thread stopped before it started.".to_owned(),
                detail: "The thread ended without reporting why, which is a defect in \
                         Hogwash rather than a problem with your music."
                    .to_owned(),
                looked_in: Vec::new(),
            }),
        }
    }

    /// Engraves `source` at `layout`, waiting for the answer.
    ///
    /// # Errors
    ///
    /// Whatever the engraver said, or [`EngravingFailure::EngineUnavailable`]
    /// if the thread has gone.
    pub fn engrave(
        &self,
        source: &[u8],
        layout: Layout,
    ) -> Result<EngravedScore, EngravingFailure> {
        // A rendezvous channel: this call blocks until the engraver answers,
        // so there is no queue of stale replies to reason about.
        let (reply, answer) = sync_channel(0);

        let gone = || EngravingFailure::EngineUnavailable {
            detail: "the engraving thread is no longer running".to_owned(),
        };

        self.requests
            .send(Request {
                source: source.to_vec(),
                layout,
                reply,
            })
            .map_err(|_| gone())?;

        answer.recv().map_err(|_| gone())?
    }
}
