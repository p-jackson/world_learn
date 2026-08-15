//! FSRS scheduling — the pure "advance one Card on a grade" step (spec §5.3, §5.4).
//!
//! Wraps the `fsrs` crate with the app's fixed policy: stock `DEFAULT_PARAMETERS`
//! (no training), `desired_retention = 0.9`, and the app-owned date granularity
//! FSRS deliberately leaves out — intervals rounded to whole days (min 1) and the
//! three local-date fields the crate does not persist (`due` / `last_review` /
//! `introduced_on`). Given the Card's prior [`CardRecord`] (or `None` for a
//! never-graded Card), a [`Grade`], and today's date, [`Scheduler::review`]
//! returns the Card's next record. It is a pure function of its inputs — no I/O,
//! no queue — so [`crate::session`] owns persistence and requeueing.
//!
//! Lands one issue ahead of its first UI caller; see [`crate::store`] for the
//! same dead-code note.
#![allow(dead_code)]

use anyhow::{Context, Result};
use fsrs::{ItemState, MemoryState, FSRS};
use jiff::civil::Date;
use jiff::ToSpan;

use crate::store::CardRecord;

/// Fixed target retention (spec §5.3). Stored as a code constant, never persisted.
pub const DESIRED_RETENTION: f32 = 0.9;

/// The learner's self-assessed 4-button grade (spec §4.1). Maps to FSRS ratings
/// 1–4; **Again** is the only failing grade and the only one that re-drills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    Again,
    Hard,
    Good,
    Easy,
}

impl Grade {
    /// Whether this grade re-drills the Card within the session (spec §5.4): only
    /// **Again**. Passes exit the Card.
    #[must_use]
    pub const fn is_again(self) -> bool {
        matches!(self, Self::Again)
    }
}

/// The scheduler: stock FSRS-6 parameters and the app's fixed retention target.
/// Construct once per session and share; [`Scheduler::review`] is the only entry.
pub struct Scheduler {
    fsrs: FSRS,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    /// A scheduler over stock `DEFAULT_PARAMETERS` (spec §5.3). `FSRS::default()`
    /// fills in the FSRS-6 / Anki defaults — no per-user training in the MVP.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fsrs: FSRS::default(),
        }
    }

    /// Advance a Card one grade and return its next persisted record (spec §5.4).
    ///
    /// `prior` is the Card's current record, or `None` for its first-ever grade (a
    /// still-new Card). On a first grade the record is created and stamped
    /// `introduced_on = today`; on later grades that stamp is preserved.
    ///
    /// - **Again** → memory updated, `due = today` (re-drills today; the persisted
    ///   `due = today` means a mid-session quit never loses the re-drill).
    /// - **Hard / Good / Easy** → memory updated, `due = today + round(interval)`,
    ///   whole days, min 1.
    ///
    /// `last_review` is set to `today` on **every** grade, so a same-day re-drill
    /// feeds `days_elapsed = 0` into FSRS (its short-term `delta_t == 0` path).
    pub fn review(
        &self,
        prior: Option<&CardRecord>,
        grade: Grade,
        today: Date,
    ) -> Result<CardRecord> {
        let current = prior.map(memory_state);
        // Spec §5.4: days_elapsed = max(0, today − last_review); a new Card's first
        // grade passes days_elapsed = 0 (with current = None).
        let days_elapsed = prior.map_or(0, |r| {
            u32::try_from((today - r.last_review).get_days().max(0)).unwrap_or(0)
        });

        let next = self
            .fsrs
            .next_states(current, DESIRED_RETENTION, days_elapsed)
            .context("computing FSRS next states")?;
        let ItemState { memory, interval } = match grade {
            Grade::Again => next.again,
            Grade::Hard => next.hard,
            Grade::Good => next.good,
            Grade::Easy => next.easy,
        };

        let due = if grade.is_again() {
            today
        } else {
            today
                .checked_add(whole_days(interval).days())
                .with_context(|| format!("computing due date from interval {interval} days"))?
        };

        Ok(CardRecord {
            stability: memory.stability,
            difficulty: memory.difficulty,
            due,
            last_review: today,
            introduced_on: prior.map_or(today, |r| r.introduced_on),
        })
    }
}

