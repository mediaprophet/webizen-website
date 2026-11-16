# Shared campsite terrain

The staged `site/world/terrain.js` is the shared terrain definition. `naturalHeightAt` preserves the original catchment. `heightAt` creates an 8 m radius level clearing centred on x=-22, z=28 at y=4.4909216892549235 m, then blends with cubic smoothstep back to the natural height at 13 m. Outside 13 m, the natural terrain is unchanged.

The website's 160-cell terrain grid supplies the OBJ input to actual Synth `import_mesh`, `compile_10d_pbr` and `project_dxf` operations. There are 25,921 vertices and 51,200 triangles. Strict replay passed. Every DXF triangle coordinate was checked against the authored OBJ geometry; the largest difference is 0.00000355431584 m (native floating-point conversion). Browser adapter positions and indices are read directly from that DXF.

Integration files:

- `../../../site/world/terrain.js`: copy to the website's corresponding path.
- `terrain-production.json`: replace the website's terrain production record with this record; all public asset URLs point below `/production/campsite/terrain/`.
- `assets/catchment-clearing.mesh.json`: exact native DXF adapter, if the scene consumes native mesh JSON.
- `assets/terrain.manifest.json`: production and geometry correspondence evidence.
- `assets/integrity-review.json`: repeatable verification results.

Reproduction: run `node export.mjs`, then `python build_terrain.py`, then `python validate_terrain.py` from this directory. New Synth production and preview paths are create-only. The export checks every flat-zone and outer-zone vertex. The validator verifies source, HMC, inputs, resources and native preview hashes, and every triangle coordinate.

Visual review completed on the native `front-view-assumed.png`, `side-view-assumed.png` and `three-quarter-view-assumed.png` in `previews/catchment-clearing-1794790749`. The continuous open heightfield, valley and ridges remain coherent; a small level pad is visible within the catchment. These are neutral native geometry previews, not the final forest material/lighting presentation. No scene, app or world manifest was changed by this package.
