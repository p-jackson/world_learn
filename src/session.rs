//! Session & derivation — the runtime lifecycle the Review loop drives (spec §5.2, §5.4).
//!
//! Everything about a Card's lifecycle is *derived* from the persisted store plus
//! today's date, never stored (spec §5.2): which Cards are due, how many new Cards
//! today's cap still allows, and the order they enter. [`build_queue`] composes
//! those into the transient session queue; [`Session`] then walks that queue,
//! grading each Card through the [`Scheduler`], persisting atomically after every
//! grade, and requeueing an **Again** to the back until it passes.
//!
//! The derivation functions ([`status`], [`new_backlog`], [`new_allowance`],
//! [`due_cards`], [`build_queue`]) are pure over `(&ReviewState, &Deck, today)`,
//! so they unit-test without a `Store` or any I/O; [`Session`] adds the store and
//! the mutable queue on top.
//!
//! Lands one issue ahead of its first UI caller; see [`crate::store`] for the same
//! dead-code note.
#![allow(dead_code)]

use std::collections::VecDeque;

use anyhow::{Context, Result};
use jiff::civil::Date;

use crate::deck::{Card, Deck};
use crate::scheduler::{Grade, Scheduler};
use crate::store::{ReviewState, Store};

/// A Card's derived lifecycle status (spec §5.2) — never persisted. `new` if the
/// store has no record; else `due` when `due ≤ today`, else `scheduled`.
/// ("Learning" isn't a separate state — it's a seen Card sitting at `due = today`.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    New,
    Due,
    Scheduled,
}

/// Derived status of `code` today (spec §5.2).
#[must_use]
pub fn status(state: &ReviewState, code: &str, today: Date) -> Status {
    match state.cards.get(code) {
        None => Status::New,
        Some(r) if r.due <= today => Status::Due,
        Some(_) => Status::Scheduled,
    }
}

/// New backlog (spec §5.2): deck keys minus seen keys, in intro order. These are
/// the Cards that have never been graded, ready to be introduced up to the cap.
#[must_use]
pub fn new_backlog<'d>(state: &ReviewState, deck: &'d Deck) -> Vec<&'d Card> {
    deck.cards()
        .iter()
        .filter(|c| !state.cards.contains_key(&c.code))
        .collect()
}

/// New allowance remaining today (spec §5.2): `new_cards_per_day` minus the count
/// of Cards already introduced today. Saturates at 0 so the cap is never exceeded
/// even if the setting is lowered mid-day below the count already introduced.
#[must_use]
pub fn new_allowance(state: &ReviewState, today: Date) -> u32 {
    let introduced_today = state
        .cards
        .values()
        .filter(|r| r.introduced_on == today)
        .count();
    let introduced_today = u32::try_from(introduced_today).unwrap_or(u32::MAX);
    state
        .settings
        .new_cards_per_day
        .saturating_sub(introduced_today)
}

/// Due set (spec §5.2): seen Cards with `due ≤ today`, in intro order.
#[must_use]
pub fn due_cards<'d>(state: &ReviewState, deck: &'d Deck, today: Date) -> Vec<&'d Card> {
    deck.cards()
        .iter()
        .filter(|c| state.cards.get(&c.code).is_some_and(|r| r.due <= today))
        .collect()
}

/// The transient session queue (spec §5.2, §5.4): the due set followed by up to
/// the remaining allowance of new Cards, all in intro order. Rebuilt from the
/// store at each session start, so a lowered cap or a new day is reflected without
/// any stored session state.
#[must_use]
pub fn build_queue(state: &ReviewState, deck: &Deck, today: Date) -> Vec<String> {
    let allowance = usize::try_from(new_allowance(state, today)).unwrap_or(usize::MAX);
    due_cards(state, deck, today)
        .into_iter()
        .chain(new_backlog(state, deck).into_iter().take(allowance))
        .map(|c| c.code.clone())
        .collect()
}

/// One review session over a fixed `today` (spec §5.4). Owns the loaded store
/// state, the transient queue, and the [`Scheduler`]; drives the front→grade loop
/// the Review screen renders. A session spans one local day — `today` is captured
/// at [`Session::start`] and every grade stamps it.
pub struct Session<'a> {
    deck: &'a Deck,
    store: &'a Store,
    scheduler: Scheduler,
    today: Date,
    state: ReviewState,
    /// `ADM0_A3` codes still to review this session, front = current Card.
    queue: VecDeque<String>,
}

impl<'a> Session<'a> {
    /// Start a session: load the store and build today's queue (spec §5.2).
    pub fn start(deck: &'a Deck, store: &'a Store, today: Date) -> Result<Self> {
        let state = store.load().context("loading store to start session")?;
        let queue = build_queue(&state, deck, today).into();
        Ok(Self {
            deck,
            store,
            scheduler: Scheduler::new(),
            today,
            state,
            queue,
        })
    }

