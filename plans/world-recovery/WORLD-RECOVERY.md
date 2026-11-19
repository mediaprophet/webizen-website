# World recovery — work packets

Date: 19 November 2026. Locale: Australian English.
Review: [REVIEW-2026-11-19.md](REVIEW-2026-11-19.md).
Machine register: [work-packets.json](work-packets.json).

This programme replaces the accepted meaning of EWM v2 waves 9–10. It does not delete that code. It makes the public site and the campsite world match the outcome already sought: a continuous inhabited catchment, trees in the ground, a person on that ground, and a road the Passat can drive to the community grounds.

## Outcome

A visitor who opens `https://webizen.au/` with no query string gets a living world, not the reading fallback.

In that world, from the camp camera:

1. The ground is soil, litter, rock, creek, and a living understorey. Kangaroo grass, tussock, and mat-rush are on the catchment, not a bare shade.
2. Each woody plant is fitted once to the soil contour. Every root-skeleton joint keeps its authored depth below the soil height at that joint's own `(x, z)`, so a lateral that runs up a rise or across a gully stays under that surface. After the fit, the root base is rigid. Wind, which moves the trunk, boughs, and leaves, pulls against those frozen anchors. The rhizosphere control is the only view that draws the roots. Grasses use the same fit at tussock scale: the crown of the tuft sits on the soil, the root tuft stays under it, and the blades move from that fixed base.
3. A person stands and walks on that ground and reads as a person at camp distance.
4. A forestry track leaves the camp and becomes a short public approach into the community grounds. The surface under the Passat is that road, not Grass, for the whole drive.
5. The Passat can hitch the Jayco and be driven along that road to the grounds. Trees stop the vehicle. The chase camera stays with the car.
6. WebGL2 presents the same layout with lower detail. The reading view still works with no GPU.
7. None of this fetches `/intenteco/` or a third-party host during bare-URL boot. `streaming.default` stays false. `art_approval` stays false until the user says otherwise.

No commit, no push, and no `wrangler deploy` unless the user asks in this session.

## Swarm

One integrator and at most three specialists at a time. A supervisor runs after every wave and is not the author of that wave.

| Role | Model | Writes |
| --- | --- | --- |
| `site-boot` | `grok-4.7` | WR-00 only |
| `ground` | `grok-4.7` | WR-01 only |
| `flora` | `grok-4.7` | WR-02 only |
| `road` | `grok-4.6` | WR-03 only |
| `character` | `grok-4.7` | WR-04 only |
| `drive` | `grok-4.7` | WR-05 only |
| `packer` | `grok-4.6` | WR-06 only |
| `integrator` | `grok-4.7` | integrator routes for that wave only |
| `visitor-review` | `grok-4.7` | evidence notes only, read-only on source |
| `supervisor` | `grok-4.7` | the wave verdict only |

`grok-4.7` is for packets where the picture, the runtime, or the verdict can lie. `grok-4.6` is for the road mesh and the static bundle, against a written checklist the supervisor re-checks. If a workflow launch rejects a model id, inherit the session model and record that in the wave note. Do not silently downgrade the supervisor or the visitor review.

Specialists edit only `owned_paths`. They do not run a WASM release, do not edit `boot.rs` after WR-00, and do not flip `streaming.default`. The integrator is the only agent who compiles `webizen-world-host` for wasm32.

WASM command, from `native/world-host`, one job:

```
$env:CARGO_TARGET_DIR='C:\github\webizen-website\target-production-core-wasm'
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='--cfg getrandom_backend="wasm_js"'
cargo build --target wasm32-unknown-unknown --release --offline --locked -j 1
```

Do not use `tooling/native-host/build.ps1`. Do not run a second cargo against `qualia-core-db` or `webizen-world-host` while that build is in flight. Do not edit `render/gpu/emf_pipeline.rs` or the EMF/QAC owners.

Isolation worktrees do not merge themselves. This swarm runs in the shared tree with disjoint paths. Prompts must be self-contained.

## Implementation

Use the meshes and skeletons already in `C:\github\eco-intent`. Synth is available (`synth_operations`, 150 morphisms; produce through `synth_produce_present`). It is the tool for the terrain mesh and for a revision when a camp still fails. It is not a reason to rebuild the River Red Gum. The host frame loop does not evaluate Vibe. Placement and wind are runtime poses.

The soil function lives in `ground.rs`, written by WR-01 and only read by everyone else.

