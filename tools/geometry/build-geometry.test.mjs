import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  isDeckFeature,
  commonName,
  largestPolygon,
  framingPolygons,
  framingBbox,
  projectBounds,
  framingCentroid,
  isPalestineMerged,
  applyPalestineSwap,
  missingSupplements,
  applySupplements,
  buildAsset,
} from './lib.mjs';
import { DECK_COUNT } from './overrides.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const ASSET = join(HERE, '..', '..', 'assets', 'geometry.json');

// --- fixtures ---------------------------------------------------------------
// A closed square ring centred at (cx,cy), half-width h. Wound so d3's spherical
// interior is the small square (not its global complement).
const ring = (cx, cy, h) => [
  [cx - h, cy - h],
  [cx - h, cy + h],
  [cx + h, cy + h],
  [cx + h, cy - h],
  [cx - h, cy - h],
];
const square = (cx, cy, h) => ({ type: 'Polygon', coordinates: [ring(cx, cy, h)] });
const feature = (props, geometry) => ({ type: 'Feature', properties: props, geometry });

// A clean, already-separated Israel + Palestine pair — drop into a base so the
// Palestine guard stays on the no-swap path when a test is about something else.
const cleanPalestine = () => [
  feature({ ADM0_A3: 'ISR', TYPE: 'Country', NAME: 'Israel', NAME_LONG: 'Israel', LABELRANK: 4, POP_EST: 1 }, square(34.8, 31.5, 0.3)),
  feature({ ADM0_A3: 'PSX', TYPE: 'Disputed', NAME: 'Palestine', NAME_LONG: 'Palestine', LABELRANK: 5, POP_EST: 1 }, square(35.2, 31.9, 0.2)),
];

// --- inclusion rule (§2.1) --------------------------------------------------
test('isDeckFeature: sovereign/country/disputed/indeterminate ride in', () => {
  for (const TYPE of ['Sovereign country', 'Country', 'Disputed', 'Indeterminate']) {
    assert.equal(isDeckFeature({ TYPE, POP_EST: 0 }), true, TYPE);
  }
});

test('isDeckFeature: inhabited dependency in, uninhabited dependency out', () => {
  assert.equal(isDeckFeature({ TYPE: 'Dependency', POP_EST: 1 }), true);
  assert.equal(isDeckFeature({ TYPE: 'Dependency', POP_EST: 0 }), false);
});

// --- common name (§2.3) -----------------------------------------------------
test('commonName: curated override wins, else NE NAME', () => {
  assert.equal(commonName({ ADM0_A3: 'COD', NAME: 'Dem. Rep. Congo' }), 'DR Congo');
  assert.equal(commonName({ ADM0_A3: 'FRA', NAME: 'France' }), 'France');
});

// --- mainland selection & bbox (§4.2) --------------------------------------
test('largestPolygon: picks the biggest part of a MultiPolygon', () => {
  const geom = {
    type: 'MultiPolygon',
    coordinates: [square(0, 0, 5).coordinates, square(100, -40, 0.5).coordinates],
  };
  assert.deepEqual(largestPolygon(geom), square(0, 0, 5));
});

test('framingPolygons: a dominant mainland frames alone (USA + Alaska shape)', () => {
  // One polygon holds the majority of the area, so distant exclaves are dropped
  // and the frame never blows out across an ocean.
  const geom = {
    type: 'MultiPolygon',
    coordinates: [square(0, 0, 5).coordinates, square(-150, 30, 2).coordinates],
  };
  assert.deepEqual(framingPolygons(geom), [square(0, 0, 5)]);
});

test('framingPolygons: a dominant-less archipelago unions its comparable islands', () => {
  // Three equal islands, none a majority: all frame together so the window
  // covers the whole nation (Indonesia: Sumatra…Papua), not one island.
  const geom = {
    type: 'MultiPolygon',
    coordinates: [
      square(0, 0, 3).coordinates,
      square(20, 0, 3).coordinates,
      square(40, 0, 3).coordinates,
    ],
  };
  assert.equal(framingPolygons(geom).length, 3);
});

