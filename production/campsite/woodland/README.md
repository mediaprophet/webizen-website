# Open-grown eucalyptus: native woodland artwork

A 14.3 m authored eucalyptus-inspired tree for the clearing margins. It uses a pale sinuous bole, eight irregular scaffold limbs, secondary branches, terminal twigs and 1,920 individually modelled hanging lanceolate leaves. This is an artistic botanical interpretation, not a species-identification model or a scan.

`assets/eucalyptus.integration.json` is the compact integration record. `assets/eucalyptus.json` contains seven material batches: the exact native exported faces are concatenated by material, with per-source ranges and hashes retained in `source_parts`. Browser presentation must preserve these faces. Position the trunk at ground y=0; its root flare intentionally extends to y=-0.11 m. Use modest scale and rotation variation when instancing around the clearing.

`build_eucalyptus.py` authors explicit native loft controls and native leaf prototype instancing in `recipes/eucalyptus.vibe`. It exports DXF faces, retains their exact combined OBJ, then seals the same geometry into a `.10d` and HMC. Strict replay succeeds for both production phases. The seal intentionally disables optional topology and spatial-index sections; they are not required for this visual asset. The first default seal exceeded a private 180-second deadline, so this folder includes an isolated longer-timeout bridge. No live MCP or engine source was changed.

`validate_eucalyptus.py` verifies every material-batch range against its original DXF faces, as well as finite coordinates, indices, native resources, source/retained-input hashes and strict replay. See `review/geometry-validation.json` and `review/visual-review.json`.

All three native PNG views and four browser colour views were inspected. The native preview shows the uniform preview material; the browser images show the authored palette. `review.cjs` uses a temporary local server and the existing site's Three.js files without modifying them. The tree's terminal sprays remain deliberately stylised. Final terrain contact, forest density and lighting belong to the campsite integration review.
