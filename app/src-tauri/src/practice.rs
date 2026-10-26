//! The practice loop, given a thread of its own.
//!
//! # Why this exists
//!
//! The design wants live per-note correctness on
//! screen, but the design is explicit that the Cursor renders no verdict —
//! `CursorMovement` has exactly two values, `Advanced` and `Held`, and `Held`
//! means "wrong note" *or* "chord not finished yet" indistinguishably. So the
//! live correctness signal cannot come from `CursorMovement`; it is computed
//! here, in Rust, the same way `Analysis::of` computes it — by checking
//! whether the played pitch was a member of `Cursor::expecting()`'s sounding
//! set *before* the Cursor is given the note. Same rule, same source, so the
//! marks on screen and the summary afterwards agree by construction
//! rather than by coincidence. The frontend still judges
//! nothing; this boolean is the only thing that crosses the boundary.
//!
//! # One channel, one owner — a second time
//!
//! `engraving.rs` already gave one thing a dedicated thread because it could
//! not safely be shared: a raw C pointer, there. Here it is a `Keyboard`,
//! an `Attempt` and a `PracticeSession` moving together through a lifecycle
//! that a Tauri command pool — several threads, running concurrently — has
//! no natural place to hold. Giving them one thread and a channel is
//! The design's "one channel, one owner", relocated from a frame loop (which
//! a webview does not have) to a loop this module supplies instead.
//!
//! # Session scope
//!
//! One `PracticeSession` per piece opened this run, many `Attempt`s inside
//! it: the thread only opens a new session when asked to start an attempt on
//! a different piece than whatever is currently open. `PracticeSession`'s
//! `close`/`ClosedSession` are legitimately unused here — nothing in this
//! scope needs the "never accepts another attempt" guarantee.
//!
//! # Why this module knows nothing of Tauri
//!
//! Events leave through a plain callback (`Sink`) rather than an
//! `AppHandle`. `lib.rs` is where the callback is built, and it is the one
//! line that knows events are emitted through Tauri at all — which is also
//! what lets a test drive a whole practice session with no window, and no
//! event loop, by handing this module a callback that only counts what it
//! was given.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, SyncSender, channel, sync_channel};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use hogwash_analysis::{Analysis, Trace};
use hogwash_capture::{Action, Instrument, Keyboard, MidiSource, NoInstrument, Touch};
use hogwash_domain::{MeasureNumber, PieceId};
use hogwash_feedback::Feedback;
use hogwash_midi::{MidiInstruments, NoOutput, OutputPort, Thru, output_ports};
use hogwash_notation::ReferencePerformance;
use hogwash_practice::{Attempt, CursorMovement, Loop, Pass, PracticeSession};
use hogwash_records::{AttemptRecord, Ended, Records, SessionRecord};
use serde::Serialize;

use crate::review::{self, LoopScope, Review, Run, Segment, Suggested};
use crate::startup::Unavailable;

/// Where every [`PracticeEvent`] this loop produces goes.
type Sink = Box<dyn Fn(&PracticeEvent) + Send>;

/// How often the loop drains the keyboard while an attempt is running.
///
/// Comfortably inside a 100 ms budget.
const CONNECTED_POLL: Duration = Duration::from_millis(25);

/// How long the loop waits when nothing is running — no keyboard to drain.
const IDLE_POLL: Duration = Duration::from_millis(500);

/// How often a lost instrument is retried while waiting for it to come back.
const RECONNECT_ATTEMPT_EVERY: Duration = Duration::from_secs(2);

/// How long an attempt waits for a lost instrument before ending itself.
///
/// The hybrid: neither "auto-end immediately" nor "wait forever" — the loss
/// is reported at once (the design's *visibly*), and the attempt only ends on
/// its own if the instrument does not come back within this window.
const RECONNECT_WINDOW: Duration = Duration::from_mins(1);

/// One update the practice loop reports to the interface.
///
/// A single tagged enum, so the frontend's listener is one `match` — the
/// same shape as `App.tsx`'s `Screen` union.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PracticeEvent {
    /// A note was played at the position named. `correct` is the boolean
    /// this module exists to compute — never derived from `CursorMovement`.
    ///
    /// `pitch` is the bare chromatic (MIDI) number a keyboard reports,
    /// nothing more — the design never attaches a spelling to a played
    /// note, and this event does not either. It exists only so a wrong
    /// note can be shown *somewhere* sensible; any spelling the interface
    /// invents for that is its own display concern, not this event's.
    Played {
        position: usize,
        correct: bool,
        pitch: u8,
    },
    /// The Cursor moved on to `position`.
    Advanced { position: usize },
    /// The Cursor has nothing left to expect.
    ReachedEnd,
    /// The instrument stopped reporting. `seconds_remaining` counts down to
    /// the attempt ending itself.
    InstrumentLost {
        instrument: String,
        // `rename_all` on the enum renames only the variants, not their fields.
        #[serde(rename = "secondsRemaining")]
        seconds_remaining: u64,
    },
    /// The same instrument came back before the window ran out.
    InstrumentRestored,
    /// The attempt ended, however it ended.
    Ended { summary: Box<AttemptSummary> },
    /// A wrong or unexpected note spoiled this pass of the loop: the count
    /// of clean passes in a row is back to zero.
    LoopCount {
        #[serde(rename = "cleanInARow")]
        clean_in_a_row: u32,
    },
    /// A pass of the loop reached its end, and the next began at `position`.
    Looped {
        position: usize,
        #[serde(rename = "cleanInARow")]
        clean_in_a_row: u32,
    },
    /// The loop reached its target and is over. The cursor waits at its
    /// start, `position`.
    LoopDone {
        position: usize,
        passes: u32,
        first: u32,
        last: u32,
        /// What to practise now: the loop a bar wider each side.
        widen: Option<Suggested>,
    },
}

/// What an ended attempt is reported as.
///
/// The design leaves the exact content of an attempt summary as an open
/// design question beyond "one overall result"; this is Stage D's
/// minimum to make the loop demonstrable end-to-end. Stage E designs the
/// screen this deserves.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttemptSummary {
    /// `Feedback::lines()`, verbatim — every sentence about the performance
    /// arrives from Rust already written.
    pub lines: Vec<String>,
    pub correct: usize,
    pub incorrect: usize,
    pub omitted: usize,
    pub unexpected: usize,
    pub reached_the_end: bool,
    /// Whether this ending was chosen by the performer or forced by the
    /// instrument never coming back.
    pub auto_ended: bool,
    pub reason: Option<String>,
    /// What the screen draws once playing stops.
    pub review: Review,
}

/// Asked to end an attempt when none is running.
#[derive(Debug)]
pub struct NotPracticing;

/// Where the next attempt starts, as a step and the bar it is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartPoint {
    /// A step in the Reference Performance, in played order.
    pub position: usize,
    /// The bar that step is in: the one clicked, or the next with something
    /// to strike if the clicked one has nothing.
    pub measure: u32,
}

/// A loop over bars `first` to `last`, as the interface is told of it.
/// Attempts start at `position` while it lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopRange {
    pub position: usize,
    pub first: u32,
    pub last: u32,
}

/// A loop asked for: bars `first` to `last` (either order), over until
/// `target` clean passes in a row, or never.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoopOf {
    pub first: MeasureNumber,
    pub last: MeasureNumber,
    pub target: Option<u32>,
}

