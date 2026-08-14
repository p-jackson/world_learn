// Expected Deck size (spec §2.1): the inclusion rule over NE 50m yields exactly
// this many entities. The build guard and the tests both assert against it.
export const DECK_COUNT = 239;

// Curated common-name overrides (spec §2.3).
//
// Common name = Natural Earth `NAME` by default. NE abbreviates a chunk of
// names ("W. Sahara", "Dem. Rep. Congo") in a way that reads badly on the
// reveal screen. This is the small curated table (~15-25 entries) that fixes
// those to natural forms. Keyed by ADM0_A3 (stable; NAME strings are not).
//
// Formal/long name on the reveal's second line comes from NE `NAME_LONG`
// unchanged — this table only touches the primary common name.
export const COMMON_NAME_OVERRIDES = {
  COD: 'DR Congo',
  SAH: 'Western Sahara',
  CYN: 'Northern Cyprus',
  BIH: 'Bosnia & Herzegovina',
  CAF: 'Central African Republic',
  DOM: 'Dominican Republic',
  GNQ: 'Equatorial Guinea',
  SDS: 'South Sudan',
  COK: 'Cook Islands',
  CYM: 'Cayman Islands',
  FLK: 'Falkland Islands',
  FRO: 'Faroe Islands',
  MHL: 'Marshall Islands',
  MNP: 'Northern Mariana Islands',
  PCN: 'Pitcairn Islands',
  PYF: 'French Polynesia',
  SLB: 'Solomon Islands',
  TCA: 'Turks & Caicos Islands',
  VGB: 'British Virgin Islands',
  VIR: 'U.S. Virgin Islands',
  WLF: 'Wallis & Futuna',
  ATG: 'Antigua & Barbuda',
  SGS: 'South Georgia & the Sandwich Islands',
  SPM: 'Saint Pierre & Miquelon',
};

// Feature TYPEs that ride into the Deck unconditionally (spec §2.1). Contested
// entities (Kosovo, W. Sahara, N. Cyprus, Somaliland, Taiwan, Palestine) carry
// inconsistent TYPEs, so they enter via these classes — not a "Disputed" filter.
export const INCLUDED_TYPES = new Set([
  'Sovereign country',
  'Country',
  'Disputed',
  'Indeterminate',
]);

// West Bank / Gaza sample points, used only to detect whether the source folds
// Palestine into Israel (spec §2.2). If the base Israel polygon contains any of
// these, the source is "merged" and the `pse` point-of-view swap is applied.
export const PALESTINE_PROBES = {
  Ramallah: [35.2, 31.9],
  Nablus: [35.26, 32.22],
  Gaza: [34.45, 31.5],
};
