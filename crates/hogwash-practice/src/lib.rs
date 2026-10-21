//! Wait-for-me practice.
//!
//! `MVP §5` calls this "the decision that shapes Version 1", and it is one
//! sentence: **the cursor does not advance until the expected note is
//! played.** Everything in this crate follows from it.
//!
//! What that buys is structural rather than clever. A skipped bar cannot
//! occur, because the Cursor will not move past unplayed notes. A long pause
//! needs no handling, because waiting is the default. Noodling records
//! nothing as failure, because the Cursor simply holds. The failure mode this
//! avoids is the one `MVP §5` names as most damaging to trust — *confident
//! wrong output on precisely the passages the performer cannot yet play*.
//!
//! # Two things this crate deliberately cannot do
//!
//! **It cannot tell time.** `004 §6.7` forbids a clock here, and `003 §15`
//! does not model timing at all. This is not an omission: if the system waits
//! for the performer, the tempo *is* the performer's speed, so "late" has
//! nothing to be measured against (`MVP §5`). No dependency in this crate can
//! supply a clock, and `scripts/check-arch.sh` fails the build if one
//! appears.
//!
//! **It cannot judge.** `003 §7.16` — *"The Cursor does not judge what was
//! played."* [`CursorMovement`] reports whether the Cursor moved, never
//! whether a note was right. Correctness is Analysis's concern
//! (`004 §8.6`), decided after the Attempt ends, over the whole performance.
//! The one exception is [`Loop`], whose clean-pass counter has to know at
//! once whether a pass was spoiled.

mod attempt;
mod cursor;
mod fixtures;
mod looping;
mod session;

pub use attempt::{Attempt, EndedAttempt};
pub use cursor::{Cursor, CursorMovement};
pub use looping::{Loop, Pass};
pub use session::{ClosedSession, PracticeSession};
