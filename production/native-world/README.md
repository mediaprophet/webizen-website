# Native Webizen world package

`scene.json` loads 85 native `.10d` material groups, with baked metre transforms from native-evaluated `scene-authoring.vibe` plus the canonical `living-world.vibe`. The package retains 433,225 vertices and 597,854 triangles. The 256 native river top-face triangles are provided separately for the existing native water pass. The full scene assets occupy 96,072,464 bytes.

Source parts are original Synth-generated meshes. Python packages exact retained faces into GLB inputs; Synth imports their authored node transforms and material table, then compiles `.10d` using `hypermedia.manifold.compile_10d_pbr`. There is no JavaScript world or browser geometry authoring in this package. Native MAT1 carries the shared 62-definition material table. Manifest RGBA values are neutral multipliers.

The complete car, raised vintage camper, Starlink Mini, two three-panel solar arrays and one two-panel solar array remain full detail. Eucalyptus uses native QEM simplification: six near trees at 39,994 triangles and twelve distant trees at 7,992 triangles each. The original 112,424-triangle tree remains intact. The 1,992-triangle candidate was rejected after three-view inspection. The bridge, receiver, double helix and earlier native sculptures are included.

## Reproduce and verify

From this directory:

```powershell
./provenance/build.ps1
python build.py
python validate.py
```

`build.py --preview` additionally generates native front, side and three-quarter group views. `--review-lods` reviews the 40k and rejected 2k candidates; `--review-lod-target 8000` reviews the accepted distant tree. Each producer invocation uses unique artifact paths; cached inputs and strict replay receipts prevent unnecessary regeneration.

`provenance/main.rs` links the same cached native QualiaDB library as the producer. It appends a native ProvenanceSidecar containing the actual GLB input bytes and the project owner's scoped permission. It retains every pre-existing section byte, stride/count, and header policy flag; native CRC, provenance validation and `decode_load_10d` confirm admission. It asserts neither a general reuse licence nor a verifiable credential. Original native outputs remain immutable in the producer artifact directories. Per-asset receipts retain pre/post SHA-256, source SHA-256, helper identity and native decoded geometry counts.

`integrity.json` records verified native hashes, source replay, evaluated transformations, material-table consistency, exact camp configuration and water geometry. `preview-review.json` records observed group views; `lod-review.json` records the accepted/rejected woodland choices. Complete scene appearance and resident GPU counts are checked by the Rust/WASM host integration separately.

For serving, copy `scene.json` and `assets/`; include referenced `recipes/`, `scene-authoring.vibe`, `authoring-evaluation.json` and JSON `provenance/` receipts when exposing source links. Do not publish executable build helpers or the large artifact/cache directories as site payloads. The combined native-evaluated source snapshot is `recipes/evaluated-world.vibe`.