/// Why the start could not be moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoStart {
    /// The bar does not exist, or nothing follows it to start at.
    NothingThere,
    /// The keyboard has gone and an attempt is waiting for it.
    InstrumentLost,
}

/// How the interface ends an attempt. The others (restart, jump, lost) are
/// decided here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Ending {
    /// Stop.
    Stop,
    /// Finish, at the end of the piece.
    Finished,
    /// Leaving the piece: back to the library, Esc, another piece. The
    /// start point goes back to the beginning.
    Left,
}

enum Command {
    Instruments {
        reply: SyncSender<Result<Vec<Instrument>, NoInstrument>>,
    },
    Start {
        piece_id: PieceId,
        reference: Arc<ReferencePerformance>,
        instrument: Instrument,
        reply: SyncSender<Result<(), NoInstrument>>,
    },
    End {
        ending: Ending,
        reply: SyncSender<Result<AttemptSummary, NotPracticing>>,
    },
    StartAt {
        piece_id: PieceId,
        reference: Arc<ReferencePerformance>,
        measure: MeasureNumber,
        reply: SyncSender<Result<StartPoint, NoStart>>,
    },
    LoopBars {
        piece_id: PieceId,
        reference: Arc<ReferencePerformance>,
        wanted: LoopOf,
        reply: SyncSender<Result<LoopRange, NoStart>>,
    },
    ClearLoop,
    LoopTarget {
        target: Option<u32>,
    },
    OutputPorts {
        reply: SyncSender<Result<Vec<OutputPort>, NoOutput>>,
    },
    /// Sets or clears the MIDI Thru route, independent of whatever `Attempt`
    /// may or may not be running — a performer can route sound to a real
    /// synth without starting a practice session at all.
    SetThru {
        input: Option<Instrument>,
        output: Option<OutputPort>,
        reply: SyncSender<Result<(), NoOutput>>,
    },
}

/// A handle to the practice thread.
#[derive(Debug, Clone)]
pub struct Practice {
    commands: Sender<Command>,
}

impl Practice {
    /// Starts the practice loop on its own thread.
    ///
    /// No keyboard being connected yet is the ordinary starting state
    /// (`NoInstrument::NoneConnected`), not a reason to refuse to open the
    /// window — the only failure here is the thread itself not starting.
    ///
    /// # Errors
    ///
    /// [`Unavailable`] if the operating system refused to create the
    /// thread.
    pub fn start(
        records: Records,
        sink: impl Fn(&PracticeEvent) + Send + 'static,
    ) -> Result<Self, Unavailable> {
        let (commands, incoming) = channel::<Command>();
        let sink: Sink = Box::new(sink);

        thread::Builder::new()
            .name("hogwash-practice".to_owned())
            .spawn(move || run(&incoming, &sink, &records))
            .map_err(|failure| Unavailable {
                summary: "Hogwash could not start its practice thread.".to_owned(),
                detail: failure.to_string(),
                looked_in: Vec::new(),
            })?;

        Ok(Self { commands })
    }

    /// The MIDI instruments currently available.
    ///
    /// # Errors
    ///
    /// [`NoInstrument::NoneConnected`] if there are none, or if the practice
    /// thread has gone.
    pub fn instruments(&self) -> Result<Vec<Instrument>, NoInstrument> {
        let (reply, answer) = sync_channel(0);

        if self.commands.send(Command::Instruments { reply }).is_err() {
            return Err(NoInstrument::NoneConnected);
        }

        answer.recv().unwrap_or(Err(NoInstrument::NoneConnected))
    }

    /// Opens `instrument` and begins an attempt at `piece_id`.
    ///
    /// # Errors
    ///
    /// [`NoInstrument`] if the instrument could not be opened.
    pub fn start_attempt(
        &self,
        piece_id: PieceId,
        reference: Arc<ReferencePerformance>,
        instrument: Instrument,
    ) -> Result<(), NoInstrument> {
        let (reply, answer) = sync_channel(0);
        let name = instrument.name().to_owned();

        let gone = move || NoInstrument::Unavailable {
            instrument: Instrument::new(name.clone()),
            detail: "the practice thread is no longer running".to_owned(),
        };

        self.commands
            .send(Command::Start {
                piece_id,
                reference,
                instrument,
                reply,
            })
            .map_err(|_| gone())?;

        answer.recv().map_err(|_| gone())?
    }

    /// Ends whatever attempt is running, as `ending` says. Leaving also
    /// puts the start point back at the beginning, running or not.
    ///
    /// # Errors
    ///
    /// [`NotPracticing`] if no attempt is running.
    pub fn end_attempt(&self, ending: Ending) -> Result<AttemptSummary, NotPracticing> {
        let (reply, answer) = sync_channel(0);

        self.commands
            .send(Command::End { ending, reply })
            .map_err(|_| NotPracticing)?;

        answer.recv().map_err(|_| NotPracticing)?
    }

    /// Moves where attempts at `piece_id` start to the beginning of
    /// `measure` (its first occurrence in played order). While an attempt
    /// is running, it is ended and kept, and a new one begins there on the
    /// same keyboard.
    ///
    /// # Errors
    ///
    /// [`NoStart`] if there is nothing to start at in that bar or after it,
    /// or the keyboard is lost.
    pub fn start_at_bar(
        &self,
        piece_id: PieceId,
        reference: Arc<ReferencePerformance>,
        measure: MeasureNumber,
    ) -> Result<StartPoint, NoStart> {
        let (reply, answer) = sync_channel(0);

        self.commands
            .send(Command::StartAt {
                piece_id,
                reference,
                measure,
                reply,
            })
            .map_err(|_| NoStart::NothingThere)?;

        answer.recv().map_err(|_| NoStart::NothingThere)?
    }

    /// Loops bars `wanted.first` to `wanted.last` of `piece_id`: attempts
    /// start at the loop's start, and each pass that reaches its end begins
    /// the next. While an attempt is running, it is ended and kept, and the
    /// first pass begins.
    ///
    /// # Errors
    ///
    /// [`NoStart`] if those bars are never played in a row or have nothing
    /// to strike, or the keyboard is lost.
    pub fn loop_bars(
        &self,
        piece_id: PieceId,
        reference: Arc<ReferencePerformance>,
        wanted: LoopOf,
    ) -> Result<LoopRange, NoStart> {
        let (reply, answer) = sync_channel(0);

        self.commands
            .send(Command::LoopBars {
                piece_id,
                reference,
                wanted,
                reply,
            })
            .map_err(|_| NoStart::NothingThere)?;

        answer.recv().map_err(|_| NoStart::NothingThere)?
    }

    /// Stops looping. The pass in progress carries on as an ordinary
    /// attempt, and the start point stays at the loop's start.
    pub fn clear_loop(&self) {
        drop(self.commands.send(Command::ClearLoop));
    }

    /// How many clean passes in a row end the loop from now on, or `None`
    /// for never.
    pub fn loop_target(&self, target: Option<u32>) {
        drop(self.commands.send(Command::LoopTarget { target }));
    }

