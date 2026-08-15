// Pure transforms for the geometry asset pipeline.
// No I/O here — `build-geometry.mjs` owns fetching/writing; these functions are
// what the unit tests exercise.
import {
  geoEquirectangular,
  geoPath,
  geoBounds,
  geoArea,
  geoCentroid,
  geoContains,
} from 'd3-geo';
import {
  COMMON_NAME_OVERRIDES,
  INCLUDED_TYPES,
  PALESTINE_PROBES,
  SUPPLEMENT_CODES,
} from './overrides.mjs';

// Equirectangular (Plate Carrée) tuned so projected x == lon and y == -lat
// (SVG y grows downward). Coordinates land in degree units, so bounding boxes
// are trivial and per-Card zoom is a pure viewBox swap. The per-entity cos(lat)
// correction happens later at render time, not here.
export const DEGREES_PER_RADIAN = 180 / Math.PI;
export function makeProjection() {
  return geoEquirectangular().scale(DEGREES_PER_RADIAN).translate([0, 0]);
}

function round2(v) {
  return Math.round(v * 100) / 100;
}

// Inclusion rule: every feature except the two uninhabited dependencies.
// Data-derived, never a hand-typed list.
export function isDeckFeature(props) {
  if (INCLUDED_TYPES.has(props.TYPE)) return true;
  if (props.TYPE === 'Dependency' && props.POP_EST > 0) return true;
  return false;
}

// Common name: NE `NAME`, with the curated override table applied by ADM0_A3.
export function commonName(props, overrides = COMMON_NAME_OVERRIDES) {
  return overrides[props.ADM0_A3] ?? props.NAME;
}

// The individual polygons of a (Multi)Polygon feature, each wrapped as its own
// GeoJSON Polygon geometry.
function polygonsOf(geometry) {
  if (geometry.type === 'MultiPolygon') {
    return geometry.coordinates.map((coordinates) => ({
      type: 'Polygon',
      coordinates,
    }));
  }
  return [{ type: 'Polygon', coordinates: geometry.coordinates }];
}

// The largest-area polygon of a feature — the "mainland". Framing
// and the pin use this so multi-part features (France + French Guiana, USA +
// Alaska) don't blow the bbox out to an ocean-spanning box.
export function largestPolygon(geometry) {
  const polygons = polygonsOf(geometry);
  let best = polygons[0];
  let bestArea = -Infinity;
  for (const polygon of polygons) {
    const area = geoArea(polygon);
    if (area > bestArea) {
      bestArea = area;
      best = polygon;
    }
  }
  return best;
}

// A polygon holding more than this share of a feature's area counts as its lone
// mainland. Above it, exclaves frame off the mainland (USA→Alaska,
// France→French Guiana); at or below it no polygon holds a majority, so the
// feature is a true archipelago and frames off its major islands instead.
export const DOMINANT_AREA_FRACTION = 0.5;
// For an archipelago, the polygons within this factor of the largest are the
// "major islands"; anything an order of magnitude smaller is a speck that would
// only bloat the bbox, so it's excluded (Indonesia keeps Sumatra…Papua; a lone
// reef does not stretch the window).
export const ARCHIPELAGO_AREA_RATIO = 10;

// The polygons a feature frames off. A feature with a dominant
// mainland frames off that one polygon (so distant exclaves never blow the bbox
// out); an archipelago with no majority island frames off every island within
// an order of magnitude of its largest.
export function framingPolygons(geometry) {
  const polygons = polygonsOf(geometry);
  if (polygons.length === 1) return polygons;
  const areas = polygons.map(geoArea);
  const total = areas.reduce((sum, a) => sum + a, 0);
  const max = Math.max(...areas);
  if (max > total * DOMINANT_AREA_FRACTION) {
    return [polygons[areas.indexOf(max)]];
  }
  return polygons.filter((_, i) => areas[i] * ARCHIPELAGO_AREA_RATIO >= max);
}

// The framing polygons as one geometry, ready for geoBounds/geoCentroid.
function framingGeometry(geometry) {
  const polygons = framingPolygons(geometry);
  return polygons.length === 1
    ? polygons[0]
    : { type: 'MultiPolygon', coordinates: polygons.map((p) => p.coordinates) };
}

// Project geoBounds output ([[minLon,minLat],[maxLon,maxLat]]) into the asset's
// coordinate space: [minx, miny, maxx, maxy] with y flipped (y = -lat), rounded.
export function projectBounds([[minLon, minLat], [maxLon, maxLat]]) {
  return [round2(minLon), round2(-maxLat), round2(maxLon), round2(-minLat)];
}

