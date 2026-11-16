# Native living-world host

This application owns the world in Rust/WASM. Its page needs only the generated `webizen_world.js` loader and `await init()`. There is no authored JavaScript scene, input controller, camera evaluator, picking implementation or frame loop.

## Inputs

- `canvas#native-world`, optional `#native-world-ui` and `#native-world-status`; the host creates missing elements.
- `/world/living-world.vibe` (override with canvas `data-vibe`). The actual `PoetSnapshot::eval_fn` evaluates `world_settings`, `input_camera`, `select_place` and `observer_path` on load or user actions. The frame loop never evaluates Vibe.
- `/publication/current.json` identifies a native Q42VOL v4 volume and SHA-256. `WasmSliceQ42Reader` reads the volume and embedded lexicon; the host decodes entity metadata, absolute positions, camera eyes, relationships and seven sanitised journey chapters. It never substitutes the JSON publication projection for Q42.
- `/production/native-world/scene.json` (override with canvas `data-scene`), schema `webizen.native-world/v1`, `assets:[{id,url,colour:[r,g,b,a],sha256}]`, optional `triangle_count`, `vertex_count` and `water:{positions,indices}`. URLs are same-origin, asset hashes are mandatory, transforms are already authored into the native assets. The host uses existing native `load_scene_assets`; native MAT1 materials survive scene assembly.

## Native behaviour

Rust owns fetch/verification, DOM controls, pointer/touch/wheel/keyboard input, orbit placement, camera interpolation, animation/reduced motion, the requestAnimationFrame lifecycle and receipts. Declared purpose is passed to Vibe with relationships decoded from Q42; it changes presentation and never authority. Returned routes must preserve every public place exactly once.

Q42 place positions become a native Tensor10D buffer using `write_tensor_buffer`. This replaces the generic tensor markers without resetting the scene mesh. Existing GPU picking returns a tensor index, which is resolved through the same graph order; mesh semantic picks are separately recorded. The ambient particle field is disabled. Marker offset/style is authored in Vibe world settings.

The native water pass consumes the compiler's actual water geometry. Vibe controls sky, sun, water appearance and camera policy. The full-scene upload is checked against authored triangle counts; the native compact fallback is not accepted as the complete artwork. Device/browser rendering quality still requires visual QA.

## Build

Run `tooling/native-host/build.ps1`. It reuses the existing QualiaDB WASM target cache, pins Rust 1.96.1 and SIMD128 with an 8 MiB stack / 1 GiB WASM memory ceiling, compiles offline with the lockfile, and invokes the installed wasm-bindgen 0.2.129. Pass another matching executable using `-Bindgen` if required. This does not overwrite the deployed QualiaPortal package or a live Synth binary.

Output: `site/pkg/native-world/webizen_world.js`, `webizen_world_bg.wasm` and generated declarations. The generated loader remains unedited. `tooling/native-host/build-receipt.json` records source/lockfile/tool/output hashes and explicitly leaves runtime visual acceptance to browser QA.

Source modules: `graph.rs` owns native graph decoding; `policy.rs` owns checked Vibe conversion/camera records; `runtime.rs` owns application state and frame lifecycle; `runtime/boot.rs`, `ui.rs` and `input.rs` own their respective boundaries. Each module is below 500 lines.

## Existing platform integration points inspected

- QualiaDB `crates/qualia-core-db/src/render/portal/mod.rs:1968`: native multi-asset scene entry, preserving coordinates with `set_preserve_authored_frame(true)` at line1001. Entry x/y/z fields are ignored; loading another batch replaces the combined mesh.
- `render/portal/mod.rs:502`: native tick; line522 native pick queue; line557 node-index result; line662 mesh semantic identity.
- `render/portal/mod.rs:853`: directional sun; line925 IBL probes; line938 native water geometry; line1008 sky presets; line2357 presented-frame scene receipt.
- `render/portal/scene_animation.rs:282`: native clip selection; line309 reduced motion; line313 seek. Authored skeletal sections are consumed during native scene assembly.
- `render/gpu/mesh_upload.rs:278`: GPU storage is approximately 64 bytes per vertex plus12 bytes per triangle and material uniforms, admitted through the shared VRAM ledger. CPU geometry/normal workspace and staging are additional costs.
- `render/portal/scene_mesh.rs:22`:4096 material-range cap; line270 rejects conflicting records sharing an ID; line319 caps skeletal actors at256 with42MiB animation residency.
- `render/gpu/portal_gpu/render_frame.rs:296`: current instance stream repeats every combined-mesh draw. It is not an independently addressable per-part instancing interface.
- `render/gpu/mesh_upload.rs:391`: actual Etalon `MeshBuffers::upload`; `portal_gpu/init.rs` owns Etalon device/queue/pipelines.
- `q42/volume/wasm_slice_reader.rs:42`: native byte-backed Q42 open; line301 collects verified pages; line210 lexical lookup. `QualiaPortal.load_q42` is a different legacy CBOR-LD helper and is deliberately not used here.

