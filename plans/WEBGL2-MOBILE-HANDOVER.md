# Handover: Webizen Website, GitHub Pages, Creativity Loader, Lush Woodland & WebGL2 Mobile Path

**Date:** 2026-11-16  
**Repository:** `c:\github\webizen-website` (`https://github.com/mediaprophet/webizen-website.git`, branch `main`, HEAD `562e25d`)

---

## 1. Completed & Pushed Work (`main` @ `562e25d`)

1. **Repository Initialization & Provenance Pin Preservation (`eb67f06`, `a0d99e3`)**
   - Pushed `webizen-website` to `https://github.com/mediaprophet/webizen-website.git` on `main`.
   - Added [`.gitattributes`](../.gitattributes) (`* -text`) so Windows/Linux Git checkouts preserve exact byte-for-byte line endings and SHA-256 provenance pins verified by [`tooling/copy_aaa_review.py`](../tooling/copy_aaa_review.py) (`--check`).
   - Updated [`.gitignore`](../.gitignore) and [`site/pkg/native-world/.gitignore`](../site/pkg/native-world/.gitignore) so local compiler outputs, backup snapshots, and intermediate artifacts are excluded while the reviewed `site/pkg/native-world/` WASM bundle and scene assets remain tracked.

2. **GitHub Actions & GitHub Pages Deployment (`aeec7de`, `a0d99e3`)**
   - Workflow: [`.github/workflows/pages.yml`](../.github/workflows/pages.yml) (runs on pushes to `main` and `codex/**`, or `workflow_dispatch`).
   - Static bundler: [`tooling/build_pages.py`](../tooling/build_pages.py) assembles `_site/` matching [`tooling/world_server.py`](../tooling/world_server.py) routes, keeps all binary/JSON/Vibe/Q42/10D/WASM files byte-exact, and injects a subpath shim + relative URL rewriting so the site works under `/webizen-website/` on `github.io` as well as at a root domain.
   - Verified green deploy on [run 43854437265](https://github.com/mediaprophet/webizen-website/actions/runs/43854437265) (`https://mediaprophet.github.io/webizen-website/`).
   - *(Note: Run for `562e25d` was triggered on push and had not yet been polled when quota paused.)*

3. **Synth-Authored Creativity Loader (`562e25d`)**
   - [`production/loader/build_loader_art.py`](../production/loader/build_loader_art.py) uses Synth (`hypermedia.locus.*` and `hypermedia.glyph.project_svg`) to produce [`site/world/loader-art.json`](../site/world/loader-art.json) (`strict_replay_verified: true`).
   - [`site/world/loader.js`](../site/world/loader.js) and [`site/world/loader.css`](../site/world/loader.css), wired into [`site/index.html`](../site/index.html), animate the 9-stage sequence ("Every mark is a beginning": cave hand stencil, clay wedges, song wave, spiral, moveable type, orbit, DNA helix, web network, "Your turn") driven by `#native-world-status`, with Skip (`Escape`) and `prefers-reduced-motion` support.
   - Local desktop WebGPU browser review on `http://127.0.0.1:8811/index.html` confirmed the loader appeared, progressed through assets (`... of 77`), faded out cleanly, and showed 0 console errors. (A local review server may still be running on port `8811`.)

4. **Lush Woodland Homepage Scene (`562e25d`)**
   - [`production/lush-woodland/build_trees.py`](../production/lush-woodland/build_trees.py) authors two new Synth-built eucalyptus prototypes without QEM foliage stripping:
     - `eucalyptus-lush-mid`: 13,048 triangles, 231 foliage clumps.
     - `eucalyptus-lush-near`: 131,528 triangles, 5,280 individual pendulous leaves + 341 foliage clumps.
   - [`production/lush-woodland/assemble_world.py`](../production/lush-woodland/assemble_world.py) evaluates placements via `vibe_eval` (preserving camp/river/sight-line exclusions and `terrain_height`), producing [`production/lush-woodland/scene.json`](../production/lush-woodland/scene.json) with **286 trees** (7 near, 279 mid), 43 shrubs, 214 grass tufts (169 mid, 45 near), and 29 leaf-litter patches (889 ecology placements total; 77 assets; 58,637,584 bytes; 440,626 source triangles; 7,593,414 expanded triangles).
   - [`site/index.html`](../site/index.html) points `#native-world` to `data-scene="/production/lush-woodland/scene.json"`.
   - **Follow-up tuning:** On a fast connection the loader still takes ~9–15 seconds before fading out; the user has not yet asked to shorten it, and ~7.6M expanded triangles will be heavy on mobile/WebGL2 GPUs (see W1 below).

---

## 2. In-Progress / Unfinished Work: WebGL2 Mobile Support (Pixel 10 Pro XL / Chrome)

**Symptom:** Does not work on **Pixel 10 Pro XL, Chrome**, which has WebGPU disabled.  
**Root Cause:**
- [`native/world-host/src/runtime/graphics.rs`](../native/world-host/src/runtime/graphics.rs) calls `portal_init_webgpu(canvas)`.
- In QualiaDB (`crates/qualia-core-db/src/render/portal/mod.rs` and `src/render/gpu/portal_gpu/constructors.rs`), `PortalGpu::try_new_async` gates on `has_webgpu()` and only creates an Etalon instance with `etalon::Backends::BROWSER_WEBGPU`, requesting `webgpu_minimum_limits()`.
- QualiaDB's `portal` feature in `crates/qualia-core-db/Cargo.toml` enables `etalon` with `"webgpu"` but not `"webgl"`.
- Even with `etalon`'s `webgl` feature enabled, WebGL2 reports `max_storage_buffers_per_shader_stage = 0` and lacks `VERTEX_STORAGE` / `COMPUTE_SHADERS`, whereas QualiaDB's vertex shaders read 3 read-only storage buffers:
  1. `mesh_instance_layout` (`crates/qualia-core-db/src/render/gpu/portal_gpu/init.rs:328`) — `array<GpuInstanceRecord>`
  2. `tensor_storage_bind_entry` (`crates/qualia-core-db/src/render/gpu/resources.rs:118`)
  3. `ambient_bind_entries` (`crates/qualia-core-db/src/render/gpu/resources.rs:153`)
  *(Plus optional compute kernels in `crates/qualia-core-db/src/render/gpu/compute.rs` and an empty timing pass in `gpu_timing.rs:150`.)*

### Approved Plan & Isolated Worktrees Already Created

The user **approved** the Etalon-first WebGL2 plan and authorized creating/pushing new branches and updating QualiaDB's Etalon pin without disturbing existing uncommitted work in `C:\github\etalon` (`codex/production-integration-20261113`) or `C:\github\qualiaDB` (`0.0.41.2`).

**Worktrees are already created and clean (no code edits made in them yet):**
- **Etalon worktree:** `C:\github\_worktrees\etalon-webgl2`
  - Branch: `webgl2-storage-emulation` at `a326f82` (the exact rev pinned in QualiaDB's `Cargo.toml`)
  - Suggested Cargo target dir: `C:\github\_worktrees\target-etalon-webgl2`
- **QualiaDB worktree:** `C:\github\_worktrees\qualiadb-webgl2`
  - Branch: `webgl2-portal` at `cb8c731c2`
- **Detailed Plan Artifact:** `C:\Users\Hp\.gemini\antigravity\brain\2433ee84-2bd9-4874-9f24-8d89f49c738c\webgl2-plan.md`

### Remaining Steps for Next Agent

1. **Phase E1 — Etalon (`C:\github\_worktrees\etalon-webgl2`, branch `webgl2-storage-emulation`) — DONE & PUSHED (`22d5a88c65d74bc76a4a6d52ab1f1ba77f596fd0`)**
   - `etalon-types/src/limits.rs`: Added `DownlevelFlags::READ_ONLY_STORAGE_EMULATION = 1 << 27`.
   - `naga/src/back/glsl/mod.rs`, `features.rs`, `writer.rs`: Added `Options::emulate_readonly_storage_with_textures`, `EmulatedStorageMapping` on `ReflectionInfo`, GLSL ES 3.00 lowering of `var<storage, read>` globals to `highp usampler2D` + `_naga_fetch_u32` (`texelFetch`, bounds-clamped) + `_naga_load_ty_*` recursive helpers (scalars, vectors, matrices, fixed-size arrays, structs, `ArrayLength`), and rejection of writable storage buffers.
   - `etalon-core/src/binding_model.rs`, `device/resource.rs`: When `DownlevelFlags::READ_ONLY_STORAGE_EMULATION` is present, `BufferBindingType::Storage { read_only: true }` bypasses `VERTEX_STORAGE`, counts against `max_sampled_textures_per_shader_stage`, and uses the `MAX_TEXTURE_SIZE^2 * 16` binding-size bound when `max_storage_buffer_binding_size == 0`. Writable storage buffers remain rejected.
   - `etalon-hal/src/gles/adapter.rs`, `mod.rs`, `device.rs`, `command.rs`, `queue.rs`: Advertises `READ_ONLY_STORAGE_EMULATION` when `!supports_storage`, pads `STORAGE` buffer allocations to 16 bytes, tracks `write_generation` on `Buffer` writes (`ClearBuffer`, `CopyBufferToBuffer`, `CopyTextureToBuffer`, `CopyQueryResults`, `unmap_buffer`), maintains per-binding shadow `RGBA32UI` textures uploaded GPU-side via `PIXEL_UNPACK_BUFFER` + `texSubImage2D`, and binds texture slots + `_width`/`_byte_size` uniforms.
   - Tests & checks: `naga` `emulated_storage_tests` (2 passed), `etalon-gpu` `vertex_storage_instanced` (1 passed on GL), `cargo check -p etalon --target wasm32-unknown-unknown --no-default-features --features webgl,webgpu,wgsl` (clean), `CHANGELOG.md` updated, pushed to `origin/webgl2-storage-emulation`.

2. **Phase Q1 & Q2 — QualiaDB (`C:\github\_worktrees\qualiadb-webgl2`, branch `webgl2-portal`) — DONE & PUSHED (`78ccab7618edaf542f1e8784ee38207e1849af48`)**
   - Pinned all `etalon*` / `naga` workspace dependencies in `Cargo.toml` to `22d5a88c65d74bc76a4a6d52ab1f1ba77f596fd0`.
   - Enabled `"webgl"` and `"fragile-send-sync-non-atomic-wasm"` alongside `"webgpu"` on `etalon` in `crates/qualia-core-db/Cargo.toml`.
   - Updated `portal_init_webgpu` (`src/render/portal/mod.rs`) to remove the `!has_webgpu()` early return, and updated `PortalGpu::try_new_async` (`src/render/gpu/portal_gpu/constructors.rs`) to use `etalon::util::new_instance_with_webgpu_detection(BROWSER_WEBGPU | GL)` and `adapter.limits()` when `Backend::Gl`.
   - Verified `cargo check -q --target wasm32-unknown-unknown -p qualia-core-db --no-default-features --features portal` passed with exit code 0; pushed to `origin/webgl2-portal`.

3. **Phase W1 — Website (`c:\github\webizen-website`) — BUILT & PUSHED**
   - Updated `native/world-host/Cargo.toml` to use `C:/github/_worktrees/qualiadb-webgl2` (`webgl2-portal` @ `78ccab7618edaf542f1e8784ee38207e1849af48`).
   - Updated `native/world-host/src/runtime/graphics.rs` and `boot.rs` to request `webgpu-or-webgl2`, record `selected_backend` (`webgpu` / `webgl2`), and load `data-scene-mobile` when on WebGL2 or viewport `<= 900px`.
   - Generated [`production/lush-woodland/scene-mobile.json`](../production/lush-woodland/scene-mobile.json) (76 assets, 47,166,968 bytes, 309,098 source triangles, 1,646,594 expanded triangles; 55 `lush-mid` trees, no 131k-tri near tree) and wired `data-scene-mobile` in [`site/index.html`](../site/index.html).
   - Rebuilt `site/pkg/native-world/webizen_world_bg.wasm` (20,033,401 bytes, SHA-256 `601a67814a5aa2c4b593aa21e8645998c4a103c471c33f07c0a1b1ae52c9d740`), updated `runtime-identity.json` and `tooling/native-host/receipt.py` (pinned to Etalon `22d5a88c65d74bc76a4a6d52ab1f1ba77f596fd0`), and verified `python tooling/copy_aaa_review.py --check` passes.
   - **Remaining verification:** Test on **Pixel 10 Pro XL (Chrome)** once GitHub Pages deploys. Note: desktop Chrome headless kept `navigator.gpu` active despite `--disable-features=WebGPU` / `--disable-blink-features=WebGPU`, so live WebGL2 shader/render verification on a non-WebGPU browser is still pending.
   - Pin `etalon` (and workspace etalon crates if needed) to the pushed `webgl2-storage-emulation` commit.
   - Enable Etalon `"webgl"` feature alongside `"webgpu"` in `crates/qualia-core-db/Cargo.toml`.
   - Update `PortalGpu::try_new_async` (`src/render/gpu/portal_gpu/constructors.rs`) and `portal_init_webgpu` (`src/render/portal/mod.rs`) to use `etalon::util::new_instance_with_webgpu_detection` with `Backends::BROWSER_WEBGPU | Backends::GL`, remove `has_webgpu()` as the hard gate, and opt into `READ_ONLY_STORAGE_EMULATION` + adapter limits on GL.
   - Gate compute pipelines (`compute.rs`, `gpu_timing.rs`), `Rgba16Float` HDR (`probe_hdr_format`), and shadow/AO formats on actual adapter/device capabilities, recording any degradations in the frame receipt.
   - Verify `cargo check --target wasm32-unknown-unknown -p qualia-core-db --no-default-features --features portal`, commit, and push `webgl2-portal`.

3. **Phase W1 — Website (`c:\github\webizen-website`)**
   - Point `native/world-host/Cargo.toml` to the updated QualiaDB (or commit), record `selected_backend` (`webgpu` / `webgl2`) in `native/world-host/src/runtime/graphics.rs`, and add a lighter mobile/WebGL2 scene option (fewer distant trees, lower triangle budget).
   - Rebuild `site/pkg/native-world/`, update `runtime-identity.json` pins, verify `python tooling/copy_aaa_review.py --check`, test in desktop Chrome with WebGPU disabled, push to `main`, and verify on Pixel 10 Pro XL Chrome.