test('framingPolygons: an archipelago still drops islands an order of magnitude smaller', () => {
  // Two comparable islands (neither a majority) plus a speck. The speck is > 10×
  // smaller than the largest, so it is excluded and can't blow out the bbox.
  const geom = {
    type: 'MultiPolygon',
    coordinates: [
      square(0, 0, 3).coordinates,
      square(10, 0, 3).coordinates,
      square(60, 0, 0.3).coordinates,
    ],
  };
  const framed = framingPolygons(geom);
  assert.equal(framed.length, 2);
  const [minx, , maxx] = framingBbox(geom);
  assert.ok(minx > -4 && maxx < 14, `frames the two big islands, not the speck (${minx}..${maxx})`);
});

test('framingBbox: an archipelago spans all its major islands', () => {
  // Islands at lon 0 and lon 40 (equal area): the framing bbox reaches both.
  const geom = {
    type: 'MultiPolygon',
    coordinates: [square(0, 0, 3).coordinates, square(40, 0, 3).coordinates],
  };
  const [minx, , maxx] = framingBbox(geom);
  assert.ok(minx < -2, `left island included (${minx})`);
  assert.ok(maxx > 42, `right island included (${maxx})`);
});

test('framingBbox: ignores the far-flung small part (France + Guiana shape)', () => {
  const geom = {
    type: 'MultiPolygon',
    coordinates: [square(0, 0, 5).coordinates, square(100, -40, 0.5).coordinates],
  };
  // mainland square lon[-5,5] lat[-5,5] -> projected y flips (symmetric here);
  // spherical edges bulge a hair, so assert closeness, not equality.
  const [minx, miny, maxx, maxy] = framingBbox(geom);
  for (const [got, want] of [[minx, -5], [miny, -5], [maxx, 5], [maxy, 5]]) {
    assert.ok(Math.abs(got - want) < 0.1, `${got} ≈ ${want}`);
  }
  assert.ok(maxx < 50, 'far-flung part excluded'); // maxx≈5, not ≈100
});

test('projectBounds: y is flipped (y = -lat)', () => {
  assert.deepEqual(projectBounds([[10, 20], [30, 40]]), [10, -40, 30, -20]);
});

test('framingCentroid: projected & inside the mainland part', () => {
  const geom = {
    type: 'MultiPolygon',
    coordinates: [square(20, 10, 5).coordinates, square(120, -50, 0.5).coordinates],
  };
  const [x, y] = framingCentroid(geom);
  assert.ok(Math.abs(x - 20) < 0.5, `x≈20 got ${x}`);
  assert.ok(Math.abs(y - -10) < 0.5, `y≈-10 got ${y}`); // lat 10 -> y -10
});

// --- Palestine handling (§2.2) ---------------------------------------------
test('isPalestineMerged: false when Israel excludes the West Bank', () => {
  const features = [
    feature({ ADM0_A3: 'ISR' }, square(34.8, 31.5, 0.3)), // west of the probes
    feature({ ADM0_A3: 'PSX' }, square(35.2, 31.9, 0.2)),
  ];
  assert.equal(isPalestineMerged(features), false);
});

test('isPalestineMerged: true when Israel covers a probe point', () => {
  const features = [
    feature({ ADM0_A3: 'ISR' }, square(35.2, 31.9, 0.5)), // contains Ramallah probe
    feature({ ADM0_A3: 'PSX' }, square(35.2, 31.9, 0.1)),
  ];
  assert.equal(isPalestineMerged(features), true);
});

test('isPalestineMerged: true when there is no Palestine feature', () => {
  const features = [feature({ ADM0_A3: 'ISR' }, square(34.8, 31.5, 0.3))];
  assert.equal(isPalestineMerged(features), true);
});

