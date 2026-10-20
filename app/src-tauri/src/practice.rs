//! The practice loop, given a thread of its own.
//!
//! # Why this exists
//!
//! `MVP §6.6` / Amendment A's FR-802 want live per-note correctness on
//! screen, but `003 §7.16` is explicit that the Cursor renders no verdict —
//! `CursorMovement` has exactly two values, `Advanced` and `Held`, and `Held`
//! means "wrong note" *or* "chord not finished yet" indistinguishably. So the
//! live correctness signal cannot come from `CursorMovement`; it is computed
//! here, in Rust, the same way `Analysis::of` computes it — by checking
//! whether the played pitch was a member of `Cursor::expecting()`'s sounding
//! set *before* the Cursor is given the note. Same rule, same source, so the
//! marks on screen and the summary afterwards agree by construction
//! (`004 §6.5`) rather than by coincidence. The frontend still judges
//! nothing; this boolean is the only thing that crosses the boundary.
//!
//! # One channel, one owner — a second time
//!
//! `engraving.rs` already gave one thing a dedicated thread because it could
//! not safely be shared: a raw C pointer, there. Here it is a `Keyboard`,
//! an `Attempt` and a `PracticeSession` moving together through a lifecycle
//! that a Tauri command pool — several threads, running concurrently — has
//! no natural place to hold. Giving them one thread and a channel is
//! `ADR-0009`'s "one channel, one owner", relocated from a frame loop (which
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

use hogwash_analysis::Analysis;
use hogwash_capture::{Action, Instrument, Keyboard, MidiSource, NoInstrument, Touch};
use hogwash_domain::PieceId;
use hogwash_feedback::Feedback;
use hogwash_midi::{MidiInstruments, NoOutput, OutputPort, Thru, output_ports};
use hogwash_notation::ReferencePerformance;
use hogwash_practice::{Attempt, CursorMovement, PracticeSession};
use hogwash_records::{AttemptRecord, Records, SessionRecord};
use serde::Serialize;

use crate::startup::Unavailable;

/// Where every [`PracticeEvent`] this loop produces goes.
type Sink = Box<dyn Fn(&PracticeEvent) + Send>;

/// How often the loop drains the keyboard while an attempt is running.
///
/// Comfortably inside Amendment A's proposed 100 ms budget (NFR-202).
const CONNECTED_POLL: Duration = Duration::from_millis(25);

/// How long the loop waits when nothing is running — no keyboard to drain.
const IDLE_POLL: Duration = Duration::from_millis(500);

/// How often a lost instrument is retried while waiting for it to come back.
const RECONNECT_ATTEMPT_EVERY: Duration = Duration::from_secs(2);

/// How long an attempt waits for a lost instrument before ending itself.
///
/// The hybrid: neither "auto-end immediately" nor "wait forever" — the loss
/// is reported at once (`004 §8.5`'s *visibly*), and the attempt only ends on
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
    /// nothing more — `ADR-0001` never attaches a spelling to a played
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
    Ended { summary: AttemptSummary },
}

/// What an ended attempt is reported as.
///
/// `MVP §10` leaves the exact content of an attempt summary as an open
/// design question beyond "one overall result" (FR-408); this is Stage D's
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
}

/// Asked to end an attempt when none is running.
#[derive(Debug)]
pub struct NotPracticing;

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
        reply: SyncSender<Result<AttemptSummary, NotPracticing>>,
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

    /// Ends whatever attempt is running.
    ///
    /// # Errors
    ///
    /// [`NotPracticing`] if no attempt is running.
    pub fn end_attempt(&self) -> Result<AttemptSummary, NotPracticing> {
        let (reply, answer) = sync_channel(0);

        self.commands
            .send(Command::End { reply })
            .map_err(|_| NotPracticing)?;

        answer.recv().map_err(|_| NotPracticing)?
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
    /// When each attempt kept in `session` began, in Unix milliseconds,
    /// index for index.
    began: Vec<Option<u64>>,
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
            if let Some(previous) = ongoing.take() {
                drop(finish(
                    session,
                    previous.attempt,
                    Some(previous.began_ms),
                    records,
                    false,
                    None,
                ));
            }

            match source.open(&instrument) {
                Ok(keyboard) => {
                    open_session(session, piece_id, reference);

                    let Some(open) = session.as_ref() else {
                        unreachable!("open_session always leaves a session in place")
                    };
                    let attempt = open.session.begin_attempt();

                    *ongoing = Some(Ongoing {
                        instrument,
                        attempt,
                        started: Instant::now(),
                        began_ms: unix_ms(),
                        keyboard,
                        lost_since: None,
                        last_reconnect_attempt: Instant::now(),
                    });

                    drop(reply.send(Ok(())));
                }
                Err(failure) => drop(reply.send(Err(failure))),
            }
        }
        Command::End { reply } => {
            let outcome = ongoing.take().map(|active| {
                let summary = finish(
                    session,
                    active.attempt,
                    Some(active.began_ms),
                    records,
                    false,
                    None,
                );
                emit(
                    sink,
                    &PracticeEvent::Ended {
                        summary: summary.clone(),
                    },
                );
                summary
            });
            drop(reply.send(outcome.ok_or(NotPracticing)));
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
                drain(active, sink);
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
                let summary = finish(
                    session,
                    active.attempt,
                    Some(active.began_ms),
                    records,
                    true,
                    Some("The instrument did not reconnect.".to_owned()),
                );
                emit(sink, &PracticeEvent::Ended { summary });
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
/// recorded.
fn drain(active: &mut Ongoing, sink: &Sink) {
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

        if active.attempt.cursor().has_reached_the_end() {
            emit(sink, &PracticeEvent::ReachedEnd);
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
        began: Vec::new(),
    });
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
    began_ms: Option<u64>,
    records: &Records,
    auto_ended: bool,
    reason: Option<String>,
) -> AttemptSummary {
    let Some(open) = session.as_mut() else {
        unreachable!("an attempt in progress implies its session is open");
    };

    let ended = attempt.end();
    let reached_the_end = ended.was_completed();

    let analysis = Analysis::of(ended.observed(), open.session.reference());
    let feedback = Feedback::of(&analysis);
    let overall = feedback.overall();
    let lines = feedback.lines();

    // An attempt in which nothing was played is not practice: it is not
    // added to the session and nothing is written.
    if !ended.observed().is_empty() {
        open.session.record(ended);
        open.began.push(began_ms);

        let record = SessionRecord {
            id: open.id.clone(),
            piece: open.piece_id.clone(),
            started: open.started.clone(),
            closed: false,
            attempts: open
                .session
                .attempts()
                .iter()
                .zip(&open.began)
                .map(|(attempt, &began_ms)| AttemptRecord {
                    began_ms,
                    played: attempt.observed().clone(),
                    reached: attempt.reached(),
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
    use hogwash_records::Records;

    use super::{OpenSession, Practice, PracticeEvent, finish};

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
            began: Vec::new(),
        });

        let nothing = session.as_ref().expect("open").session.begin_attempt();
        drop(finish(&mut session, nothing, None, &records, false, None));
        assert_eq!(records.sessions_of(&id), 0, "an empty attempt was kept");

        // D5 is the first note of the Minuet.
        let mut one = session.as_ref().expect("open").session.begin_attempt();
        one.play(Pitch::new(74).expect("D5 is a piano key"));
        drop(finish(&mut session, one, None, &records, false, None));
        assert_eq!(
            records.sessions_of(&id),
            1,
            "an attempt with a note was not kept"
        );
    }
}