// Framing bbox (the mainland, or an archipelago's major islands) in
// projected/rounded coordinates.
export function framingBbox(geometry) {
  return projectBounds(geoBounds(framingGeometry(geometry)));
}

// Framing centroid, projected ([lon, -lat]) and rounded — so a dropped pin
// lands inside the framed region, not pulled offshore by overseas parts.
export function framingCentroid(geometry) {
  const [lon, lat] = geoCentroid(framingGeometry(geometry));
  return [round2(lon), round2(-lat)];
}

// One asset entry for a feature.
export function buildEntity(feature, path) {
  const p = feature.properties;
  return {
    name: commonName(p),
    name_long: p.NAME_LONG,
    d: path(feature),
    bbox: framingBbox(feature.geometry),
    labelrank: p.LABELRANK,
    pop_est: p.POP_EST,
    centroid: framingCentroid(feature.geometry),
  };
}

const byAdm0A3 = (features, a3) =>
  features.find((f) => f.properties.ADM0_A3 === a3);

// Detect whether the source folds Palestine into Israel. True when
// there is no separate Palestine feature, or when the Israel polygon still
// covers the West Bank / Gaza.
export function isPalestineMerged(features) {
  const psx = byAdm0A3(features, 'PSX');
  if (!psx) return true;
  const isr = byAdm0A3(features, 'ISR');
  if (!isr) return false;
  return Object.values(PALESTINE_PROBES).some((pt) => geoContains(isr, pt));
}

// Swap the clean Palestine polygon and the Israel-without-it in over the merged
// Israel, sourced from the `pse` point-of-view file. Geometry only —
// the base attributes (TYPE, POP_EST, LABELRANK, names) are preserved so
// downstream rules are unaffected. Returns a new feature array.
export function applyPalestineSwap(baseFeatures, pseFeatures) {
  const pseIsr = byAdm0A3(pseFeatures, 'ISR');
  const psePsx = byAdm0A3(pseFeatures, 'PSX');
  if (!pseIsr || !psePsx) {
    throw new Error('pse point-of-view file is missing ISR or PSX');
  }
  const swapGeometry = { ISR: pseIsr.geometry, PSX: psePsx.geometry };
  const result = baseFeatures.map((f) => {
    const geometry = swapGeometry[f.properties.ADM0_A3];
    return geometry ? { ...f, geometry } : f;
  });
  // If the base had no Palestine at all, carry the pse one in verbatim.
  if (!byAdm0A3(result, 'PSX')) result.push(psePsx);
  return result;
}

// Which of `codes` have no feature in `features` yet.
export function missingSupplements(features, codes) {
  const present = new Set(features.map((f) => f.properties.ADM0_A3));
  return codes.filter((code) => !present.has(code));
}

// Append the named entities, sourced from the 10m features, to the base set
// (always-in). Attributes come straight from 10m — these entities are
// absent from 50m, so there is nothing to preserve. Returns a new array.
export function applySupplements(baseFeatures, tenMFeatures, codes) {
  const add = codes.map((code) => {
    const f = byAdm0A3(tenMFeatures, code);
    if (!f) throw new Error(`supplement ${code} not found in 10m source`);
    return f;
  });
  return [...baseFeatures, ...add];
}

// Full pipeline over already-loaded GeoJSON. `loadPse` is an optional async
// thunk returning the pse point-of-view features; it is only awaited when the
// base source actually folds Palestine into Israel, so the clean 50m case never
// pays for (or mixes in) the 10m file. Returns { asset, report }.
export async function buildAsset(
  baseGeo,
  { loadPse, loadTenM, supplements = SUPPLEMENT_CODES } = {},
) {
  let features = baseGeo.features;

  let palestineHandling = 'source-already-separated';
  if (isPalestineMerged(features)) {
    if (!loadPse) {
      throw new Error(
        'source folds Palestine into Israel but no pse loader was provided',
      );
    }
    features = applyPalestineSwap(features, await loadPse());
    palestineHandling = 'pse-swap';
  }

  const supplemented = missingSupplements(features, supplements);
  if (supplemented.length) {
    if (!loadTenM) {
      throw new Error(
        `entities absent from 50m need the 10m source: ${supplemented.join(', ')}`,
      );
    }
    features = applySupplements(features, await loadTenM(), supplemented);
  }

  const deck = features.filter((f) => isDeckFeature(f.properties));
  const path = geoPath(makeProjection()).digits(2);

  const asset = {};
  for (const feature of deck) {
    asset[feature.properties.ADM0_A3] = buildEntity(feature, path);
  }

  return {
    asset,
    report: {
      source_features: baseGeo.features.length,
      deck_count: Object.keys(asset).length,
      palestine_handling: palestineHandling,
      palestine_present: 'PSX' in asset,
      supplemented,
    },
  };
}