test('applyPalestineSwap: swaps ISR+PSX geometry, preserves attributes & others', () => {
  const base = [
    feature({ ADM0_A3: 'ISR', TYPE: 'Country' }, square(0, 0, 1)),
    feature({ ADM0_A3: 'PSX', TYPE: 'Disputed' }, square(0, 0, 1)),
    feature({ ADM0_A3: 'JOR', TYPE: 'Sovereign country' }, square(9, 9, 1)),
  ];
  const pseIsr = square(2, 2, 1);
  const psePsx = square(3, 3, 1);
  const out = applyPalestineSwap(base, [
    feature({ ADM0_A3: 'ISR' }, pseIsr),
    feature({ ADM0_A3: 'PSX' }, psePsx),
  ]);
  const by = (a3) => out.find((f) => f.properties.ADM0_A3 === a3);
  assert.deepEqual(by('ISR').geometry, pseIsr);
  assert.deepEqual(by('PSX').geometry, psePsx);
  assert.equal(by('ISR').properties.TYPE, 'Country'); // attributes preserved
  assert.deepEqual(by('JOR').geometry, square(9, 9, 1)); // untouched
});

// --- buildAsset orchestration ----------------------------------------------
test('buildAsset: clean source never touches the pse loader', async () => {
  const baseGeo = {
    features: [
      feature(
        { ADM0_A3: 'ISR', TYPE: 'Country', NAME: 'Israel', NAME_LONG: 'Israel', LABELRANK: 4, POP_EST: 1 },
        square(34.8, 31.5, 0.3),
      ),
      feature(
        { ADM0_A3: 'PSX', TYPE: 'Disputed', NAME: 'Palestine', NAME_LONG: 'Palestine', LABELRANK: 5, POP_EST: 1 },
        square(35.2, 31.9, 0.2),
      ),
    ],
  };
  const { asset, report } = await buildAsset(baseGeo, {
    supplements: [],
    loadPse: () => {
      throw new Error('loadPse must not be called for a clean source');
    },
  });
  assert.equal(report.palestine_handling, 'source-already-separated');
  assert.equal(report.deck_count, 2);
  assert.ok('PSX' in asset);
  assert.equal(asset.ISR.name, 'Israel');
  assert.match(asset.ISR.d, /^M/);
  assert.equal(asset.ISR.bbox.length, 4);
});

test('buildAsset: merged source pulls Palestine from the pse loader', async () => {
  const baseGeo = {
    features: [
      feature(
        { ADM0_A3: 'ISR', TYPE: 'Country', NAME: 'Israel', NAME_LONG: 'Israel', LABELRANK: 4, POP_EST: 1 },
        square(35.2, 31.9, 0.6), // covers the West Bank probes -> merged
      ),
    ],
  };
  const loadPse = () => [
    feature({ ADM0_A3: 'ISR' }, square(34.8, 31.5, 0.3)),
    feature(
      { ADM0_A3: 'PSX', TYPE: 'Disputed', NAME: 'Palestine', NAME_LONG: 'Palestine', LABELRANK: 5, POP_EST: 1 },
      square(35.2, 31.9, 0.2),
    ),
  ];
  const { asset, report } = await buildAsset(baseGeo, { supplements: [], loadPse });
  assert.equal(report.palestine_handling, 'pse-swap');
  assert.ok('PSX' in asset, 'Palestine introduced by the swap');
});

// --- supplements (§2.1 always-in small states absent from 50m) --------------
test('missingSupplements: only codes with no feature yet', () => {
  const features = [feature({ ADM0_A3: 'FRA' }, square(0, 0, 1))];
  assert.deepEqual(missingSupplements(features, ['TUV', 'FRA']), ['TUV']);
});

test('applySupplements: appends the named 10m entities', () => {
  const base = [feature({ ADM0_A3: 'FRA' }, square(0, 0, 1))];
  const tenM = [feature({ ADM0_A3: 'TUV', NAME: 'Tuvalu' }, square(179, -8, 0.1))];
  const out = applySupplements(base, tenM, ['TUV']);
  assert.equal(out.length, 2);
  assert.equal(out[1].properties.ADM0_A3, 'TUV');
});

test('applySupplements: throws when a code is absent from the 10m source', () => {
  assert.throws(() => applySupplements([], [], ['TUV']), /supplement TUV not found/);
});