    /// The MIDI output devices currently available, for routing Thru.
    ///
    /// # Errors
    ///
    /// [`NoOutput::NoneConnected`] if there are none, or if the practice
    /// thread has gone.
    pub fn output_ports(&self) -> Result<Vec<OutputPort>, NoOutput> {
        let (reply, answer) = sync_channel(0);

        if self.commands.send(Command::OutputPorts { reply }).is_err() {
            return Err(NoOutput::NoneConnected);
        }

        answer.recv().unwrap_or(Err(NoOutput::NoneConnected))
    }

    /// Sets or clears the MIDI Thru route.
    ///
    /// Independent of `start_attempt`/`end_attempt`: a performer can route
    /// sound to a real synth whether or not a practice attempt is running,
    /// because Thru forwards raw MIDI and has nothing to do with correctness.
    /// Passing `None` for either side clears whatever route was set.
    ///
    /// # Errors
    ///
    /// [`NoOutput`] if either device could not be opened.
    pub fn set_thru(
        &self,
        input: Option<Instrument>,
        output: Option<OutputPort>,
    ) -> Result<(), NoOutput> {
        let (reply, answer) = sync_channel(0);

        self.commands
            .send(Command::SetThru {
                input,
                output,
                reply,
            })
            .map_err(|_| NoOutput::Unavailable {
                detail: "the practice thread is no longer running".to_owned(),
            })?;

        answer.recv().map_err(|_| NoOutput::Unavailable {
            detail: "the practice thread is no longer running".to_owned(),
        })?
    }
}

/// One piece's practice sitting, and enough about it to keep a record.
struct OpenSession {
    piece_id: PieceId,
    id: String,
    started: String,
    session: PracticeSession,
    /// What the record says of each attempt kept in `session`, beyond what
    /// was played, index for index.
    kept: Vec<Kept>,
    /// The step the next attempt starts at.
    from: usize,
    /// The loop being practised, if any.
    looping: Option<Looping>,
    /// Everything played since Start, for the results.
    run: Run,
}

impl OpenSession {
    /// Starts the results afresh: a Start that is not a restart.
    fn begin_run(&mut self) {
        self.run = Run::default();
        if let Some(looping) = self.looping.as_mut() {
            looping.since = 0;
        }
    }
}

/// What the record says of an attempt, beyond what was played.
#[derive(Debug, Clone, Copy)]
struct Kept {
    /// When it began, in Unix milliseconds.
    began_ms: Option<u64>,
    ended: Ended,
    /// For a pass of a loop, the loop's end.
    to: Option<usize>,
}

/// A loop in progress, and the bars it was asked for by.
struct Looping {
    pass: Loop,
    first: u32,
    last: u32,
    /// The part of the run it began at.
    since: usize,
}

/// An attempt in progress, and its live keyboard connection.
struct Ongoing {
    instrument: Instrument,
    attempt: Attempt,
    /// When it began: the zero every played note's time counts from. Kept
    /// across a reconnect, so times run on rather than starting again.
    started: Instant,
    /// The same moment on the wall clock, for the record.
    began_ms: u64,
    /// For a pass of a loop, the loop's end.
    to: Option<usize>,
    keyboard: Box<dyn Keyboard>,
    /// `Some` once the instrument has stopped reporting; the moment it did.
    lost_since: Option<Instant>,
    last_reconnect_attempt: Instant,
}

fn run(incoming: &Receiver<Command>, sink: &Sink, records: &Records) {
    let mut source = MidiInstruments;
    let mut session: Option<OpenSession> = None;
    let mut ongoing: Option<Ongoing> = None;
    // Decoupled from `ongoing` on purpose: a performer can route MIDI Thru
    // to a real synth whether or not an Attempt is running.
    let mut thru: Option<Thru> = None;

    loop {
        let timeout = if ongoing.is_some() {
            CONNECTED_POLL
        } else {
            IDLE_POLL
        };

        match incoming.recv_timeout(timeout) {
            Ok(command) => handle(
                command,
                &mut source,
                &mut session,
                &mut ongoing,
                &mut thru,
                records,
                sink,
            ),
            Err(RecvTimeoutError::Timeout) => {}
            // The last `Practice` handle was dropped — the application is
            // shutting down. Nothing left to serve.
            Err(RecvTimeoutError::Disconnected) => return,
        }

        poll(&mut source, &mut session, &mut ongoing, records, sink);
    }
}

fn handle(
    command: Command,
    source: &mut MidiInstruments,
    session: &mut Option<OpenSession>,
    ongoing: &mut Option<Ongoing>,
    thru: &mut Option<Thru>,
    records: &Records,
    sink: &Sink,
) {
    match command {
        Command::Instruments { reply } => {
            drop(reply.send(source.instruments()));
        }
        Command::Start {
            piece_id,
            reference,
            instrument,
            reply,
        } => {
            // A restart (or a second piece opened) while one is running: the
            // one running is ended and kept first, never silently dropped. No
            // `Ended` event: the screen is starting again, not reading a summary.
            // A restart carries the run on; any other Start begins a new one.
            let restarting = ongoing.is_some();
            if let Some(previous) = ongoing.take() {
                let kept = previous.kept(Ended::Restart);
                drop(finish(
                    session,
                    previous.attempt,
                    kept,
                    records,
                    false,
                    None,
                ));
            }

            match source.open(&instrument) {
                Ok(keyboard) => {
                    open_session(session, piece_id, reference);

                    let Some(open) = session.as_mut() else {
                        unreachable!("open_session always leaves a session in place")
                    };
                    if !restarting {
                        open.begin_run();
                    }
                    let attempt = open.session.begin_attempt_at(open.from);
                    // A pass begun again: what spoiled the last one is gone.
                    let to = open.looping.as_mut().map(|looping| {
                        looping.pass.again();
                        looping.pass.range().end
                    });

                    *ongoing = Some(Ongoing {
                        instrument,
                        attempt,
                        started: Instant::now(),
                        began_ms: unix_ms(),
                        to,
                        keyboard,
                        lost_since: None,
                        last_reconnect_attempt: Instant::now(),
                    });

                    drop(reply.send(Ok(())));
                }
                Err(failure) => drop(reply.send(Err(failure))),
            }
        }
        Command::End { ending, reply } => {
            drop(reply.send(end(session, ongoing, ending, records, sink)));
        }
        Command::StartAt {
            piece_id,
            reference,
            measure,
            reply,
        } => {
            drop(reply.send(start_at(
                session, ongoing, piece_id, reference, measure, records, sink,
            )));
        }
        Command::LoopBars {
            piece_id,
            reference,
            wanted,
            reply,
        } => {
            drop(reply.send(loop_bars(
                session, ongoing, piece_id, reference, wanted, records, sink,
            )));
        }
        Command::ClearLoop => clear_loop(session, ongoing),
        Command::LoopTarget { target } => {
            if let Some(looping) = session.as_mut().and_then(|open| open.looping.as_mut()) {
                looping.pass.aim_for(target);
            }
        }
        Command::OutputPorts { reply } => {
            drop(reply.send(output_ports()));
        }
        Command::SetThru {
            input,
            output,
            reply,
        } => {
            // Torn down unconditionally first: reconnecting is cheap
            // compared to reasoning about whether a change is "the same
            // route" worth keeping open.
            *thru = None;

            let outcome = match (input, output) {
                (Some(input), Some(output)) => {
                    Thru::connect(&input, &output).map(|connected| *thru = Some(connected))
                }
                _ => Ok(()),
            };

            drop(reply.send(outcome));
        }
    }
}

