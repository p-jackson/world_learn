// PROTOTYPE geometry generator — mirrors the production pipeline (ticket 03):
// NE 110m topojson -> topojson.feature -> d3.geoEquirectangular -> per-feature
// { id, name, d, bbox } in projected (lon, -lat) degree space.
// 110m (not 50m) is deliberate: fast + fine to *feel* the zoom. Throwaway.
import { readFileSync, writeFileSync } from "node:fs";
import * as topojson from "topojson-client";
import { geoEquirectangular, geoPath, geoArea } from "d3-geo";

// bbox of the LARGEST polygon only — mainland framing. Real pipeline (03) does
// this properly (per-feature geoBounds + antimeridian); here we just drop the
// scattered overseas parts (French Guiana etc.) that would blow up the bbox.
function mainlandBounds(path, feature) {
  const geom = feature.geometry;
  const polys = geom.type === "MultiPolygon" ? geom.coordinates
    : geom.type === "Polygon" ? [geom.coordinates] : null;
  if (!polys || polys.length === 1) return path.bounds(feature);
  let best = null, bestA = -1;
  for (const rings of polys) {
    const f = { type: "Feature", geometry: { type: "Polygon", coordinates: rings } };
    const a = geoArea(f);
    if (a > bestA) { bestA = a; best = f; }
  }
  return path.bounds(best);
}

// 50m = the production decision (ticket 03): coastline detail survives zoom-in.
const topo = JSON.parse(readFileSync("countries-50m.json", "utf8"));
const fc = topojson.feature(topo, topo.objects.countries);

// scale = 180/PI, translate [0,0] => projected x = lon(deg), y = -lat(deg).
const proj = geoEquirectangular().scale(180 / Math.PI).translate([0, 0]);
const path = geoPath(proj);

// 2 decimals (~1km) keeps small-island/peninsula detail at 50m; still shrinks the asset.
const round = (s) => s.replace(/-?\d+\.\d+/g, (n) => (+n).toFixed(2));

const out = [];
for (const f of fc.features) {
  const d = path(f);
  if (!d) continue;
  const [[x0, y0], [x1, y1]] = mainlandBounds(path, f); // mainland projected bbox
  out.push({
    id: String(f.id),
    name: f.properties.name,
    d: round(d),
    bbox: [x0, y0, x1, y1].map((n) => +n.toFixed(1)),
  });
}
out.sort((a, b) => a.name.localeCompare(b.name));
writeFileSync("geometry.json", JSON.stringify(out));
console.log(`emitted ${out.length} features -> geometry.json (${(JSON.stringify(out).length/1024).toFixed(0)} KB)`);
console.log("sample names:", out.slice(0, 8).map((f) => f.name).join(", "));
console.log("has Kosovo:", out.some(f=>f.name==="Kosovo"), "| Taiwan:", out.some(f=>f.name==="Taiwan"), "| W.Sahara:", out.some(f=>/Sahara/.test(f.name)));