test('buildAsset: pulls a missing supplement from the 10m loader', async () => {
  const baseGeo = {
    features: [
      ...cleanPalestine(),
      feature(
        { ADM0_A3: 'FJI', TYPE: 'Sovereign country', NAME: 'Fiji', NAME_LONG: 'Fiji', LABELRANK: 4, POP_EST: 1 },
        square(178, -17, 0.5),
      ),
    ],
  };
  const loadTenM = () => [
    feature(
      { ADM0_A3: 'TUV', TYPE: 'Sovereign country', NAME: 'Tuvalu', NAME_LONG: 'Tuvalu', LABELRANK: 6, POP_EST: 11052 },
      square(179, -8, 0.1),
    ),
  ];
  const { asset, report } = await buildAsset(baseGeo, {
    supplements: ['TUV'],
    loadTenM,
    loadPse: () => { throw new Error('unused'); },
  });
  assert.deepEqual(report.supplemented, ['TUV']);
  assert.ok('TUV' in asset, 'Tuvalu supplemented from 10m');
  assert.equal(asset.TUV.name, 'Tuvalu');
});

test('buildAsset: a present supplement never touches the 10m loader', async () => {
  const baseGeo = {
    features: [
      ...cleanPalestine(),
      feature(
        { ADM0_A3: 'TUV', TYPE: 'Sovereign country', NAME: 'Tuvalu', NAME_LONG: 'Tuvalu', LABELRANK: 6, POP_EST: 1 },
        square(179, -8, 0.1),
      ),
    ],
  };
  const { report } = await buildAsset(baseGeo, {
    supplements: ['TUV'],
    loadTenM: () => { throw new Error('loadTenM must not run when TUV already present'); },
    loadPse: () => { throw new Error('unused'); },
  });
  assert.deepEqual(report.supplemented, []);
});

// --- produced asset invariants (§ verification) ----------------------------
// Runs against the committed assets/geometry.json; skipped if it hasn't been
// built yet (e.g. a fresh checkout before `npm run build`).
test('produced asset holds exactly the Deck entities', { skip: !existsSync(ASSET) }, async () => {
  const asset = JSON.parse(await readFile(ASSET, 'utf8'));
  const keys = Object.keys(asset);
  assert.equal(keys.length, DECK_COUNT);
  assert.ok('PSX' in asset, 'Palestine present as its own entity');
  assert.ok('TUV' in asset, 'Tuvalu supplemented from 10m');
  for (const [a3, e] of Object.entries(asset)) {
    assert.match(e.d, /^M/, `${a3} has a path`);
    assert.equal(e.bbox.length, 4, `${a3} bbox`);
    assert.equal(e.centroid.length, 2, `${a3} centroid`);
    assert.ok(typeof e.name === 'string' && e.name.length, `${a3} name`);
    assert.ok(Number.isFinite(e.labelrank), `${a3} labelrank`);
  }
});

test('antimeridian entities are cut, not globe-wrapped (mainland bbox stays local)', { skip: !existsSync(ASSET) }, async () => {
  const asset = JSON.parse(await readFile(ASSET, 'utf8'));
  for (const a3 of ['RUS', 'USA', 'FJI', 'NZL']) {
    const [minx, , maxx] = asset[a3].bbox;
    assert.ok(minx < maxx, `${a3} bbox not inverted`);
    assert.ok(maxx - minx < 180, `${a3} mainland span ${maxx - minx} stays local, not globe-wrapped`);
  }
});

test('archipelago nations frame across the whole chain, not one island (Indonesia)', { skip: !existsSync(ASSET) }, async () => {
  const asset = JSON.parse(await readFile(ASSET, 'utf8'));
  // Real Indonesia runs Sumatra (~100°E) to Papua (~137°E); the largest single
  // island (Kalimantan) alone would frame a narrow window over ~109–119°E.
  const [minx, , maxx] = asset.IDN.bbox;
  assert.ok(minx < 100, `bbox reaches Sumatra (${minx})`);
  assert.ok(maxx > 137, `bbox reaches Papua (${maxx})`);
});

test('contested entities land mid-to-late by LABELRANK (§2.4)', { skip: !existsSync(ASSET) }, async () => {
  const asset = JSON.parse(await readFile(ASSET, 'utf8'));
  assert.equal(asset.TWN.labelrank, 3);
  assert.equal(asset.PSX.labelrank, 5);
  assert.equal(asset.SOL.labelrank, 5);
  assert.equal(asset.KOS.labelrank, 6);
  assert.equal(asset.CYN.labelrank, 6);
  assert.equal(asset.SAH.labelrank, 7);
});