/// Ends the running attempt as the performer chose. Leaving also puts the
/// start point back at the beginning and ends any loop, running or not.
fn end(
    session: &mut Option<OpenSession>,
    ongoing: &mut Option<Ongoing>,
    ending: Ending,
    records: &Records,
    sink: &Sink,
) -> Result<AttemptSummary, NotPracticing> {
    let outcome = ongoing.take().map(|active| {
        let kept = active.kept(match ending {
            Ending::Stop => Ended::Stop,
            Ending::Finished => Ended::Finished,
            Ending::Left => Ended::Left,
        });
        let summary = finish(session, active.attempt, kept, records, false, None);
        emit(
            sink,
            &PracticeEvent::Ended {
                summary: Box::new(summary.clone()),
            },
        );
        summary
    });
    if ending == Ending::Left
        && let Some(open) = session.as_mut()
    {
        open.from = 0;
        open.looping = None;
        open.run = Run::default();
    }
    outcome.ok_or(NotPracticing)
}

/// Stops looping. The start point stays at the loop's start.
fn clear_loop(session: &mut Option<OpenSession>, ongoing: &mut Option<Ongoing>) {
    if let Some(open) = session.as_mut() {
        open.looping = None;
    }
    // The pass in progress carries on past where the loop ended, so it is
    // no longer a pass of it.
    if let Some(active) = ongoing.as_mut() {
        active.to = None;
    }
}

/// Drains the keyboard and advances the loop by one tick, if an attempt is
/// live. Also handles the disconnect/reconnect/auto-end cycle.
fn poll(
    source: &mut MidiInstruments,
    session: &mut Option<OpenSession>,
    ongoing: &mut Option<Ongoing>,
    records: &Records,
    sink: &Sink,
) {
    let Some(active) = ongoing.as_mut() else {
        return;
    };

    let lost_since = active.lost_since;

    match lost_since {
        None => {
            if active.keyboard.is_connected() {
                drain(active, session, records, sink);
            } else {
                active.lost_since = Some(Instant::now());
                emit(
                    sink,
                    &PracticeEvent::InstrumentLost {
                        instrument: active.instrument.name().to_owned(),
                        seconds_remaining: RECONNECT_WINDOW.as_secs(),
                    },
                );
            }
        }
        Some(since) => {
            let elapsed = since.elapsed();

            if elapsed >= RECONNECT_WINDOW {
                let Some(active) = ongoing.take() else {
                    unreachable!("matched on ongoing.as_mut() above")
                };
                let kept = active.kept(Ended::Lost);
                let summary = finish(
                    session,
                    active.attempt,
                    kept,
                    records,
                    true,
                    Some("The instrument did not reconnect.".to_owned()),
                );
                emit(
                    sink,
                    &PracticeEvent::Ended {
                        summary: Box::new(summary),
                    },
                );
            } else if active.last_reconnect_attempt.elapsed() >= RECONNECT_ATTEMPT_EVERY {
                active.last_reconnect_attempt = Instant::now();

                match source.open(&active.instrument) {
                    Ok(keyboard) => {
                        active.keyboard = keyboard;
                        active.lost_since = None;
                        emit(sink, &PracticeEvent::InstrumentRestored);
                    }
                    Err(_) => emit(
                        sink,
                        &PracticeEvent::InstrumentLost {
                            instrument: active.instrument.name().to_owned(),
                            seconds_remaining: RECONNECT_WINDOW.saturating_sub(elapsed).as_secs(),
                        },
                    ),
                }
            }
        }
    }
}

/// Drains everything heard since the last tick, playing each note into the
/// Attempt and reporting what happened. Releases and the pedal are only
/// recorded. While looping, a pass that reaches the loop's end begins the
/// next, and notes after it in the same batch belong to that one.
fn drain(active: &mut Ongoing, session: &mut Option<OpenSession>, records: &Records, sink: &Sink) {
    for heard in active.keyboard.heard() {
        // Saturating: a message stamped a moment before the attempt began
        // (already in flight at Start) counts as its first instant.
        let at_us = u64::try_from(
            heard
                .at
                .saturating_duration_since(active.started)
                .as_micros(),
        )
        .unwrap_or(u64::MAX);

        let (pitch, velocity) = match heard.action {
            Action::Pressed { pitch, velocity } => (pitch, velocity),
            Action::Released { pitch } => {
                active.attempt.release(pitch, at_us);
                continue;
            }
            Action::Pedal { down } => {
                active.attempt.pedal(at_us, down);
                continue;
            }
        };

        // The correctness signal `CursorMovement` deliberately does not
        // give: was this pitch one the Cursor was waiting for, checked
        // *before* the Cursor is told about it.
        let correct = active
            .attempt
            .cursor()
            .expecting()
            .is_some_and(|expected| expected.sounding().any(|note| note == pitch));

        // A grace note or part of an ornament: recorded like every note,
        // but neither correct nor wrong, so nothing is marked on the score.
        let tolerated = active.attempt.cursor().tolerates(pitch);

        if let Some(looping) = session.as_mut().and_then(|open| open.looping.as_mut())
            && looping.pass.heard(active.attempt.cursor(), pitch)
        {
            emit(sink, &PracticeEvent::LoopCount { clean_in_a_row: 0 });
        }

        let position = active.attempt.cursor().position();
        let movement = active.attempt.play_touched(
            pitch,
            Touch {
                at_us,
                velocity,
                released_us: None,
            },
        );

        if tolerated {
            continue;
        }

        emit(
            sink,
            &PracticeEvent::Played {
                position,
                correct,
                pitch: u8::from(pitch),
            },
        );

        if movement == CursorMovement::Advanced {
            emit(
                sink,
                &PracticeEvent::Advanced {
                    position: active.attempt.cursor().position(),
                },
            );
        }

        let wrapped = session
            .as_ref()
            .and_then(|open| open.looping.as_ref())
            .is_some_and(|looping| looping.pass.is_at_the_end(active.attempt.cursor()));
        if wrapped {
            wrap(active, session, records, sink);
        } else if active.attempt.cursor().has_reached_the_end() {
            emit(sink, &PracticeEvent::ReachedEnd);
        }
    }
}

/// A pass of the loop reached its end: it is kept, and the next begins at
/// the loop's start, unless that was the last one asked for.
fn wrap(active: &mut Ongoing, session: &mut Option<OpenSession>, records: &Records, sink: &Sink) {
    let Some(open) = session.as_mut() else {
        unreachable!("an attempt in progress implies its session is open");
    };
    let Some(looping) = open.looping.as_mut() else {
        return;
    };
    let range = looping.pass.range();

    match looping.pass.wrapped() {
        Pass::Again { clean_in_a_row } => {
            begin_again(
                session,
                active,
                range.start,
                Some(range.end),
                Ended::Pass,
                records,
            );
            emit(
                sink,
                &PracticeEvent::Looped {
                    position: range.start,
                    clean_in_a_row,
                },
            );
        }
        Pass::Done { passes } => {
            let (first, last) = (looping.first, looping.last);
            open.looping = None;
            let widen = review::widened(first, last, open.session.reference());
            begin_again(session, active, range.start, None, Ended::Pass, records);
            emit(
                sink,
                &PracticeEvent::LoopDone {
                    position: range.start,
                    passes,
                    first,
                    last,
                    widen,
                },
            );
        }
    }
}