The previous JavaScript `docs/js/qualia-shell.js` provides a useful reference for lifecycle ordering (GPU initialisation must precede portal construction), but is not a runtime dependency of this native host.


## Renderer admission and art direction

The material-rich world requires successful native WebGPU initialisation. The host records the original thrown error (or an honest ambiguous false outcome) in `window.__webizenNativeInit`, and runs the existing native graphics probe only after failure. It then presents the reading view instead of a white WebGL2 scene: the current native WebGL2 multi-asset upload does not bind MAT1 materials. Successful receipts include `graphics_initialisation` and the actual evaluated `presentation_settings`.

Optional Vibe `world_settings` fields bind existing native APIs: `exposure_ev` (-8..8), `sky_colour` (three linear RGB values 0..1), `environment_fallback` (three radiance values 0..16) and `markers_visible` (boolean). Marker picking remains native when visual projection is disabled. `environment_fallback:[0,0,0]` disables the default constant [0.18,0.22,0.26] additive environment contribution; it does not disable direct sun or material-dependent ambient light. Native daylight sky preset1 uses fog density0.0004 per world unit; preset0 uses no fog. A custom clear colour retains the selected atmosphere profile.

Descriptions and camera help are inside a collapsed native HTML details element, while the selected place title, reading link, purpose controls and journey controls remain available. Rust source is ASCII-checked during build receipts to prevent shell-induced mojibake; Q42 public content remains Unicode.

Touch zoom buttons call the same native Vibe input_camera action as wheel/keyboard input. Selecting a place passes its Q42-derived camera and actual viewport aspect to fit_view; Vibe owns portrait fitting and the distance clamp. No world coordinates are changed.


## First native proof laboratory

Rust creates `details#native-proof-lab`, four instrument sections and shared structured results. `/data/publications/aaa/catalogue.json` supplies `webizen.proof-data/v1` dataset descriptors and actual sibling Q42 hashes. The reader verifies the digest, reads every quin and native lexicon, pages the complete data table, executes the same native mini-parser/bytecode VM as the original WASM query workload, and exports actual result rows/receipts. The single-pattern UI deliberately refuses SPARQL or extra tokens rather than silently running only a prefix. Exact decoded rows are compared with published fixtures where supplied; all fixtures include expected counts.

Native rights controls call compile_norm_quin/evaluate_deontic_contract for an explicitly authored public scenario (aggregate permit, individual prohibition, withdrawal and expiry). They do not claim to establish the visitor's credentials or legal compliance. Native Earth/sky controls call WGS84 conversions, Julian date/Earth rotation and analytical Sun/Moon ephemerides. The result identifies model/units and UTC-as-UT1 approximation. They are actual calculations, not observed astronomical measurements.

Benchmarks run three untimed warm-ups and5..200 measured repetitions of an unchanged published query. Timings include compile, VM, native result buffer/statistics construction; they exclude data acquisition, decode, DOM, browser yields and scene rendering. Correctness is checked before/during timing; exact raw samples, median and nearest-rank p95 are exported. The Rust frame loop pauses rendering for the measured job. Cancellation takes effect between bounded native query calls and retains completed samples. No competitor performance claim is made.

Current proof data profile:32MiB per native Q42,100,000rows, complete lexicon scratch bounded by input bytes; excess fails instead of silently sampling. Native texture profile:128dependencies,4MiB each,128MiB aggregate; actual MAT1 used-material references must match hydrated hash/role/cutoff and valid vertex-aligned UV01. Textured scenes refuse generic material-pipeline fallback.

Optional Vibe `environment_probes` is a list of up to four25-number arrays: position3,radius1,irradiance3,five specular RGB levels15,dominant direction3. The host validates finite values/positive radius and invokes the existing native probe API alongside the separate environment_fallback value.

`tooling/native-host/proof-checks.rs` links the actual cached native QualiaDB library to run the same proof computation source. Its output is separate from WASM browser evidence; use the browser instrument to validate the compiled browser target. No instrument/scene is artistically approved merely because this harness passes.
