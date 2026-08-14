# Deck-derivation & introduction-order rules

Type: grilling
Status: resolved
Blocked by: 03

## Question

Given what the geometry pipeline (03) yields, decide the rules that turn the
dataset into the Deck — as *rules*, not a hand-typed 250-row list.

- **Inclusion rule**: which Natural Earth features are in vs out. All admin-0?
  A minimum-area threshold to drop unguessable specks? How are the contested
  entities (Kosovo, Western Sahara, N. Cyprus, Somaliland, Taiwan, Palestine)
  explicitly handled — the whole point is to include them.
- **Display name**: which name each Entity shows on reveal (dataset field vs a
  curated override for awkward long-form names).
- **Introduction order**: the concrete rule for big→obscure (by area?
  population? a curated tier list?), producing the fixed sequence new cards enter.

Output: the deck-derivation rules the spec will state.

## Answer

The Deck is derived from NE 50m admin-0 (241 features) by three rules. Key
every Entity on **`ADM0_A3`** (per ticket 03).

### Inclusion rule — 239 Cards

**Include every feature EXCEPT uninhabited dependencies.** Concretely:

- `TYPE ∈ {Sovereign country, Country, Disputed, Indeterminate}` → **211** (all
  sovereign states, semi-independent constituents like Greenland, and all 6
  contested entities — Kosovo, W. Sahara, N. Cyprus, Somaliland, Taiwan,
  Palestine, whose `TYPE` is inconsistent so they ride in on these classes, not
  on a `TYPE="Disputed"` filter).
- **PLUS** `TYPE = Dependency AND POP_EST > 0` → **28** of the 30 dependencies.
- **Drop only** the two POP_EST-0 dependencies: **Heard I. & McDonald Is.** and
  **Ashmore & Cartier Is.**

Total = **211 + 28 = 239 Cards.** Criterion is **inhabited**, not "big enough to
locate" — tiny-but-inhabited entities (Niue, Anguilla, Pitcairn, Bermuda) are in;
making sub-dot shapes visible in their regional frame is deferred to the build
phase, not an inclusion filter. `POP_EST > 0` is a single data-derived rule, no
hand-curated allow/deny list. (The 3 "staff-only" deps — BIOT/Diego Garcia,
Fr. S. Antarctic Lands, S. Georgia — stay in; excluding them would need exactly
the judgment list we're avoiding.) Small *sovereign* states (Vatican, Nauru,
Tuvalu) are always in regardless of size. Note: no area field exists in NE, so
any area needed downstream is computed from geometry — but inclusion needs none.

### Display name

Reveal shows **curated common name (primary) + `NAME_LONG` (formal, secondary)**
— matches ticket 04's "short + formal". The common name is derived from NE
`NAME` by rule, with a **small curated override table (~15–25 entries)** fixing
NE's abbreviations to natural forms (`W. Sahara`→Western Sahara, `Dem. Rep.
Congo`→DR Congo, `N. Cyprus`→Northern Cyprus, `Bosnia and Herz.`→Bosnia &
Herzegovina, …). Rules + overrides, never a hand-typed 239-row list. Grading is
self-assessed, so the displayed string never gates correctness — it only has to
read naturally.

### Introduction order

The fixed big→obscure sequence new cards enter = **sort by `LABELRANK`
ascending (lower = more prominent), tiebreak `POP_EST` descending.** "Big" =
recognizable, not largest-area: LABELRANK is NE's own curated label-prominence
signal (populated for all entities incl. contested), so famous big countries
lead and lesser-known ones trail; POP_EST orders within each LABELRANK band.
Contested entities land mid-to-late (Taiwan LABELRANK 3; Palestine/Somaliland 5;
Kosovo/N.Cyprus 6; W. Sahara 7), which is the desired feel. (Rejected: pure
POP_EST desc — buries famous-small nations, front-loads high-pop/low-recognition
ones. Rejected for inclusion: `MIN_ZOOM` cutoff — would drop Palestine at 7.0.)
