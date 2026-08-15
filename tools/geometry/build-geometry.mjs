#!/usr/bin/env node
// Dev-only build step. Turns Natural Earth admin-0 into the static geometry
// asset the app ships and renders from: assets/geometry.json, a flat map keyed
// by ADM0_A3. This script never ships in the app build.
//
//   node build-geometry.mjs          # fetch (cached) -> write asset
//   npm run build                    # same, from tools/geometry/
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { buildAsset } from './lib.mjs';
import { DECK_COUNT } from './overrides.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = join(HERE, '..', '..');
const CACHE_DIR = join(REPO, 'tmp', 'ne-cache');
const OUTPUT = join(REPO, 'assets', 'geometry.json');

// Data sources are pinned to specific commits so builds are reproducible.
const MARTYNAFFORD = '0b9a6ceb0a7032713abd9460ac1e995a9c60cd1e';
const NVKELSO = 'ca96624a56bd078437bca8184e78163e5039ad19';

// Full-attribute NE 50m admin-0 — not the attribute-stripped world-atlas
// TopoJSON. martynafford's GeoJSON ships the full property set.
const BASE_URL =
  `https://raw.githubusercontent.com/martynafford/natural-earth-geojson/${MARTYNAFFORD}/50m/cultural/ne_50m_admin_0_countries.json`;
// 10m admin-0, source for entities NE drops at 50m (Tuvalu). Same provider as
// the base, so the property schema matches. Only fetched if a supplement is
// missing from the base.
const TENM_URL =
  `https://raw.githubusercontent.com/martynafford/natural-earth-geojson/${MARTYNAFFORD}/10m/cultural/ne_10m_admin_0_countries.json`;
// Palestine point-of-view file, only fetched if the base is merged. NE's actual
// filename is `_pse`.
const PSE_URL =
  `https://raw.githubusercontent.com/nvkelso/natural-earth-vector/${NVKELSO}/geojson/ne_10m_admin_0_countries_pse.geojson`;

// Fetch with an on-disk cache under tmp/ (gitignored) so repeat builds are
// offline and fast.
async function fetchCached(url, filename) {
  await mkdir(CACHE_DIR, { recursive: true });
  const cached = join(CACHE_DIR, filename);
  if (existsSync(cached)) {
    return JSON.parse(await readFile(cached, 'utf8'));
  }
  process.stderr.write(`fetching ${url}\n`);
  const res = await fetch(url);
  if (!res.ok) throw new Error(`fetch ${url} -> ${res.status}`);
  const text = await res.text();
  await writeFile(cached, text);
  return JSON.parse(text);
}

async function main() {
  const baseGeo = await fetchCached(BASE_URL, 'ne_50m_admin_0_countries.json');
  const { asset, report } = await buildAsset(baseGeo, {
    loadPse: async () =>
      (await fetchCached(PSE_URL, 'ne_10m_admin_0_countries_pse.geojson')).features,
    loadTenM: async () =>
      (await fetchCached(TENM_URL, 'ne_10m_admin_0_countries.json')).features,
  });

  await mkdir(dirname(OUTPUT), { recursive: true });
  await writeFile(OUTPUT, JSON.stringify(asset));

  const bytes = Buffer.byteLength(JSON.stringify(asset));
  process.stderr.write(
    `wrote ${OUTPUT}\n` +
      `  source features: ${report.source_features}\n` +
      `  deck entities:   ${report.deck_count}\n` +
      `  palestine:       ${report.palestine_handling} (present: ${report.palestine_present})\n` +
      `  supplemented:    ${report.supplemented.length ? report.supplemented.join(', ') : 'none'}\n` +
      `  asset size:      ${(bytes / 1024).toFixed(0)} KiB\n`,
  );

  if (report.deck_count !== DECK_COUNT) {
    throw new Error(`expected ${DECK_COUNT} deck entities, got ${report.deck_count}`);
  }
  if (!report.palestine_present) {
    throw new Error('Palestine (PSX) missing from asset');
  }
}

main().catch((err) => {
  process.stderr.write(`build-geometry failed: ${err.message}\n`);
  process.exit(1);
});
