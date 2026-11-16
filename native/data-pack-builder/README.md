# Native proof data production

This package is the bounded A00/A02 Earth, space, graph, rights and benchmark proof subset. It does not complete the 204-route migration or claim final artwork acceptance.

`python native/data-pack-builder/build.py` compiles the actual current QualiaDB DEM adapter, terrain mesh and terrain pipeline source, using the existing native library for Q42, query VM, provenance and other core functions. Private HTTP and adapter-trait support are byte-exact source excerpts, regenerated from the owning module. No Python DEM decoder, geometry algorithm, Q42 writer or query evaluator is used. Dependency paths and hashes are in evidence/aaa-data/native-build.json. The core source files were patched only for the two observed geospatial defects described below.

`data-pack.exe dem NEW_OUTPUT_DIRECTORY` performs the consent-gated native acquisition for the fixed public Canberra reference bbox. It is bounded to four Terrarium tiles. Existing output is create-only. This is public reference terrain independently selected for this demonstration, not the user's campsite or travel data. Acquired raster data remains in the original native QDEM source bundle and .10d provenance.

`python native/data-pack-builder/acquire_jpl.py` acquires two Earth and two Mars JPL Horizons vectors with explicit ICRF frame-plane, Sun-centred origin, TDB epochs and km/km/s units. It only parses the official CSV response; no orbital solver is implemented in Python. Local native solver comparisons remain a separate qualification step.

`python native/data-pack-builder/assemble.py` preserves the complete checked-in Schema.org Q42 graph, copies five exact historical benchmark JSONs, evaluates six public rights fixtures with native compile_norm_quin/evaluate_deontic_contract, and builds a new native Q42 metadata graph with UnifiedVolumeBuilder. Its new Q42 output is create-only. Each query fixture uses actual compile_ntriples_to_bytecode/execute_program_with_stats. Historical results retain all original units/engine labels and have not been rerun or promoted to new benchmark claims.

`data-pack.exe query FILE.q42 "<subject> ?p ?o ."` verifies native blocks and executes the real native VM. These flat NTriples pattern fixtures are not SPARQL join claims. The Schema.org lexicon has three pages with maximum encoded page length290,510 bytes; a64KiB scratch buffer is insufficient.

`data-pack.exe terrain-mesh HEIGHTFIELD.json OUTPUT.json` calls the corrected native generate_terrain_mesh against the retained actual elevation samples. The JSON is a native build interchange, never browser geometry. `python native/data-pack-builder/build_terrain_model.py` packages those native faces into retained GLB and calls Synth import_mesh/compile_10d_pbr. Native GLB node transformation creates a1:30 model, without vertical exaggeration, Z-up toY-up, elevation541m at displayY0. PBR is neutral relief shading, not measured landcover. Final model-v2 scene and its exact source/transform/provenance receipts are in data/publications/aaa/terrain-model-v2.

`terrain-admit.exe COMPILED.10d SOURCE-TERRAIN.10d OUTPUT.10d` uses existing native codecs to copy the original DEM provenance sidecar byte-for-byte, including actual source tiles, attribution, licence and source hash. Original compiled geometry/material payloads and header policy flags remain unchanged. It validates native CRC/provenance/admission. No artwork licence replaces third-party source terms.

## Observed defects and correction

1. dem_adapter.rs had0.25 inside the Mercator latitude logarithm, shifting Y by a constant. The first bounded acquisition fetched ocean tiles and returned all zero elevations. It remains rejected evidence at data/acquisition/aaa/canberra-reference, never a published valid Canberra dataset. Corrected requests cover z13/x7487-7488/y4954-4955 and produce541–677m actual elevation. Tests cover the equator, both hemispheres, inverse projection and exact locality.
2. dem.rs produced clockwise triangles facing-Z for a declared Z-up heightfield. Source triangle winding was corrected and tested on flat and sloping grids. Display-v2 reuses the original sampled positions and only changes face orientation through that actual native function. Original source bytes remain available for reproducibility; the first dark downward-facing display model is superseded, not approved artwork.

The public catalogue includes native data, raw source, actual expected query results, policy fixtures and source hashes for inspect/export; old page links are not treated as migration completion. No private Timeline data, source paths or personal location records are used.