/// The FSRS memory half of a persisted record.
const fn memory_state(record: &CardRecord) -> MemoryState {
    MemoryState {
        stability: record.stability,
        difficulty: record.difficulty,
    }
}

/// Spec §5.4: `interval.round().max(1.0)`. FSRS returns raw fractional days; the
/// app owns whole-day granularity. `interval` is a positive, finite day count, so
/// the round-then-clamp keeps the unavoidable float→int cast in range.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
const fn whole_days(interval: f32) -> i64 {
    interval.round().max(1.0).min(i64::MAX as f32) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    /// A prior record standing at `last_review`, due `due`, introduced `intro`.
    fn record(
        stability: f32,
        difficulty: f32,
        last_review: Date,
        due: Date,
        intro: Date,
    ) -> CardRecord {
        CardRecord {
            stability,
            difficulty,
            due,
            last_review,
            introduced_on: intro,
        }
    }

    #[test]
    fn whole_days_rounds_and_enforces_min_one() {
        assert_eq!(whole_days(0.0), 1); // min 1, never 0
        assert_eq!(whole_days(0.4), 1);
        assert_eq!(whole_days(1.4), 1);
        assert_eq!(whole_days(1.6), 2);
        assert_eq!(whole_days(9.5), 10);
    }

    #[test]
    fn first_grade_of_a_new_card_stamps_introduced_and_reviews_today() {
        let today = date(2026, 8, 15);
        let sched = Scheduler::new();

        for grade in [Grade::Again, Grade::Hard, Grade::Good, Grade::Easy] {
            let r = sched.review(None, grade, today).unwrap();
            assert_eq!(
                r.introduced_on, today,
                "{grade:?} must stamp introduced_on = today"
            );
            assert_eq!(
                r.last_review, today,
                "{grade:?} must stamp last_review = today"
            );
            assert!(
                r.stability > 0.0,
                "{grade:?} must produce positive stability"
            );
        }
    }

    #[test]
    fn again_is_due_today_pass_is_due_later() {
        let today = date(2026, 8, 15);
        let sched = Scheduler::new();

        // Again: re-drills today.
        assert_eq!(sched.review(None, Grade::Again, today).unwrap().due, today);

        // Passes: due strictly after today (min interval 1 day).
        for grade in [Grade::Hard, Grade::Good, Grade::Easy] {
            let r = sched.review(None, grade, today).unwrap();
            assert!(
                r.due > today,
                "{grade:?} must schedule due after today, got {}",
                r.due
            );
        }
    }

    #[test]
    fn easy_schedules_no_sooner_than_good_than_hard() {
        // Monotonic first-interval ordering is a core FSRS property; a regression
        // in grade→ItemState mapping (e.g. swapped arms) would break it.
        let today = date(2026, 8, 15);
        let sched = Scheduler::new();
        let due = |g| sched.review(None, g, today).unwrap().due;
        assert!(due(Grade::Hard) <= due(Grade::Good));
        assert!(due(Grade::Good) <= due(Grade::Easy));
    }

    #[test]
    fn later_grade_preserves_introduced_on() {
        let intro = date(2026, 8, 10);
        let prior = record(3.0, 5.0, date(2026, 8, 12), date(2026, 8, 14), intro);
        let today = date(2026, 8, 15);
        let sched = Scheduler::new();

        let r = sched.review(Some(&prior), Grade::Good, today).unwrap();
        assert_eq!(
            r.introduced_on, intro,
            "introduced_on is stamped once, never re-stamped"
        );
        assert_eq!(r.last_review, today);
        assert!(r.due > today);
    }

    #[test]
    fn again_on_a_seen_card_pins_due_today_and_keeps_introduced_on() {
        let intro = date(2026, 8, 10);
        let prior = record(20.0, 5.0, date(2026, 8, 1), date(2026, 8, 15), intro);
        let today = date(2026, 8, 15);
        let sched = Scheduler::new();

        let r = sched.review(Some(&prior), Grade::Again, today).unwrap();
        assert_eq!(
            r.due, today,
            "Again pins due = today so a re-drill survives a quit"
        );
        assert_eq!(r.last_review, today);
        assert_eq!(r.introduced_on, intro);
    }
}