`height_at(x, z)` and `normal_at(x, z)` sample the terrain mesh, within 5 cm. `fit_soil_joint(trunk_xz, yaw, joint_xyz)` places one authored joint:

- Rotate the joint's `(x, z)` by the instance yaw and add the trunk position.
- Set world `y` to `height_at` at that point, plus the joint's authored `y`.
- Authored `y` is depth below the flat soil plane of the asset. The collar is `y = 0`. Roots are negative. A lateral tip 9 m out sits `authored depth` below the soil at the tip, not below the collar, and not on one tilted disc.

`compute_slope_conforming_root_transform` is retired as the placement path. A unit test that locks the disc is replaced by a contour test: a rise under one lateral and a gully under the opposite lateral, with every fitted joint still below the soil.

After the fit, root joints are stored and never integrated. `inv_mass` stays 0. Wind integrates only joints above the collar. Amplitude and damping come from `wind_and_root_mechanics` in the species `actuation.vibe`: the fitted plate supplies the anchorage, the boughs and leaves move, and a second sample of the root joints must be identical. Grasses use the same function at tuft scale. The tuft base freezes. The blades rotate about that base.

The River Red Gum root graph to fit is the one already lofted in `source/eucalyptus-camaldulensis.vibe`: `taproot_sinker`, seven `primary_lateral_root_*` parts, and the deeper orders hung off them. The XPBD anchors `taproot_deep_anchor` and `lignotuber_root_plate` are the rigid bodies. `bole_collar` and everything above it is the wind chain. The same split exists, at smaller depth, on `poa-labillardierei`, `themeda-triandra`, and `lomandra-longifolia`.

## Waves

Wave 0 — make the current world reachable. Packet WR-00, then integrator WASM if `boot.rs` changed, then supervisor.

Wave 1 — the soil mesh, the fit function, and the grasses. Packet WR-01. Integrator points locomotion, vehicle, and placement at `ground::height_at`. Supervisor looks at soil, rock, creek, and grasses.

Wave 2 — fit the woody root skeletons and build the road, in parallel. WR-02 and WR-03. WR-02 calls `ground::fit_soil_joint` and does not edit `ground.rs`. Integrator compiles the scene. Supervisor looks at a contoured root, a frozen root under wind, and the track.

Wave 3 — the existing person, and a drive along the road, in parallel. WR-04 and WR-05. Integrator. Supervisor walks and drives.

Wave 4 — put the accepted bytes where Cloudflare can serve them. WR-06, then WR-08 builds `_site` and probes it with no eco-intent mount. Visitor review WR-07 runs against that static server, WebGPU and WebGL2. Supervisor fail-closed.

Agent budget for a full run: 48. That covers five waves, two-wide parallel, one integrator, one supervisor, and one visitor review, with a small retry margin. A rejected supervisor verdict stops the swarm. Restart at `start_wave`.

## Acceptance that is not a toast

Every visual packet saves PNGs under `evidence/world-recovery/<packet-id>/` and a note that quotes what is in the picture. The picture has to be the world-host canvas in a browser, after wasm-bindgen has written `site/pkg/native-world`, and the note has to name that wasm sha256 and the local URL. A frame from `render_evidence.rs`, a joint plot, or a note that says the frame is not the canvas is a fail. The supervisor opens those PNGs. A pass is refused when any of these are true:

- Bare URL on a server rooted at `_site` shows the reading fallback or the `?ewm=1` message.
- `_site/publication/world.q42` is missing.
- Bare-URL boot requests any `/intenteco/` or `githubusercontent.com` URL.
- Normal view shows root wood above the soil, on flat ground, on a slope, or where a lateral crosses a rise or a gully.
- A root plate is only a single tilted disc. The laterals do not follow the soil contour.
- Wind moves the root base, or the crown sways with no rigid root anchor.
- The camp ground has no grasses.
- Rhizosphere view matches the normal view.
- The corridor has no drivable surface, or the HUD stays on Grass while the car is on the track.
- The person is missing, or does not read as a person at about 8 m.
- The drive does not reach the community grounds on the road.
- `streaming.default` is true, or `art_approval` was set true.
- The note says PASS and the PNG disagrees.
- The PNG is an offline plot, or the note does not name the served wasm sha256.

`python tests/ewm/run.py` from `C:\github\webizen-website` stays green. New checks live under `tests/world-recovery/` and do not weaken the old honesty asserts.

## Deploy

WR-08 proves `_site`. Publishing to webizen.au is a separate user instruction. The last push already deployed `50a23b9`; another push will deploy again through `.github/workflows/cloudflare.yml`.
