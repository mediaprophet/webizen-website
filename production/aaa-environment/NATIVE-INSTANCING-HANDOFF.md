# Native instancing handover

Read-only source audit, 16 November2026. The Hypermedia FUNCTIONS/module index was checked first for instance_on_points/realize_instances. QualiaDB and Etalon do not have an audit-docs/FUNCTIONS.md at the guessed root paths; their actual source files were searched and read. This is a map of verified APIs, not a claim that any catalog algorithm is absent.

## Existing API

All paths below are relative to `C:/github`.

| Existing entry | Exact source | Behaviour relevant here |
|---|---|---|
| QualiaPortal::set_mesh_instances | qualiaDB/crates/qualia-core-db/src/render/portal/mod.rs:1173 | Accepts ABI version and packed byte records; actual generated qualia.d.ts exports it. |
| pack_mesh_instance_records | same file:1249 | Column-major16-float transform and low/high semantic ID words; native inverse-transpose normal frame and orientation sign. |
| cull_mesh_instance_indices / compact_mesh_instance_records | same file:1204 /1228 | Caller-buffered native stable selection/compaction. |
| stride / version | same file:1289 /1294 |128 bytes and ABI2 currently. Read these contracts rather than hard-coding a future ABI. |
| GpuInstanceRecord | etalon/crates/etalon-render/src/instance_culling/mod.rs:58 | Transform64bytes, normal frame48bytes, semantic ID8bytes, orientation sign4bytes, padding4bytes. Constructor:70; maximum10,000 records at:9. |
| GpuInstanceStream<R> | etalon/crates/etalon-render/src/instance_stream.rs:227 | Existing bounded GPU storage and reservation provider. new:237, replace_packed:260, replace_culled_packed:331, update_dirty_ranges_packed:353. |
| update_mesh_instance_visibility | qualiaDB/crates/qualia-core-db/src/render/gpu/instance_visibility.rs:12 | Stable camera selection; source stream retained for shadow casters; scratch buffers preallocated. |
| select_lods_screen_space | qualiaDB/crates/qualia-core-db/src/render/lod_chain.rs:420 | Existing error-based native LOD selection with hysteresis, stable source/semantic IDs, caller-owned output. Needs actual monotonic geometric-error inputs. |

## Integration boundary

The current PortalGpu mesh setter replaces a **shared** stream. The forward pass in `qualiaDB/crates/qualia-core-db/src/render/gpu/portal_gpu/render_frame.rs:296` repeats each material range over `0..visible_mesh_instances.count()`. Loading a merged world and supplying93 tree transforms would replicate the whole world93 times. The current host correctly does not do this.

Production should retain nine prototype meshes (two tree LODs plus seven new understorey forms), then associate each prototype's native material/index ranges with its own existing GpuInstanceStream. Static terrain/buildings retain identity placement. This requires native draw-batch integration using existing providers; no JavaScript world or newly invented instancing solver. Forward, AO, depth, shadow and semantic picking must use corresponding batch transforms/IDs. Current shared source/visible stream lifecycle is the reference implementation, not sufficient by itself for mixed prototypes.

The per-prototype admission path must preserve MAT1, UV01, authored normals/tangents and provenance. Independent material IDs still need namespacing. Do not solve material collision by dropping bindings or bypassing governance. Prototype source hash, native output hash and placement-recipe/Q42 identities should remain distinct lineage records.

`SceneMeshAccum::normalise_to_orbit_frame` (`portal/scene_mesh.rs:493`) applies a global centre and uniform scale to mesh vertices and semantic nodes. New prototype transforms must compose the same mapping once. Do not normalise each prototype independently and then apply unchanged metre translations.

`gpu/mesh_upload.rs:53–63` explicitly refuses multiple instances when any draw uses Blend opacity, pending sorted transparent submission. Current grass and leaf silhouettes are opaque actual geometry, so they do not require transparent instancing. Preserve this guard.

367 instance records are46,976bytes; the baked ecology payload is170,611,600bytes. That arithmetic indicates the duplication opportunity, not a measured final transfer or frame-time result. Prototype geometry, materials, textures, culling buffers and other scene assets still consume memory. A single-prototype native proof using the existing Portal API is immediately feasible; mixed-world savings require the per-prototype integration and real resident/upload/frame measurements.

The existing Synth `instance_on_points`/`realize_instances` path is also real. `manifold_fields_sdf/fields_instances.rs:553` realises vertices/triangles into a new mesh (bounded200k vertices/400k triangles), so using it does not alone remove duplicated resident scene geometry. It is useful for production geometry, not a substitute for the retained GPU instance stream.

No runtime or provider source was edited by this audit.
