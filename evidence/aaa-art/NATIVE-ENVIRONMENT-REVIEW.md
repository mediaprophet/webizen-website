# Native woodland and understorey proof review

Status: candidate assets ready for native world integration. This is not AAA acceptance, mobile qualification, ecological simulation or completion of the website overhaul.

## Delivered

`production/aaa-environment/assets/integration.json` names 36 sealed native groups and 15 unique texture-role bindings. The groups contain 93 trees, 71 stones, 161 grass tufts, 14 shrubs (matching separate branch/leaf groups) and 14 curled-leaf litter patches. Geometry totals 1,846,066 triangles and 1,562,362 native vertices; sealed payloads total 170,611,600 bytes. Geometry instance count is367 because each shrub contributes a branch and a leaf instance.

Seven new shape/material prototypes use Synth lofts: near/mid grass, near/mid bank rock, shrub branches, shrub leaves and litter. Trees reuse the existing native 40k/8k simplifications of the commissioned eucalyptus artwork; there is no cone forest substitute. The 8k form is for distance, not close inspection. No newly produced far LOD is claimed.

Native steps retained in recipes and receipts: loft, project_dxf, import_mesh, unwrap_slim, compute_mikktspace, synthesize_maps, pack_pbr_set, compile_bsdf_graph and compile_10d_pbr. Python specifies control rings and packages exact native DXF triangles/retained native UV01 into attributed GLB; it does not substitute a geometry or UV solver. Native GLB import computes MikkTSpace corner splits. Triangle count remains exact; native vertex count can legitimately increase.

Placement clearance and terrain sampling execute the actual Vibe evaluator in bounded chunks. Receipts retain the world-frame source hash and input/output chunks. The frame is the existing fictional catchment, Y-up metres. It must not be relabelled the public Canberra DEM, a surveyed campsite, or a local species distribution.

## Visual inspection performed

Front, side and three-quarter native images were read for:

- `previews/bank-rock-near-1794798578217784000`
- `previews/bank-rock-mid-1794798834273684200`
- `previews/grass-near-1794798702731199600`
- `previews/grass-mid-1794798914092508100`
- `previews/shrub-branches-1794798976552367900`
- `previews/shrub-leaves-1794799398816693600`
- `previews/leaf-litter-1794799520196865000`
- Reused native tree previews `production/native-world/previews/eucalyptus-lod8000-1794794223` and `eucalyptus-lod40000-1794794000`.

The initial rock showed excessively sharp bedding; the final near prototype uses cubic interpolation and has a softer, uneven silhouette. The mid form is visibly faceted up close and should remain at distance. Grass retains curved blades, thin folded cross-sections and seed culms rather than opaque tuft volumes. The shrubs have articulated canes and paired leaves; these are artwork, not a verified taxon. The tree has an airy, drooping outline, but simplified trunks and repetitive branching remain visible close up.

The native assembled study at `production/aaa-environment/previews/understorey-study-1794799841879897300/frame-0000.png` was read. It combines rocks, two grass densities, the paired shrub and curled litter. Branch/leaf alignment is coherent, grass separates in silhouette, and the stone is legible. The study has no ground plane and does not prove slope contact. Foliage is still dark and fine against the neutral background; it needs whole-world lighting/composition judgement, not a declaration that the artwork bar is met.

The stone's initial additive environment fallback washed out material colour. The subsequent actual environment-probe study (`previews/bank-rock-ibl-proof/frame-0000.png`) retains albedo and surface relief more convincingly. The assembled study uses actual native irradiance/specular probes and zero additive fallback. This is evidence for the material path, not a completed lighting design.

## Source reuse decision

The Ecopunk Corymbia candidate was inspected at `C:/github/ecopunk/docs/qa/implementation-2026-10-27/round6-produced-assets/corymbia-aparrerinja/open_world_game`. Its run records declare `fixture_only:true` and software-fixture schema. The hero preview has a faceted white bole and solid green ellipsoid crowns. It is retained as lineage/reference evidence, not reused as an approved hero tree. No claim is made that Corymbia aparrerinja belongs at the Canberra reference site. Dataset-driven species selection remains a separate required step.

## Integration and remaining gates

1. Independent GLB material identities collide with the existing world MAT1 table. Root owns a native-codec namespaced derivative; original producer outputs remain immutable. Preserve every original section other than the required identity/table references and retain derivative hashes/lineage.
2. Inspect the actual assembled native world from campsite, bridge, riverbank and low walking views. Foliage must frame the approach without hiding car/camper or interactive destinations. Native clearance excludes the current camp and camera corridor, river channel, bridge and exhibit footprints.
3. Check tree roots, stone bottoms, grass bases and especially flat litter patches against local slope. Origin height alone does not prove contact across a wide patch. Correct native transforms/placements if floating or buried patches appear.
4. Judge distant tree repetition and close trunk faceting. Do not substitute isolated front/side preview quality for a natural woodland composition. The current proof has one tree archetype and one shrub form; species and age diversity remain incomplete.
5. Measure actual GPU residency, first load and frame time on the host. This proof adds170.6MB of sealed geometry. No mobile or performance acceptance is asserted.
6. Verify the retained texture URLs/role hashes in the integrated host and preserve normals, UVs and tangents through native material namespacing. Do not flatten back to position-only exchange.
7. Whole-world final screenshot and human review remain required before calling this proof accepted. All150 operations, data-backed demonstrations, accessible equivalents and the other realm proofs remain governed by the master plan.

`native-environment-validation.json` records the completed mechanical checks separately from these visual gates. It verifies asset hash/size, each strict replay receipt, source GLB hash, original native hash, attested native hash, native triangle counts and section-preserving provenance receipts. It does not stand in for a visual review.