    /// Cards left to review (Agains still queued count until they pass).
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.queue.len()
    }

    /// Whether the session queue is drained — the done-for-today state (spec §4.5).
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.queue.is_empty()
    }

    /// The Card now at the front of the queue, or `None` when done. Resolves the
    /// queued code against the Deck.
    #[must_use]
    pub fn current(&self) -> Option<&Card> {
        self.queue.front().and_then(|code| self.deck.get(code))
    }

    /// The persisted state as it stands (for derived Home-screen counts).
    #[must_use]
    pub const fn state(&self) -> &ReviewState {
        &self.state
    }

    /// Grade the current Card and advance the session (spec §5.4).
    ///
    /// Advances FSRS state, updates the record, and persists atomically. **Again**
    /// requeues the Card to the back (it re-drills, its persisted `due = today`
    /// surviving a quit); a pass exits it. A first grade creates the record and
    /// counts it against today's new-Card cap from that moment.
    pub fn grade(&mut self, grade: Grade) -> Result<()> {
        let code = self
            .queue
            .pop_front()
            .context("grading with an empty session queue")?;

        let record = self
            .scheduler
            .review(self.state.cards.get(&code), grade, self.today)
            .with_context(|| format!("grading card {code}"))?;
        self.state.cards.insert(code.clone(), record);
        self.store
            .save(&self.state)
            .with_context(|| format!("persisting graded card {code}"))?;

        if grade.is_again() {
            self.queue.push_back(code);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{CardRecord, Settings};
    use jiff::civil::{date, Date};
    use jiff::ToSpan;
    use std::collections::BTreeMap;

    const TODAY: Date = date(2026, 8, 15);

    fn deck() -> Deck {
        Deck::load().unwrap()
    }

    /// A store over a fresh temp dir, plus the tempdir guard (kept alive by the
    /// caller so the directory outlives the store).
    fn temp_store() -> (Store, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open_in(dir.path()).unwrap();
        (store, dir)
    }

    /// The first `n` deck codes in intro order — the ones a fresh session introduces.
    fn first_codes(deck: &Deck, n: usize) -> Vec<String> {
        deck.cards()
            .iter()
            .take(n)
            .map(|c| c.code.clone())
            .collect()
    }

    fn record(due: Date, last_review: Date, introduced_on: Date) -> CardRecord {
        CardRecord {
            stability: 5.0,
            difficulty: 5.0,
            due,
            last_review,
            introduced_on,
        }
    }

    fn state_with(new_cards_per_day: u32, cards: BTreeMap<String, CardRecord>) -> ReviewState {
        ReviewState {
            schema_version: crate::store::SCHEMA_VERSION,
            settings: Settings { new_cards_per_day },
            cards,
        }
    }

    #[test]
    fn status_derives_new_due_scheduled() {
        let deck = deck();
        let code = &deck.cards()[0].code;
        let mut cards = BTreeMap::new();
        cards.insert(code.clone(), record(TODAY, TODAY, TODAY)); // due ≤ today
        let state = state_with(10, cards);

        assert_eq!(status(&state, code, TODAY), Status::Due);
        assert_eq!(status(&state, "ZZZ", TODAY), Status::New); // absent key
                                                               // Same record, evaluated the day before it's due → scheduled.
        assert_eq!(status(&state, code, TODAY - 1.days()), Status::Scheduled);
    }

    #[test]
    fn new_allowance_counts_only_today_introductions() {
        let deck = deck();
        let codes = first_codes(&deck, 3);
        let mut cards = BTreeMap::new();
        // Two introduced today, one introduced earlier.
        cards.insert(codes[0].clone(), record(TODAY + 3.days(), TODAY, TODAY));
        cards.insert(codes[1].clone(), record(TODAY + 3.days(), TODAY, TODAY));
        cards.insert(
            codes[2].clone(),
            record(TODAY + 3.days(), TODAY, TODAY - 5.days()),
        );
        let state = state_with(10, cards);

        // 10 cap − 2 introduced today = 8; the older introduction doesn't count.
        assert_eq!(new_allowance(&state, TODAY), 8);
    }

    #[test]
    fn build_queue_is_due_then_new_capped_in_intro_order() {
        let deck = deck();
        // Make the 5th deck card due; leave the rest new. Cap new at 2.
        let due_code = deck.cards()[4].code.clone();
        let mut cards = BTreeMap::new();
        cards.insert(
            due_code.clone(),
            record(TODAY, TODAY - 2.days(), TODAY - 2.days()),
        );
        let state = state_with(2, cards);

        let queue = build_queue(&state, &deck, TODAY);

        // Due card leads; then exactly `allowance` new cards in intro order.
        assert_eq!(queue[0], due_code, "due set precedes new cards");
        assert_eq!(queue.len(), 3, "1 due + 2 new (cap)");
        // The two new cards are the first backlog entries in intro order (skipping
        // the one already-seen due card).
        let new_tail: Vec<&String> = queue[1..].iter().collect();
        let expected: Vec<String> = deck
            .cards()
            .iter()
            .map(|c| c.code.clone())
            .filter(|c| *c != due_code)
            .take(2)
            .collect();
        assert_eq!(new_tail, expected.iter().collect::<Vec<_>>());
    }

    #[test]
    fn again_requeues_to_back_and_persists_due_today() {
        let deck = deck();
        let (store, _guard) = temp_store();
        store.save(&state_with(2, BTreeMap::new())).unwrap();

        let mut session = Session::start(&deck, &store, TODAY).unwrap();
        let front = session.current().unwrap().code.clone();
        let started = session.remaining();

        session.grade(Grade::Again).unwrap();

        // Requeued to the back: same count, front advanced, original now last.
        assert_eq!(
            session.remaining(),
            started,
            "Again keeps the card in the queue"
        );
        assert_ne!(
            session.current().unwrap().code,
            front,
            "front advanced past the re-drill"
        );
        assert_eq!(
            session.queue.back().unwrap(),
            &front,
            "re-drill sits at the back"
        );

        // Persisted due = today survives a quit — reload proves it.
        let reloaded = store.load().unwrap();
        assert_eq!(reloaded.cards[&front].due, TODAY);
        assert_eq!(reloaded.cards[&front].introduced_on, TODAY);
    }

    #[test]
    fn pass_exits_the_card_and_schedules_a_future_due() {
        let deck = deck();
        let (store, _guard) = temp_store();
        store.save(&state_with(2, BTreeMap::new())).unwrap();

        let mut session = Session::start(&deck, &store, TODAY).unwrap();
        let front = session.current().unwrap().code.clone();
        let started = session.remaining();

        session.grade(Grade::Good).unwrap();

        assert_eq!(
            session.remaining(),
            started - 1,
            "a pass drops the card from the queue"
        );
        assert!(!session.queue.contains(&front));

        let reloaded = store.load().unwrap();
        assert!(
            reloaded.cards[&front].due > TODAY,
            "a pass schedules a future due date"
        );
    }

    #[test]
    fn first_grade_of_a_new_card_stamps_introduced_on_today() {
        let deck = deck();
        let (store, _guard) = temp_store();
        store.save(&state_with(1, BTreeMap::new())).unwrap();

        let mut session = Session::start(&deck, &store, TODAY).unwrap();
        let code = session.current().unwrap().code.clone();
        assert_eq!(status(session.state(), &code, TODAY), Status::New);

        session.grade(Grade::Hard).unwrap();

        let reloaded = store.load().unwrap();
        assert_eq!(reloaded.cards[&code].introduced_on, TODAY);
    }

    /// The daily new-Card cap resets at the local day boundary (spec §5.4): new
    /// introductions are exhausted for `today` but a later day frees the cap again.
    #[test]
    fn new_cap_is_enforced_within_a_day_and_resets_across_the_boundary() {
        let deck = deck();
        let (store, _guard) = temp_store();
        store.save(&state_with(3, BTreeMap::new())).unwrap();

        // Day 1: introduce and pass all 3 allowed new cards.
        let mut day1 = Session::start(&deck, &store, TODAY).unwrap();
        assert_eq!(day1.remaining(), 3, "cap of 3 admits 3 new cards");
        let introduced: Vec<String> = (0..3)
            .map(|_| {
                let code = day1.current().unwrap().code.clone();
                day1.grade(Grade::Good).unwrap();
                code
            })
            .collect();

        // Same day, fresh session: cap is spent, no new cards, and none of the 3
        // passes are due yet → nothing to do.
        let same_day = Session::start(&deck, &store, TODAY).unwrap();
        assert_eq!(new_allowance(same_day.state(), TODAY), 0);
        assert_eq!(
            same_day.remaining(),
            0,
            "the day's new-card cap is exhausted"
        );

        // Next day: cap resets; a fresh batch of new cards (none of the day-1 three)
        // is admitted.
        let tomorrow = TODAY + 1.days();
        let day2 = Session::start(&deck, &store, tomorrow).unwrap();
        assert_eq!(
            new_allowance(day2.state(), tomorrow),
            3,
            "the cap resets at the day boundary"
        );
        let day2_new: Vec<String> = new_backlog(day2.state(), &deck)
            .into_iter()
            .take(3)
            .map(|c| c.code.clone())
            .collect();
        assert_eq!(day2_new.len(), 3);
        for code in &day2_new {
            assert!(
                !introduced.contains(code),
                "day 2 introduces cards not seen on day 1"
            );
        }
    }
}