/// Ends the running attempt as `ended` (kept if anything was played) and
/// begins the next at `position` on the same keyboard: a pass of a loop
/// ending at `to`, if given.
fn begin_again(
    session: &mut Option<OpenSession>,
    active: &mut Ongoing,
    position: usize,
    to: Option<usize>,
    ended: Ended,
    records: &Records,
) {
    let Some(open) = session.as_ref() else {
        unreachable!("an attempt in progress implies its session is open");
    };
    let next = open.session.begin_attempt_at(position);
    let kept = active.kept(ended);
    let previous = std::mem::replace(&mut active.attempt, next);
    active.began_ms = unix_ms();
    active.to = to;
    active.started = Instant::now();
    drop(finish(session, previous, kept, records, false, None));
}

impl Ongoing {
    /// What the record will say of this attempt, ended as `ended`.
    const fn kept(&self, ended: Ended) -> Kept {
        Kept {
            began_ms: Some(self.began_ms),
            ended,
            to: self.to,
        }
    }
}

/// Opens a session on `piece_id`, unless one is already open on it.
fn open_session(
    session: &mut Option<OpenSession>,
    piece_id: PieceId,
    reference: Arc<ReferencePerformance>,
) {
    if session
        .as_ref()
        .is_some_and(|open| open.piece_id == piece_id)
    {
        return;
    }

    let stamp = unix_ms();

    *session = Some(OpenSession {
        id: format!("{}-{stamp}", piece_id.as_str()),
        started: stamp.to_string(),
        piece_id,
        session: PracticeSession::open(reference),
        kept: Vec::new(),
        from: 0,
        looping: None,
        run: Run::default(),
    });
}

/// Moves the start point to `measure`, and, while practising, ends the
/// running attempt as a jump and begins the next one there.
///
/// The one place a start point is set from outside, so the bar click, and
/// later looping and the feedback buttons, all go through the same rule.
fn start_at(
    session: &mut Option<OpenSession>,
    ongoing: &mut Option<Ongoing>,
    piece_id: PieceId,
    reference: Arc<ReferencePerformance>,
    measure: MeasureNumber,
    records: &Records,
    sink: &Sink,
) -> Result<StartPoint, NoStart> {
    if ongoing
        .as_ref()
        .is_some_and(|active| active.lost_since.is_some())
    {
        return Err(NoStart::InstrumentLost);
    }

    let position = reference
        .first_position_of(measure)
        .ok_or(NoStart::NothingThere)?;
    let used = reference
        .at(position)
        .map_or(measure.get(), |step| step.measure().get());

    open_session(session, piece_id, reference);
    let Some(open) = session.as_mut() else {
        unreachable!("open_session always leaves a session in place")
    };
    open.from = position;
    // A plain click is somewhere to start, not more of the loop.
    open.looping = None;

    if let Some(active) = ongoing.as_mut() {
        // Notes already played belong to the attempt being left.
        drain(active, session, records, sink);
        begin_again(session, active, position, None, Ended::Jump, records);
    }

    Ok(StartPoint {
        position,
        measure: used,
    })
}

/// Loops the bars `wanted` asks for: the start point moves to the loop's
/// start, and, while practising, the running attempt ends as a jump and the
/// first pass begins. Refused, changing nothing, as [`start_at`] is.
fn loop_bars(
    session: &mut Option<OpenSession>,
    ongoing: &mut Option<Ongoing>,
    piece_id: PieceId,
    reference: Arc<ReferencePerformance>,
    wanted: LoopOf,
    records: &Records,
    sink: &Sink,
) -> Result<LoopRange, NoStart> {
    if ongoing
        .as_ref()
        .is_some_and(|active| active.lost_since.is_some())
    {
        return Err(NoStart::InstrumentLost);
    }

    let range = reference
        .range_of(wanted.first, wanted.last)
        .ok_or(NoStart::NothingThere)?;
    let (first, last) = (
        wanted.first.min(wanted.last).get(),
        wanted.first.max(wanted.last).get(),
    );

    open_session(session, piece_id, reference);
    let Some(open) = session.as_mut() else {
        unreachable!("open_session always leaves a session in place")
    };
    open.from = range.start;
    open.looping = None;

    if let Some(active) = ongoing.as_mut() {
        drain(active, session, records, sink);
        begin_again(
            session,
            active,
            range.start,
            Some(range.end),
            Ended::Jump,
            records,
        );
    }

    if let Some(open) = session.as_mut() {
        open.looping = Some(Looping {
            pass: Loop::new(range.clone(), wanted.target),
            first,
            last,
            since: open.run.segments.len(),
        });
    }

    Ok(LoopRange {
        position: range.start,
        first,
        last,
    })
}

/// The wall clock, in Unix milliseconds.
fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

/// Ends an attempt, analyses it, derives feedback, keeps the record, and
/// reports all of it as an [`AttemptSummary`].
///
/// The one function every ending funnels through — an explicit `End`
/// command, reaching the end of the piece, or the instrument never coming
/// back — so there is exactly one place an [`hogwash_practice::EndedAttempt`]
/// is produced.
fn finish(
    session: &mut Option<OpenSession>,
    attempt: Attempt,
    kept: Kept,
    records: &Records,
    auto_ended: bool,
    reason: Option<String>,
) -> AttemptSummary {
    let Some(open) = session.as_mut() else {
        unreachable!("an attempt in progress implies its session is open");
    };

    let finished = attempt.end();
    let reached_the_end = finished.was_completed();

    let end = kept.to.unwrap_or_else(|| open.session.reference().len());
    let analysis = Analysis::over(
        finished.observed(),
        open.session.reference(),
        finished.from()..end,
    );
    let feedback = Feedback::of(&analysis);
    let overall = feedback.overall();
    let lines = feedback.lines();

    let steps = finished.from()..end;
    let gave_up = matches!(kept.ended, Ended::Restart | Ended::Stop);
    let trace = Trace::of(
        finished.observed(),
        open.session.reference(),
        steps.clone(),
        gave_up,
    );
    // This attempt joins the run; the results cover the whole run, or the
    // loop still running when it stopped.
    let ending = match kept.ended {
        Ended::Pass => review::Ending::Pass,
        Ended::Restart => review::Ending::Restart,
        _ => review::Ending::Other,
    };
    open.run.segments.extend(Segment::of(
        open.session.reference(),
        finished.from(),
        finished.observed(),
        &analysis,
        trace,
        ending,
    ));
    let looping = open
        .looping
        .as_ref()
        .filter(|_| kept.to.is_some())
        .map(|looping| LoopScope {
            first: looping.first,
            last: looping.last,
            since: looping.since,
        });
    let review = review::of(open.session.reference(), &open.run, looping);

    // An attempt in which nothing was played is not practice: it is not
    // added to the session and nothing is written.
    if !finished.observed().is_empty() {
        open.session.record(finished);
        open.kept.push(kept);

        let record = SessionRecord {
            id: open.id.clone(),
            piece: open.piece_id.clone(),
            started: open.started.clone(),
            closed: false,
            attempts: open
                .session
                .attempts()
                .iter()
                .zip(&open.kept)
                .map(|(attempt, kept)| AttemptRecord {
                    began_ms: kept.began_ms,
                    from: attempt.from(),
                    to: kept.to,
                    played: attempt.observed().clone(),
                    reached: attempt.reached(),
                    ended: Some(kept.ended),
                })
                .collect(),
        };

        // A failure here is a filesystem problem, not a reason to withhold the
        // summary the performer just earned — they can still read it, even if
        // it could not be saved this time.
        drop(records.keep(&record));
    }

    AttemptSummary {
        lines,
        correct: overall.correct(),
        incorrect: overall.incorrect(),
        omitted: overall.omitted(),
        unexpected: overall.unexpected(),
        reached_the_end,
        auto_ended,
        reason,
        review,
    }
}

