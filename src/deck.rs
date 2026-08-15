//! The in-memory Deck the rest of the app iterates (spec §2, §5.2).
//!
//! Loads the static geometry asset (`assets/geometry.json`, built by
//! `tools/geometry`) once at runtime and exposes its Entities as Cards in the
//! fixed big→obscure introduction order: `LABELRANK` ascending, tiebreak
//! `POP_EST` descending (§2.4). New Cards always enter in this order, and every
//! screen shares the one ordering.
//!
//! Deck membership and order are derived purely from the asset — nothing here is
//! persisted (§5.2). The asset is already filtered to exactly the Deck features
//! by the build pipeline (issue 01), so loading is: parse, sort, index.
//!
//! Like [`crate::store`], this module lands ahead of its first UI caller, so its
//! public surface reads as dead code until the app shell consumes it. Allowed
//! module-wide; drop the allow once a screen iterates the Deck.
#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};
use std::ops::Deref;
use std::rc::Rc;

use anyhow::{Context, Result};
use serde::Deserialize;

/// The geometry asset, embedded at compile time. Parsed once by [`Deck::load`].
const GEOMETRY_JSON: &str = include_str!("../assets/geometry.json");

/// One map feature's static data, as emitted by the geometry pipeline (§3.3).
/// Keyed externally by its `ADM0_A3` code (see [`Card::code`]); the code is not
/// a field here because the asset stores it as the map key.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Entity {
    /// Curated common name — the reveal's primary line (§2.3).
    pub name: String,
    /// Formal `NAME_LONG` — the reveal's secondary line (§2.3).
    pub name_long: String,
    /// SVG path in projected coordinates (§3.3), rendered by the map component.
    pub d: String,
    /// `[minx, miny, maxx, maxy]` in projected coordinates; drives framing (§4.2).
    pub bbox: [f64; 4],
    /// NE label-prominence signal, lower = more prominent — primary sort key (§2.4).
    pub labelrank: u32,
    /// NE population estimate — sort tiebreak (§2.4).
    pub pop_est: u64,
    /// `[x, y]` in projected coordinates; the reveal drops its pin here (§4.1).
    pub centroid: [f64; 2],
}

/// An Entity paired with its `ADM0_A3` code — the unit the Deck orders and the
/// scheduler tracks (one Card per Entity in the MVP; see `CONTEXT.md`).
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    /// `ADM0_A3` code — the key the store and geometry asset share.
    pub code: String,
    pub entity: Entity,
}

/// The full set of Cards in fixed introduction order (§2.4). Build once with
/// [`Deck::load`] and share it; membership and order are derived from the asset
/// alone and never persisted (§5.2).
pub struct Deck {
    /// Cards in intro order — `LABELRANK` asc, tiebreak `POP_EST` desc.
    cards: Vec<Card>,
    /// `ADM0_A3` → position in [`Self::cards`], for O(1) lookup.
    index: HashMap<String, usize>,
}

impl Deck {
    /// Parse the embedded geometry asset into the ordered Deck.
    pub fn load() -> Result<Self> {
        let entities: BTreeMap<String, Entity> =
            serde_json::from_str(GEOMETRY_JSON).context("parsing embedded geometry asset")?;
        Ok(Self::build(entities))
    }

    /// Sort into intro order and index by code. Takes a `BTreeMap` so the input
    /// is code-sorted: [`Vec::sort_by`] is stable, so Cards that tie on both
    /// sort keys keep that alphabetical-by-code order, making the sequence fully
    /// deterministic rather than dependent on parse order.
    fn build(entities: BTreeMap<String, Entity>) -> Self {
        let mut cards: Vec<Card> = entities
            .into_iter()
            .map(|(code, entity)| Card { code, entity })
            .collect();
        // §2.4: LABELRANK ascending (lower = more prominent), tiebreak POP_EST
        // descending.
        cards.sort_by(|a, b| {
            a.entity
                .labelrank
                .cmp(&b.entity.labelrank)
                .then_with(|| b.entity.pop_est.cmp(&a.entity.pop_est))
        });
        let index = cards
            .iter()
            .enumerate()
            .map(|(i, c)| (c.code.clone(), i))
            .collect();
        Self { cards, index }
    }

