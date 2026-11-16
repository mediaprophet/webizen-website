# Native prototype scene

The native scene is `scene.json`; review entry is `/aaa-instanced-proof.html`. It preserves the complete authored world, including all 367 ecology placements. `assemble_scene.py` and `verify_assets.py` reproduce the manifest and asset checks.

| Measurement | Full baked scene | Prototype scene |
| --- | ---: | ---: |
| Native asset bytes | 262,515,624 | 100,647,704 |
| Uploaded source vertices | 1,917,824 | 416,487 |
| Uploaded source triangles | 2,443,920 | 679,904 |
| Expanded authored triangles | 2,443,920 | 2,443,920 |
| Containers | 118 | 91 |
| Ecology placements | 367 | 367 |

The payload falls by 161,867,920 bytes (61.66%). This is the native geometry payload; it does not include WASM, textures, manifest text or other page resources. It is not a frame-rate claim.

Nine local prototypes are produced with the same Synth import pipeline, retained source geometry and authored UVs. Native import creates/preserves surface frames. Each prototype has strict replay evidence, native provenance attestation and canonical material namespace receipts. The transform list is copied exactly from the evaluated Vibe placement output; the host validates and composes it using QualiaDB/Etalon quaternion math and native instance ABI packing. Static assets receive identity transforms.

QualiaDB now accepts bounded four-word descriptors that partition mesh index ranges and instance records exactly once. Native culling uses actual prototype geometry bounds, compacts camera-visible records without heap allocation, and clips material draws to the correct group. Forward HDR/LDR, AO and picking use those visible groups; shadows retain all source instances so off-camera casters remain valid. Materials, textures and tangents stay in their existing native paths. Existing picking readback resolves the actual stable placement identities.

Ten focused native tests pass, including invalid partitions and indices, geometry-derived bounds, material ranges crossing prototype boundaries, visibility and semantic identity, off-camera shadow retention, empty visibility, legacy behaviour, transformed world bounds and zero allocation during cull/draw iteration. These tests compile the actual QualiaDB implementation against the installed native Etalon provider; the combined browser build separately checks the pinned provider.

Asset verification passes for all 91 containers, every texture hash, all 367 exact placement mappings, all 9 strict Synth replays and native preservation receipts. `asset-verification.json` records the evidence.

Authored near/mid prototype choices are retained. This change does not introduce automatic LOD substitution. Transparent multi-instance groups continue to fail closed under the existing provider contract; the current authored ecological geometry uses opaque materials. Admission receipts are snapshots, not continuous GPU timing. Browser visual, picking, GPU validation and quiescent before/after timing remain required before promoting this review scene.

No Etalon source change is included. The existing immutable provider pin remains 785ab9c1b7d729db9afcd9b0f3fd70344b37be1e.
