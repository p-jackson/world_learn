# Context

Glossary for the country-location flashcard app. A glossary only — no
implementation detail.

## Terms

### Entity

The guessable unit: one feature on the world map — a sovereign state, a
dependency, or a disputed territory. Drawn from Natural Earth admin-0; the MVP
Deck is the **240** that are inhabited (every sovereign state and contested
territory, plus dependencies that have residents — only the uninhabited
dependency specks are left out; Tuvalu, too small for NE 50m, is sourced from 10m). Deliberately broader than "country": contested
and dependent territories are in, on purpose. Prefer **Entity** over "country"
or "territory" when the distinction matters.

### Card

A pairing of an Entity with the locate-and-name prompt. The unit the scheduler
tracks. One Card per Entity in the MVP.

### Review

One presentation of a Card: a regional-zoom map with the Entity highlighted,
the learner recalls the name mentally (self-reveal), taps to reveal the true
name, then self-grades on the 4-button scale (Again / Hard / Good / Easy).

### Deck

The full set of Cards derived from the Entity set.

### Regional zoom

The Review's framing: the map viewport is the Entity's bounding box expanded by
a padding factor, so immediate neighbours show as location cues — not the whole
world, not an isolated silhouette.
