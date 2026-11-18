# GIS ingest (EWM-04b / LW-08)

Native-first, consent-gated job (`webizen.ewm-gis-ingest-job/v1`) that reuses
the existing Canberra DEM path (`data-pack.exe dem` / `DemAdapter`) and packs
one additional public place from an OSM consent fixture (`OsmAdapter`). Public
boot never runs this. The homepage does not call live adapters.
`streaming.default` stays false. `inner_ranges` stays false.

This is the public Canberra reference bbox, not a personal campsite.

## Default deny

`job.json` and `jobs/river-osm.json` keep `opt_in: false`. Wasm public boot keeps
`public_boot_calls_live_adapters() == false` and
`wasm_ingest_opt_in_allowed() == false`. The browser only Ranges packed members
under `/packages/` or `/production/`. Live GBIF JSON parse stays deferred.

## Opt-in command

From the website root:

```
python tooling/ewm_ingest.py --opt-in
python tooling/ewm_ingest.py --opt-in --job data/ewm/gis/ingest/jobs/river-osm.json --consent-fixture data/ewm/gis/ingest/fixtures/river-osm-overpass.json
```

Without `--opt-in` the helper refuses. The flag is an explicit action (named-place
job), not a camera-tick scrape.

On Windows the Canberra helper shells to `native/data-pack-builder/data-pack.exe dem <dir>`
when that binary exists. On other platforms it looks for `data-pack` without
`.exe`. Create-only: if `native-receipt.json` is already in the output directory,
the helper exits 0 and does not overwrite.

## Consent notes

CI and review machines must not fetch live GIS hosts. LW-08 packs the river
place from `fixtures/river-osm-overpass.json` (generator
`webizen.ewm-gis-consent-fixture/v1`). That fixture is a bounded public OSM
extract, ODbL-attributed, not a live Overpass, Terrarium, STAC, or GBIF call.
`--consent-fixture` is required for OsmAdapter jobs. Live GBIF parse remains
deferred.

## Output (gitignored rasters)

Create-only artefacts go under `data/ewm/gis/ingest/out/`. DEM rasters, QDEM
bundles, and heightfields stay gitignored. Packed OSM measured members,
catalog rows, and ingest receipts may be kept as Range-able static files.
Root `.gitignore` is not owned by this packet.

New members enter the windowed Schedule as catalog rows with
`epistemic_class: measured-gis`. `append_measured_member` in
`native/world-host/src/runtime/gis_ingest.rs` validates those rows. It does not
fetch HTTP. River OSM is an Enhancement identity, not a 25th first-paint member.

Ingest out paths:

- `data/ewm/gis/ingest/out/canberra-public-reference/` (DEM, create-only)
- `data/ewm/gis/ingest/out/river/measured-member.json`
- `data/ewm/gis/ingest/out/river/catalog-row.json`
- `data/ewm/gis/ingest/out/river/ingest-receipt.json`

## Self-check (no live adapters)

```
python data/ewm/gis/ingest/check_job.py
```

## Never

- Timeline.json, exact campsite coordinates, placeIds, or INFERRED_WORK
- Live GBIF fetch or parse
- Public-boot live adapter calls
- Overwrite of an existing native receipt
- Live Overpass / Terrarium / STAC hosts in CI without a consent fixture
