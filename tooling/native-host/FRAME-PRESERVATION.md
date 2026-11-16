# Native scene-frame preservation repair

The full authored proof contains 118 assets, 1,917,824 vertices and 2,443,920 triangles. Portal scene assembly previously retained source normals but discarded source tangent frames. It then passed no tangents, together with UV01, to the GPU uploader. That uploader correctly limits tangent-generation scratch to 1,048,576 vertices. Applying that generation limit to the merged scene caused material admission to fail, even though every individual asset fit its existing bounded generation path. Portal then attempted its colour fallback; the website host refused that fallback.

The repair keeps authored tangent direction and handedness. Assets without tangents use the existing allocation-free Etalon UV-derivative kernel through caller-owned per-asset scratch. The existing generation ceiling is unchanged. The merged scene passes its aligned tangent stream directly to the existing material uploader. Uniform orbit normalisation leaves directions unchanged; nonuniform body fitting regenerates surface frames within each original asset span and refuses invalid cross-asset indices. Original geometry, material records, UV seams, provenance and shader interpretation are preserved.

A separate integer preflight checks the actual device max_buffer_size before cold normal-workspace/GPU allocations. For this full proof, the largest vertex stream is 30,685,184 bytes and the index stream is 29,327,040 bytes. Total mesh residency is approximately 152,067,776 bytes plus material uniforms; these are separate vertex/index buffers, not one 128 MiB storage binding. Native global and viewport residency reservations still apply. No budget is increased.

Changed core owners: render/portal/scene_mesh.rs and its existing tests; focused scene_frames.rs + scene_frames_tests.rs; one upload argument in portal/mod.rs; GPU mesh_upload.rs plus focused mesh_upload_limits.rs. The existing scene accumulator remains 522 lines; the substantive new cold frame lifecycle is isolated in the 135-line helper. This is a scoped integration repair, not a renderer replacement.

Validation:
- scene-frame-checks.rs compiles the actual new helper sources against the existing native QualiaDB/Etalon libraries. Ten tests pass: authored frame round trip, UV-rotated handedness, legacy zero-UV frames, unchanged generation cap, nonuniform-fit local indexing, malformed span atomic refusal, combined assets above the former merged ceiling, and actual device-buffer boundary cases.
- scene-frame-inspection-full.json records successful actual codec/frame generation for all 118 source-qualified full-scene assets and all 1,917,824 vertices.
- Additional scene-accumulator regression tests cover merge/orbit preservation and nonuniform fitting in the core test module.
- Full rebuilt WASM material binding, exact geometry counts and presented pixels require browser review. CPU helper success alone is not visual acceptance.
