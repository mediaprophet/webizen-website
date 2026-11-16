# Native world: visual parity audit

16 November 2026. Verdict: the inspected native presentation is materially below the previous Three.js review. The native architecture is correct to retain; visual parity is not achieved. This is an image/source comparison, not a performance or final art acceptance claim.

## Images actually inspected

- Earlier campsite: `C:/github/synth/production/webizen-swarm/evidence/campsite-independent/desktop.png` and `evidence/campsite/desktop.png` (the latter includes native eucalyptus within the old presentation).
- Earlier river: `C:/github/webizen-website/evidence/world-v2-river.png`.
- Native campsite: `C:/github/synth/production/webizen-swarm/evidence/native-world/browser/world/desktop.png` (saved 13:28).
- Native river/canopy: `evidence/native-world/interaction/the-living-river.png` and `the-forest-canopy.png` under the same stage root.

These are different revision captures, not controlled equal-camera measurements. The visible environmental omissions agree with current sources. Concurrent lighting edits may postdate the screenshots and require new captures.

## Largest losses and direct restoration

| Priority | Visible regression | Source cause / concrete change |
|---|---|---|
| 1 | Huge bare green expanses; the riverbank has saw-toothed tan/green triangles. | `production/native-world/build.py:63–69` assigns whole triangles to four flat height bands. Old `site/world/scene.js:13–15` used varying vertex colours, world X/Z UVs and an existing Synth normal map. Replace hard face bands with continuous, surface-aware variation and restore native texture bindings. Do not conceal the colour boundary with extra trees alone. |
| 1 | Dense, layered forest has become isolated specimens; “forest canopy” centres one decorative frond in empty space. | Old `scene.js:21–25` distributes forest from 1,700 candidate positions, with clearance exclusions, height and crown variation. Native `scene-authoring.vibe` contains six near and twelve far eucalyptus, no understorey. Restore dense native woodland in depth bands around every exhibit, with bounded LOD and selective clearings. Retain recognisable native eucalyptus rather than reinstating cone trees. Reframe the canopy stop within that woodland. |
| 1 | River has lost its rocky edge, foreground scale and natural transition into terrain. | Old `scene.js:27` adds 200 boulders along the banks; native placement list includes no rocks. Add reviewed Synth rock families, gravel/ground detail and bank vegetation in authored native groups. Avoid an evenly spaced decorative border. |
| 2 | Gold car, glazing and solar panels read dark/flat; the scene lacks the old soft atmospheric layering. | Old presentation combines hemisphere fill, warm sun, ACES exposure, exponential fog and a PMREM sky environment (`scene.js:47–54`). Native has sun/ambient, sky, AO and shadows, but no equivalent authored probe setup in the inspected build. Current Vibe explicitly sets `environment_fallback:[0,0,0]`; host uploads empty probes with that zero fallback. Restore calibrated sky/ground environment radiance or native probes; review metallic and glass surfaces before adjusting exposure further. A preset is not proof of matched appearance. |
| 2 | Main subjects compete with a large right panel; river/bridge are partly obscured, and other stops look across empty ground. | Native camera targets retain exhibit positions without environmental staging. Adjust native Vibe camera targets and distances after restoring foreground/middle/background composition. Keep the campsite readable and give each stop a distinct ecological setting. Test compact overlays and mobile safe areas. |
| 3 | Less environmental life and motion. | Old source has 260 fine pollen points, a 16-bird flock and a rotating receiver ring. Native compiler has none of these scene elements. Restore only after terrain, vegetation and lighting: sparse authored native particles/actors and gentle native animation, with reduced-motion behaviour. Water is already supplied to the native water pass; do not claim it is absent. |

The older version also had bare clearing ground and stylised generic trees. Neither version meets the final artwork brief. Grass/leaf litter/shrubs are improvements still required, not features falsely claimed to have existed in the old shot.

## Exact material loss

`build.py::glb` emits only `POSITION` and indices. It does not author `TEXCOORD_0`, `COLOR_0`, normals or tangents. The native seal can compute normals, so this is not a claim that rendered normals are absent; it means the input preserves no authored surface frame, UVs or vertex colours. `material()` preserves base colour, roughness, metallic and sidedness, with no textures.

The accepted Synth maps already exist in `production/assets/environment.json`:

- `production/artifacts/catchment-7a5bb20972d0/ground-colour.png`
- `production/artifacts/catchment-7a5bb20972d0/ground-normal.png`
- `production/artifacts/catchment-7a5bb20972d0/ground-orm.png`

The old world actually loads the normal map. The native compiler does not consume this texture record. Reuse or improve these real assets, with correct UV scale, rather than making another plain material.

## Existing native restoration capabilities

Located in Hypermedia `audit-docs/FUNCTIONS.md` and `audit-docs/modules/crates_asset-production-vibe.md`; relevant current implementations were read. No new engine or JavaScript world is required.

- **Surface maps:** `hypermedia.surface.synthesize_maps` and `pack_pbr_set`; `graphics_pack.rs::pack_pbr_set` accepts colour/normal/ORM handles. Reuse the retained maps where suitable.
- **UV/tangents:** `surface.unwrap_slim` supports slim/atlas modes; `surface.compute_mikktspace` supplies tangent frames. Planar terrain can retain deliberately authored world UVs through attributed GLB.
- **Native binding:** current `manifold_ops.rs::manifold_compile_10d_pbr` accepts `source_10d`, `tangents`, `bsdf_graph`, `colour_map`, `normal_map`, `orm_map`. Choose either texture-bearing attributed GLB material import, or geometry without imported materials plus explicit native binding. Explicit maps cannot replace imported material assignments: current code fails closed on that combination.
- **Native population:** `manifold.instance_on_points` plus `realize_instances`, with native `simplify` derivatives; loft/revolve/evaluate_field for reviewed rock, shrub, litter and grass prototypes. Keep actual source geometry/material provenance and bound final scene residency. The current merged-world instance stream is not an arbitrary per-part draw solution; compiler batching must respect its real interface.
- **Lighting:** existing `QualiaPortal::set_environment_probe_data`, `set_lighting`, `set_sky_preset`, `set_exposure_compensation`, AO and shadow controls should be exercised from the native host/Vibe. Restore lighting through these native interfaces.
- **Atmospheric studies:** `radiance.solve_sdfgi_froxel` exists and consumes an image plus atmospheric/GI parameters; useful for production studies. Do not claim invoking this image operation automatically wires interactive world fog/GI. Use existing live native atmospheric rendering for the scene.
- **Motion:** existing kinetics interpolation/NLA and native scene-animation APIs can carry accepted effects. Prioritise restrained movement that helps scale and presence over unrelated demo particles.

## Next image comparison

Restore terrain mapping/blended colour, bank rocks and coherent native forest density first; then calibrate environment lighting and cameras. Capture campsite, river and canopy at comparable framing with overlays both open and closed. Accept only when the native images recover depth, surface detail, readable materials and ecological composition at least equal to the earlier reference. Counts, strict replay and correct native architecture remain necessary but cannot close this visual regression.

Scope: only this audit document changed. No runtime, asset or scene edits. There was no separate `campsite.js` in the inspected staged world directory; campsite loading is implemented in `scene.js::loadCampsite` and `nativeWoodland`.