    /// Number of Cards in the Deck.
    pub const fn len(&self) -> usize {
        self.cards.len()
    }

    /// Whether the Deck is empty (never true for the shipped asset).
    pub const fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// The Cards in fixed introduction order (§2.4).
    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    /// The Card for `code`, or `None` if the code is not in the Deck.
    pub fn get(&self, code: &str) -> Option<&Card> {
        self.index.get(code).map(|&i| &self.cards[i])
    }

    /// Whether `code` is a Deck member.
    pub fn contains(&self, code: &str) -> bool {
        self.index.contains_key(code)
    }
}

/// A shared, immutable [`Deck`] cheap enough to pass as a prop or own in session
/// state: an `Rc` compared by pointer identity. The Deck is built once and never
/// mutated, so identity equality is the right prop-diff — it keeps consumers like
/// [`crate::map::WorldMap`] from re-rendering when only an unrelated signal changed.
#[derive(Clone)]
pub struct SharedDeck(Rc<Deck>);

impl SharedDeck {
    #[must_use]
    pub fn new(deck: Deck) -> Self {
        Self(Rc::new(deck))
    }
}

impl PartialEq for SharedDeck {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Deref for SharedDeck {
    type Target = Deck;

    fn deref(&self) -> &Deck {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(labelrank: u32, pop_est: u64) -> Entity {
        Entity {
            name: "Test".to_string(),
            name_long: "Test Republic".to_string(),
            d: "M0,0Z".to_string(),
            bbox: [0.0, 0.0, 1.0, 1.0],
            labelrank,
            pop_est,
            centroid: [0.5, 0.5],
        }
    }

    /// Position of `code` in the Deck's intro order.
    fn pos(deck: &Deck, code: &str) -> usize {
        deck.cards()
            .iter()
            .position(|c| c.code == code)
            .unwrap_or_else(|| panic!("{code} missing from deck"))
    }

    #[test]
    fn loads_the_full_deck() {
        let deck = Deck::load().unwrap();
        // 240, not 239: the rule over NE 50m yields 239; Tuvalu is too small for
        // 50m and is supplemented from 10m (§2.1, issue 01). Kept in lockstep
        // with the asset by store.rs's snapshot tripwire.
        assert_eq!(deck.len(), 240);
    }

    #[test]
    fn ordered_by_labelrank_then_pop_desc() {
        let entities = BTreeMap::from([
            // Same labelrank, differing pop → higher pop first.
            ("AAA".to_string(), entity(2, 100)),
            ("BBB".to_string(), entity(2, 900)),
            // Lower labelrank leads regardless of pop.
            ("CCC".to_string(), entity(1, 1)),
            // Higher labelrank trails regardless of pop.
            ("DDD".to_string(), entity(5, 9_999)),
        ]);
        let deck = Deck::build(entities);
        let order: Vec<&str> = deck.cards().iter().map(|c| c.code.as_str()).collect();
        assert_eq!(order, ["CCC", "BBB", "AAA", "DDD"]);
    }

    #[test]
    fn full_ties_break_alphabetically_by_code() {
        // Identical sort keys → stable sort keeps code order, so the sequence is
        // deterministic rather than parse-order dependent.
        let entities = BTreeMap::from([
            ("ZZZ".to_string(), entity(3, 50)),
            ("AAA".to_string(), entity(3, 50)),
            ("MMM".to_string(), entity(3, 50)),
        ]);
        let deck = Deck::build(entities);
        let order: Vec<&str> = deck.cards().iter().map(|c| c.code.as_str()).collect();
        assert_eq!(order, ["AAA", "MMM", "ZZZ"]);
    }

    #[test]
    fn real_deck_applies_the_pop_tiebreak() {
        // China and India share LABELRANK 2; China's larger population puts it
        // first — the tiebreak exercised against the shipped asset.
        let deck = Deck::load().unwrap();
        assert!(pos(&deck, "CHN") < pos(&deck, "IND"));
    }

    #[test]
    fn contested_entities_land_mid_to_late() {
        let deck = Deck::load().unwrap();

        // Data tripwire: the intro-order feel (§2.4) rests on these LABELRANKs.
        let lr = |code| deck.get(code).unwrap().entity.labelrank;
        assert_eq!(lr("TWN"), 3, "Taiwan");
        assert_eq!(lr("PSX"), 5, "Palestine");
        assert_eq!(lr("SOL"), 5, "Somaliland");
        assert_eq!(lr("KOS"), 6, "Kosovo");
        assert_eq!(lr("CYN"), 6, "Northern Cyprus");
        assert_eq!(lr("SAH"), 7, "Western Sahara");

        // None rides in with the famous leading pack: all land past the first
        // fifth of the Deck.
        let lead = deck.len() / 5;
        for code in ["TWN", "PSX", "SOL", "KOS", "CYN", "SAH"] {
            assert!(
                pos(&deck, code) > lead,
                "{code} at {} should trail the leading pack (>{lead})",
                pos(&deck, code)
            );
        }

        // Relative order follows the LABELRANK buckets: Taiwan before the LR5
        // pair, which precede the LR6 pair, which precede Western Sahara (LR7).
        assert!(pos(&deck, "TWN") < pos(&deck, "PSX"));
        assert!(pos(&deck, "TWN") < pos(&deck, "SOL"));
        assert!(pos(&deck, "PSX") < pos(&deck, "KOS"));
        assert!(pos(&deck, "SOL") < pos(&deck, "KOS"));
        assert!(pos(&deck, "KOS") < pos(&deck, "SAH"));
        assert!(pos(&deck, "CYN") < pos(&deck, "SAH"));
    }

    #[test]
    fn famous_small_nations_are_not_buried_by_pure_population() {
        // §2.4's rejected alternative: pure POP_EST-desc "buries famous-small
        // nations". LABELRANK rescues them. Iceland is the clean case against the
        // shipped asset — a genuinely small nation (~340k) that a pure-population
        // sort drops deep into the tail, but LABELRANK surfaces into the Deck's
        // leading half. New Zealand rides along as a second famous nation
        // pure-pop pushes far down (its population isn't small, but the rescue
        // is the same shape). NB the issue's literal example, Vatican/Nauru, does
        // NOT hold: both carry LABELRANK 6, so LABELRANK buries them at the tail
        // just as hard as population would — they are not a valid rescue case.
        let deck = Deck::load().unwrap();

        let mut by_pop: Vec<&Card> = deck.cards().iter().collect();
        by_pop.sort_by_key(|c| std::cmp::Reverse(c.entity.pop_est));
        let pop_pos = |code: &str| {
            by_pop
                .iter()
                .position(|c| c.code == code)
                .unwrap_or_else(|| panic!("{code} missing"))
        };

        let half = deck.len() / 2;
        for code in ["ISL", "NZL"] {
            let intro = pos(&deck, code);
            assert!(
                intro < half,
                "{code} at {intro} should sit in the Deck's leading half (<{half})"
            );
            assert!(
                intro < pop_pos(code),
                "{code}: LABELRANK order ({intro}) should surface it earlier than pure-pop ({})",
                pop_pos(code)
            );
        }
    }

    #[test]
    fn shared_deck_compares_by_identity() {
        let a = SharedDeck::new(Deck::load().unwrap());
        let b = a.clone();
        // Clones share the Rc → equal; an independently loaded Deck is not.
        // (`assert!` rather than `assert_ne!` so `SharedDeck` needn't impl Debug.)
        assert!(a == b);
        assert!(a != SharedDeck::new(Deck::load().unwrap()));
        // Deref reaches the Deck's own surface.
        assert_eq!(a.len(), 240);
    }

    #[test]
    fn lookup_resolves_every_card_to_its_own_position() {
        let deck = Deck::load().unwrap();
        assert_eq!(deck.get("FRA").unwrap().entity.name, "France");
        assert!(deck.contains("FRA"));
        assert!(deck.get("ZZZ").is_none());
        assert!(!deck.contains("ZZZ"));

        // Every code resolves back to the same Card object (no collisions in the
        // index, no code owned by two positions).
        for (i, card) in deck.cards().iter().enumerate() {
            assert_eq!(pos(&deck, &card.code), i, "{} indexed twice", card.code);
            assert_eq!(deck.get(&card.code).unwrap(), card);
        }
    }
}
