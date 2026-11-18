# Living-world playable (after EWM v1)

Specialised programme beside EWM v1 streaming and AAA overhaul. Id series **LW-00+**. Do not reuse A00–A27, N01–N09, or EWM-00+.

EWM v1 already streams whole HMC entries over same-origin HTTP Range, admits Q42VOL v4, and keeps `streaming.default` false. Camp P1 already unhitches: Pack up → hitch → named-place travel of the whole set; Pack up → Unhitch → named-place travel of car+operator while chassis/roof/panels/starlink/desk stay.

This programme makes the woodland **playable across places**: authored camp remaining, then **unload the resident set** when the visitor travels so a different place can present a different scene from **static files** (HTTP Range, no streaming server).

## What P1 already landed (do not re-implement)

- Generic player `native/world-host/src/runtime/macros.rs`. No `camp.rs`. No `if id == "car"`.
- Dual flags `travelsHitched` / `travelsUnhitched`, `parked[]` freeze, Unhitch action, N3 dump `data/ewm/camp-sequence.n3`.
- Headed: hitch river moves all origins; unhitch river leaves panels at field studio and moves the car.
- Catalog 85, four-tuple first-paint 24, `inner_ranges` false.

## Still missing

| Capability | State |
| --- | --- |
| Car driven with the camper left standing | **Done (P1).** Unhitch + `travelsUnhitched` on car+operator. |
| Re-hitch after a solo drive | Hitch pose is local delta. After a solo drive the car is parked away from the chassis; Pack up hitch does not require proximity and does not lerp the car back. |
| Continuous driving | Travel is still a spring lerp to a named public place. No WASD / steering. Named places stay the long-distance destinations. |
| Operator walking the pack | One button plays the Allen Meets translation chain. Operator `.10d` has no SKL1 walk clips. |
| Awning pack | Sail-track awning is static geometry inside the desk `.10d`. |
| Mechanical hitch / tow physics | Hitch is Passat translation `delta [-2.35, 0, 1.15]`, not a tow joint. |
| Place-to-place scenes | `compose_window` always inserts global `first_paint` (24 woodland organs). The campsite cannot vanish. There is no per-place Base, no unload of omitted ids, no second scene volume. |
| HTTP Range static packs | v1 Range exists. It still fetches woodland members. Other places need their own static `.10d` / `.q42` files under `/packages/` and `/data/ewm/places/`. No streaming server. No QDNF ProtectedSession. |

## Architecture (do not fork)

- **Synth** is the development tool (produce/preview/compile). **QualiaPortal + world-host** is the visitor runtime.
- Scenes, environments, animations: HMC / Q42 / Vibe. One generic player. Changing `camp.q42` or `living-world.vibe` must not require a host rebuild of the sequence.
- Author logic as **N3**. Keep `triples.json` for `q42_web.exe`. Visitors never read N3/Turtle.
- SPARQL over admitted Q42. GIS OSM/DEM/STAC later, **opt-in**, native ingest, Range the packed members.
- Add assets as **external HMC** without a host rebuild: catalog row + static file + Range.
- Operator CAD: `recipes/field-studio-operator.vibe` loft/extrude/revolve/`sweep_assembly_profile`/`compile_10d_pbr` only. Do **not** call `compile_vertebrate_anatomy_10d`. Seal with `attest.exe`.
- `streaming.default` stays **false**. `inner_ranges` stays **false**. HTTP Range is whole-entry on static `/packages/`, `/production/`, and `/data/ewm/places/`. No streaming server.
- Wasm budget starts at **23.5 MiB** and may be raised at closeout if a measured good product needs it. Four-tuple first-paint stays **24** for the field-studio place. Player stack budgets: 16 groups, 32 stages, 16 norms, 8 actions. Woodland catalog 85 may grow Enhancement rows.
- Do not kill Synth MCP or world_server `:8800`. Do not rebuild synth `target` / `target-studio`. Do not commit unless the user asks.
- Timeline.json never enters git, `_site`, HMC, or Q42. No exact campsite pins, Timeline `placeId`s, or `INFERRED_WORK`/home.

## Place unload (the next vertical after camp authored)

1. Each `PublicPlace` declares a **Base** id list (field-studio Base is today's 24).
2. `select_place` sets the live place. `compose_window` uses **that place's Base**, then overlapping Enhancement for that place. Window cap starts at 64 and may grow if a place Base plus overlapping Enhancement needs it.
3. Ids omitted from the new resident list are **v1 unload** (rebuild GPU via `load_scene_assets`). Do not wait for QualiaDB `unload_scene_parts` (EWM PR10).
4. Fetch new members with existing `FetchRangeService` whole-entry Range on static `/packages/` and `/production/` files. 206 on local/`world_server.py` and Pages; Cloudflare may still be 200 whole-object.
5. Place scene knowledge is a static Q42 (`/data/ewm/places/<id>.q42`) plus optional logic volume. Camera/sky/water appearance stays Vibe at load/place change, never per-frame.

## Later-track (not this swarm)

Tow/XPBD hitch joint, live GBIF parse, `inner_ranges`, QDNF ProtectedSession, ranged Q42 spine, `navigator.storage.persist()`, flora/fauna mass production, art review, flipping `streaming.default`.

## Wave graph

| Wave | Title | Packets | Live |
| --- | --- | --- | --- |
| wave-0 | Programme contract | LW-00 | 0 + integrator |
| wave-1 | Camp authored remainder: player, N3 facts, vibe/awning CAD | LW-01, LW-02, LW-03 | 3 + integrator |
| wave-2 | Place Base catalog, place scene volumes, SKL1 walk; then unload | LW-04, LW-06, LW-07, then LW-05 (same specialist as LW-04) | 3 + integrator |
| wave-3 | Opt-in GIS second place, headed evidence | LW-08, LW-09 | 2 + integrator |
| wave-4 | Closeout | LW-10 | 0 + integrator |

File ownership, `owned_paths`, EWM handoff, and later-track live in `plans/living-world/work-packets.json` and `plans/living-world/file-ownership.json`. Specialists edit owned_paths only. Integrator routes parent files then compiles wasm into `target-production-core-wasm`. This packet does not change runtime behaviour.

## EWM handoff

- Stream: EWM-02 v1 landed. LW-05 inherits `selection.rs` and may add `unload.rs`. `flag.rs` stays EWM-09.
- Qualification tests: EWM-08 keeps `tests/ewm/qualification/**`. LW tests live under `tests/ewm/living-world/**`.
- First-paint: EWM-00 keeps `data/ewm/first-paint.json` length 24. LW-04 adds place Base beside it.
- GIS ingest: EWM-04b default-deny stub landed. LW-08 inherits `tooling/ewm_ingest.py`.
- Packages: EWM-03 whole-entry shards. LW-06 adds place members as additional static files. `packages/current.json` stays EWM-10. `wrangler.jsonc` stays EWM-05.