fn emit(sink: &Sink, event: &PracticeEvent) {
    sink(event);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use hogwash_domain::Pitch;
    use hogwash_practice::PracticeSession;
    use hogwash_records::{Ended, Records};

    use super::{
        Command, Ending, Kept, LoopOf, LoopRange, NoStart, Ongoing, OpenSession, Practice,
        PracticeEvent, Sink, drain, end, finish, handle, loop_bars, start_at,
    };

    // The interface reads `secondsRemaining`. Serde's `rename_all` on the enum
    // leaves a variant's fields alone, and the countdown once arrived as
    // `seconds_remaining`: the banner said "Waiting s".
    #[test]
    fn the_lost_event_names_its_countdown_as_the_interface_reads_it() {
        let event = PracticeEvent::InstrumentLost {
            instrument: "Digital Piano".to_owned(),
            seconds_remaining: 60,
        };

        let json = serde_json::to_value(&event).expect("the event serialises");

        assert_eq!(json["kind"], "instrumentLost");
        assert_eq!(json["instrument"], "Digital Piano");
        assert_eq!(json["secondsRemaining"], 60);
    }

    // Nothing is kept for an attempt in which nothing was played, and the
    // next one with a note is. Uses the real Minuet, so it needs a real engine.
    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn only_an_attempt_with_notes_is_kept() {
        let directory = std::env::temp_dir().join("hogwash-practice-keeps-test");
        drop(std::fs::remove_dir_all(&directory));

        let records = Records::under(&directory);
        let engraving = crate::engraving::Engraving::start().expect("the engine starts");
        let practice =
            Practice::start(records.clone(), |_| {}).expect("the practice thread starts");
        let hogwash = crate::state::Hogwash::start(engraving, records.clone(), practice)
            .expect("the provided material prepares");

        let minuet = hogwash
            .library()
            .pieces()
            .into_iter()
            .find(|piece| piece.title() == "Minuet in G Major")
            .expect("the Minuet is on the shelf");
        let id = minuet.id().clone();

        let mut session = Some(OpenSession {
            piece_id: id.clone(),
            id: "keeps-test".to_owned(),
            started: "0".to_owned(),
            session: PracticeSession::open(Arc::new(minuet.reference().clone())),
            kept: Vec::new(),
            from: 0,
            looping: None,
            run: crate::review::Run::default(),
        });

        let stop = Kept {
            began_ms: None,
            ended: Ended::Stop,
            to: None,
        };
        let nothing = session.as_ref().expect("open").session.begin_attempt();
        drop(finish(&mut session, nothing, stop, &records, false, None));
        assert_eq!(records.sessions_of(&id), 0, "an empty attempt was kept");

        // D5 is the first note of the Minuet.
        let mut one = session.as_ref().expect("open").session.begin_attempt();
        one.play(Pitch::new(74).expect("D5 is a piano key"));
        drop(finish(&mut session, one, stop, &records, false, None));
        assert_eq!(
            records.sessions_of(&id),
            1,
            "an attempt with a note was not kept"
        );
    }

    /// A keyboard that is there and plays nothing.
    struct Silent;
    impl hogwash_capture::Keyboard for Silent {
        fn heard(&mut self) -> Vec<hogwash_capture::Heard> {
            Vec::new()
        }
        fn is_connected(&self) -> bool {
            true
        }
    }

    /// The real Minuet, through the real engine, and an empty history.
    fn minuet(
        name: &str,
    ) -> (
        Records,
        hogwash_domain::PieceId,
        Arc<hogwash_notation::ReferencePerformance>,
    ) {
        let directory = std::env::temp_dir().join(format!("hogwash-practice-{name}"));
        drop(std::fs::remove_dir_all(&directory));
        let records = Records::under(&directory);
        let engraving = crate::engraving::Engraving::start().expect("the engine starts");
        let practice =
            Practice::start(records.clone(), |_| {}).expect("the practice thread starts");
        let hogwash = crate::state::Hogwash::start(engraving, records.clone(), practice)
            .expect("the provided material prepares");
        let piece = hogwash
            .library()
            .pieces()
            .into_iter()
            .find(|piece| piece.title() == "Minuet in G Major")
            .expect("the Minuet is on the shelf");
        (
            records,
            piece.id().clone(),
            Arc::new(piece.reference().clone()),
        )
    }

    fn practising(reference: &Arc<hogwash_notation::ReferencePerformance>) -> Ongoing {
        Ongoing {
            instrument: hogwash_capture::Instrument::new("Silent".to_owned()),
            attempt: hogwash_practice::Attempt::begin(Arc::clone(reference)),
            started: std::time::Instant::now(),
            began_ms: 0,
            to: None,
            keyboard: Box::new(Silent),
            lost_since: None,
            last_reconnect_attempt: std::time::Instant::now(),
        }
    }

    fn bar(number: u32) -> hogwash_domain::MeasureNumber {
        hogwash_domain::MeasureNumber::new(number).expect("a bar")
    }

    fn quiet() -> Sink {
        Box::new(|_| {})
    }

    // Bar 9 of the Minuet is step 36 in played order: the start point and
    // the attempt that begins there both say 36, never 9.
    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn clicking_a_bar_before_starting_only_moves_the_start() {
        let (records, id, reference) = minuet("idle-click");
        let mut session = None;
        let mut ongoing = None;

        let point = start_at(
            &mut session,
            &mut ongoing,
            id.clone(),
            Arc::clone(&reference),
            bar(9),
            &records,
            &quiet(),
        )
        .expect("bar 9 has notes");

        assert_eq!((point.position, point.measure), (36, 9));
        assert_eq!(session.as_ref().expect("open").from, 36);
        assert!(ongoing.is_none());
        assert_eq!(records.sessions_of(&id), 0);
    }

    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn jumping_repeatedly_without_playing_saves_no_attempts() {
        let (records, id, reference) = minuet("empty-jumps");
        let mut session = None;
        let mut ongoing = Some(practising(&reference));

        for number in [9, 3, 5, 9] {
            start_at(
                &mut session,
                &mut ongoing,
                id.clone(),
                Arc::clone(&reference),
                bar(number),
                &records,
                &quiet(),
            )
            .expect("a bar with notes");
        }

        assert_eq!(records.sessions_of(&id), 0);
        assert_eq!(ongoing.expect("still practising").attempt.from(), 36);
    }

    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn a_jump_keeps_what_was_played_and_begins_again_at_the_bar() {
        let (records, id, reference) = minuet("jump-keeps");
        let mut session = None;
        let mut ongoing = Some(practising(&reference));
        // D5, the Minuet's first note.
        ongoing
            .as_mut()
            .expect("practising")
            .attempt
            .play(Pitch::new(74).expect("a key"));

        start_at(
            &mut session,
            &mut ongoing,
            id.clone(),
            Arc::clone(&reference),
            bar(9),
            &records,
            &quiet(),
        )
        .expect("bar 9 has notes");

        let (kept, _) = records.sessions().expect("history reads");
        let attempts = &kept.first().expect("one session").attempts;
        assert_eq!(attempts.len(), 1);
        assert_eq!(
            (attempts[0].from, attempts[0].ended),
            (0, Some(Ended::Jump))
        );
        let running = &ongoing.expect("still practising").attempt;
        assert_eq!((running.from(), running.cursor().position()), (36, 36));
    }

    // The banner says the keyboard is gone; a click must change nothing.
    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn a_click_while_the_keyboard_is_lost_is_refused_and_changes_nothing() {
        let (records, id, reference) = minuet("lost-click");
        let mut session = None;
        let mut active = practising(&reference);
        active.attempt.play(Pitch::new(74).expect("a key"));
        active.lost_since = Some(std::time::Instant::now());
        let mut ongoing = Some(active);

        let refused = start_at(
            &mut session,
            &mut ongoing,
            id.clone(),
            Arc::clone(&reference),
            bar(9),
            &records,
            &quiet(),
        );

        assert_eq!(refused, Err(NoStart::InstrumentLost));
        assert!(session.is_none());
        assert_eq!(ongoing.expect("still waiting").attempt.from(), 0);
        assert_eq!(records.sessions_of(&id), 0);
    }

    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn leaving_the_piece_puts_the_start_back_at_the_beginning() {
        let (records, id, reference) = minuet("leave-resets");
        let mut session = None;
        let mut ongoing = None;
        start_at(
            &mut session,
            &mut ongoing,
            id,
            Arc::clone(&reference),
            bar(9),
            &records,
            &quiet(),
        )
        .expect("bar 9 has notes");

        let (reply, answer) = std::sync::mpsc::sync_channel(1);
        handle(
            Command::End {
                ending: Ending::Left,
                reply,
            },
            &mut hogwash_midi::MidiInstruments,
            &mut session,
            &mut ongoing,
            &mut None,
            &records,
            &quiet(),
        );

        assert!(
            answer.recv().expect("a reply").is_err(),
            "nothing was running"
        );
        assert_eq!(session.expect("open").from, 0);
    }

    /// A keyboard that plays the keys given, all at once, then nothing.
    struct Plays(Vec<u8>);
    impl hogwash_capture::Keyboard for Plays {
        fn heard(&mut self) -> Vec<hogwash_capture::Heard> {
            std::mem::take(&mut self.0)
                .into_iter()
                .map(|key| hogwash_capture::Heard {
                    at: std::time::Instant::now(),
                    action: hogwash_capture::Action::Pressed {
                        pitch: Pitch::new(key).expect("a key"),
                        velocity: 64,
                    },
                })
                .collect()
        }
        fn is_connected(&self) -> bool {
            true
        }
    }

    /// Bar 9 of the Minuet, steps 36 to 40: D5 over B3, G4, A4, B4 over
    /// A3, C5.
    const BAR_9: [u8; 7] = [74, 59, 67, 69, 71, 57, 72];

    fn looping(first: u32, last: u32, target: Option<u32>) -> LoopOf {
        LoopOf {
            first: bar(first),
            last: bar(last),
            target,
        }
    }

    /// What was sent to the interface, as it reads it.
    fn listening() -> (Sink, Arc<std::sync::Mutex<Vec<serde_json::Value>>>) {
        let heard = Arc::new(std::sync::Mutex::new(Vec::new()));
        let into = Arc::clone(&heard);
        let sink: Sink = Box::new(move |event| {
            into.lock()
                .expect("not poisoned")
                .push(serde_json::to_value(event).expect("serialises"));
        });
        (sink, heard)
    }

    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn looping_before_starting_only_moves_the_start() {
        let (records, id, reference) = minuet("idle-loop");
        let mut session = None;

        let range = loop_bars(
            &mut session,
            &mut None,
            id.clone(),
            reference,
            looping(10, 9, Some(3)),
            &records,
            &quiet(),
        )
        .expect("bars 9-10 are played in a row");

        assert_eq!(
            range,
            LoopRange {
                position: 36,
                first: 9,
                last: 10
            }
        );
        let open = session.expect("open");
        assert_eq!(open.from, 36);
        assert_eq!(open.looping.expect("looping").pass.range(), 36..44);
        assert_eq!(records.sessions_of(&id), 0);
    }

    // Bars 15-18 run over the end of the first repeat: only the second time
    // through bar 15 goes on into bar 17.
    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn a_loop_over_a_repeat_sign_starts_where_its_bars_are_played_in_a_row() {
        let (records, id, reference) = minuet("repeat-loop");
        let first_15 = reference.first_position_of(bar(15)).expect("bar 15");

        let range = loop_bars(
            &mut None,
            &mut None,
            id,
            Arc::clone(&reference),
            looping(15, 18, None),
            &records,
            &quiet(),
        )
        .expect("played in a row the second time");

        assert!(range.position > first_15);
        assert_eq!(
            reference.at(range.position).expect("a step").measure(),
            bar(15)
        );
    }

    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn a_pass_that_reaches_the_loops_end_is_kept_and_the_next_begins() {
        let (records, id, reference) = minuet("pass-kept");
        let mut session = None;
        let mut ongoing = Some(practising(&reference));
        loop_bars(
            &mut session,
            &mut ongoing,
            id.clone(),
            Arc::clone(&reference),
            looping(9, 9, None),
            &records,
            &quiet(),
        )
        .expect("bar 9 loops");
        let (sink, heard) = listening();

        let active = ongoing.as_mut().expect("practising");
        active.keyboard = Box::new(Plays(BAR_9.to_vec()));
        drain(active, &mut session, &records, &sink);

        let (kept, _) = records.sessions().expect("history reads");
        let attempts = &kept.first().expect("one session").attempts;
        assert_eq!(attempts.len(), 1);
        let pass = &attempts[0];
        assert_eq!(
            (pass.from, pass.to, pass.reached, pass.ended),
            (36, Some(41), 41, Some(Ended::Pass))
        );
        let running = ongoing.expect("still practising");
        assert_eq!((running.attempt.from(), running.to), (36, Some(41)));
        assert!(heard.lock().expect("not poisoned").iter().any(|event| {
            event["kind"] == "looped" && event["cleanInARow"] == 1 && event["position"] == 36
        }));
    }

    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn reaching_the_target_ends_the_loop_with_the_cursor_at_its_start() {
        let (records, id, reference) = minuet("loop-done");
        let mut session = None;
        let mut ongoing = Some(practising(&reference));
        loop_bars(
            &mut session,
            &mut ongoing,
            id,
            Arc::clone(&reference),
            looping(9, 9, Some(1)),
            &records,
            &quiet(),
        )
        .expect("bar 9 loops");
        let (sink, heard) = listening();

        let active = ongoing.as_mut().expect("practising");
        // A wrong C#5 first: that pass does not count, the next does.
        let mut keys = vec![73];
        keys.extend(BAR_9);
        keys.extend(BAR_9);
        active.keyboard = Box::new(Plays(keys));
        drain(active, &mut session, &records, &sink);

        assert!(session.expect("open").looping.is_none());
        let running = ongoing.expect("still practising");
        assert_eq!(
            (running.attempt.cursor().position(), running.to),
            (36, None)
        );
        let events = heard.lock().expect("not poisoned");
        assert!(
            events
                .iter()
                .any(|event| event["kind"] == "looped" && event["cleanInARow"] == 0)
        );
        assert!(events.iter().any(|event| {
            event["kind"] == "loopDone"
                && event["passes"] == 1
                && event["first"] == 9
                && event["widen"]["label"] == "Loop bars 8–10"
        }));
    }

    // Started at bar 9, one wrong C#5 there, bar 9 played, then Stop: the
    // review runs from bar 9 to the end, bar 10 on not reached.
    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn stopping_reviews_the_attempt_from_where_it_began() {
        let (records, id, reference) = minuet("review-stop");
        let mut session = None;
        let mut ongoing = Some(practising(&reference));
        start_at(
            &mut session,
            &mut ongoing,
            id,
            Arc::clone(&reference),
            bar(9),
            &records,
            &quiet(),
        )
        .expect("bar 9 starts");
        let active = ongoing.as_mut().expect("practising");
        let mut keys = vec![73];
        keys.extend(BAR_9);
        active.keyboard = Box::new(Plays(keys));
        drain(active, &mut session, &records, &quiet());

        let review = end(&mut session, &mut ongoing, Ending::Stop, &records, &quiet())
            .expect("an attempt was running")
            .review;

        let state = |measure: u32| {
            review
                .bars
                .iter()
                .find(|column| column.measure == measure)
                .map(|column| (column.state, column.wrong))
        };
        assert_eq!(review.bars.first().map(|column| column.measure), Some(9));
        assert_eq!(review.bars.last().map(|column| column.measure), Some(32));
        assert_eq!(state(9), Some((crate::review::BarState::Played, 1)));
        assert_eq!(
            state(11).map(|(state, _)| state),
            Some(crate::review::BarState::NotReached)
        );
        assert!(
            review
                .marks
                .iter()
                .any(|mark| mark.position == 36 && mark.correct && mark.wrong == 1)
        );
        assert!(review.headline.starts_with("Bar"), "{}", review.headline);
        assert!(
            review
                .suggestion
                .is_some_and(|button| button.first == Some(9))
        );
    }

    // Bar 9 with a wrong C#5, a click back to bar 9, bar 9 with the same
    // wrong C#5, then Stop: the results cover both parts of the run, and the
    // same mistake both times is worth a card.
    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn the_results_cover_the_whole_run() {
        let (records, id, reference) = minuet("review-run");
        let mut session = None;
        let mut ongoing = Some(practising(&reference));
        for keys in [
            [&[73][..], &BAR_9[..]].concat(),
            [&[73][..], &BAR_9[..]].concat(),
        ] {
            start_at(
                &mut session,
                &mut ongoing,
                id.clone(),
                Arc::clone(&reference),
                bar(9),
                &records,
                &quiet(),
            )
            .expect("bar 9 starts");
            let active = ongoing.as_mut().expect("practising");
            active.keyboard = Box::new(Plays(keys));
            drain(active, &mut session, &records, &quiet());
        }

        let review = end(&mut session, &mut ongoing, Ending::Stop, &records, &quiet())
            .expect("an attempt was running")
            .review;

        let numbers = &review.numbers;
        assert_eq!((numbers.right, numbers.wrong), (14, 2));
        // Something to work on in bar 9, worded, with a button.
        let card = review.cards.first().expect("a card");
        assert!(card.first <= 9 && 9 <= card.last, "{card:?}");
        assert!(!card.title.is_empty() && !card.steps.is_empty() && card.button.is_some());
        assert_eq!(numbers.accuracy, Some(88));
        assert!(review.passes.is_empty());
        assert_eq!(
            review
                .bars
                .iter()
                .find(|column| column.measure == 9)
                .map(|column| column.wrong),
            Some(2)
        );
    }

    // Two passes of bar 9, the second spoiled by a wrong note, then Stop part
    // way into a third: the review is of the whole run.
    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn stopping_a_loop_reviews_every_pass() {
        let (records, id, reference) = minuet("review-loop");
        let mut session = None;
        let mut ongoing = Some(practising(&reference));
        loop_bars(
            &mut session,
            &mut ongoing,
            id,
            Arc::clone(&reference),
            looping(9, 9, None),
            &records,
            &quiet(),
        )
        .expect("bar 9 loops");
        let active = ongoing.as_mut().expect("practising");
        let mut keys = BAR_9.to_vec();
        keys.push(73);
        keys.extend(BAR_9);
        keys.push(74);
        active.keyboard = Box::new(Plays(keys));
        drain(active, &mut session, &records, &quiet());

        let review = end(&mut session, &mut ongoing, Ending::Stop, &records, &quiet())
            .expect("an attempt was running")
            .review;

        assert_eq!(review.headline, "Bar 9 · 2 passes · 1 clean.");
        assert_eq!(
            review
                .bars
                .iter()
                .map(|column| column.measure)
                .collect::<Vec<_>>(),
            [9]
        );
    }

    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn a_plain_click_or_leaving_ends_the_loop() {
        let (records, id, reference) = minuet("click-unloops");
        let mut session = None;
        let mut ongoing = None;
        let loop_9 = |session: &mut Option<OpenSession>| {
            loop_bars(
                session,
                &mut None,
                id.clone(),
                Arc::clone(&reference),
                looping(9, 10, None),
                &records,
                &quiet(),
            )
            .expect("bars 9-10 loop");
        };

        loop_9(&mut session);
        start_at(
            &mut session,
            &mut ongoing,
            id.clone(),
            Arc::clone(&reference),
            bar(3),
            &records,
            &quiet(),
        )
        .expect("bar 3 has notes");
        assert!(session.as_ref().expect("open").looping.is_none());

        loop_9(&mut session);
        let (reply, _answer) = std::sync::mpsc::sync_channel(1);
        handle(
            Command::End {
                ending: Ending::Left,
                reply,
            },
            &mut hogwash_midi::MidiInstruments,
            &mut session,
            &mut ongoing,
            &mut None,
            &records,
            &quiet(),
        );
        assert!(session.expect("open").looping.is_none());
    }

    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn a_loop_asked_for_while_the_keyboard_is_lost_is_refused() {
        let (records, id, reference) = minuet("lost-loop");
        let mut active = practising(&reference);
        active.lost_since = Some(std::time::Instant::now());
        let mut session = None;

        let refused = loop_bars(
            &mut session,
            &mut Some(active),
            id,
            reference,
            looping(9, 10, None),
            &records,
            &quiet(),
        );

        assert_eq!(refused, Err(NoStart::InstrumentLost));
        assert!(session.is_none());
    }

    #[test]
    #[ignore = "needs a real libverovio; run `just provided-test`"]
    fn a_bar_that_does_not_exist_is_refused() {
        let (records, id, reference) = minuet("no-bar");
        let refused = start_at(
            &mut None,
            &mut None,
            id,
            reference,
            bar(99),
            &records,
            &quiet(),
        );

        assert_eq!(refused, Err(NoStart::NothingThere));
    }
}
