# EWM v2 (`WEB-WP-09`) — Final Dual-Backend Build, Test & Visual QA Gate Report

- **Date:** 2026-11-19
- **Auditor:** `dual-backend-qa-gate`
- **Repository:** `C:\github\webizen-website`
- **Overall Gate Verdict:** **`PASS WITH NOTES`**

---

## 1. Fresh WASM World-Host Build (`tooling/native-host/build.ps1`)

| Field | Verified Value |
|---|---|
| Command | `powershell -ExecutionPolicy Bypass -File C:\github\webizen-website\tooling\native-host\build.ps1` |
| Toolchain / Bindgen | `rustc` (`RUSTUP_TOOLCHAIN=1.96.1`), `wasm-bindgen 0.2.129` |
| Exit Code | **`0` (PASS)** (`0` errors, `12` dead-code warnings in `webizen-world-host`) |
| `webizen_world_bg.wasm` Size | `20,688,516` bytes (`~19.73 MiB`) |
| `webizen_world_bg.wasm` SHA-256 | `d5b8126e518927104ea21457c9ab59941845fbaa683eccd139ade10238b9043c` |
| `webizen_world.js` SHA-256 | `3e1f26217f93c5630a700441028c5c2c6cc467a126a77f70fbcf3804b4e26e7a` |
| `host_source_fingerprint` | `b52e2a4766bc0338e58319c99ca3682c228b798e592b90b044dd6abe77f8fb79` |
| Qualified Bundle | `site/pkg/native-world/qualified/1e3cc809d314247a87f8b3df6a314bdfc52fd341b1e54bf21201b449778bb109` |
| Receipt | `site/pkg/native-world/runtime-identity.json` |

---

## 2. Automated Test Suites, Q42 Catalogs & Anti-Monolith Checks

### 2.1 Existing EWM Contract Suite (`tests/ewm/run.py`)
- **Result:** **`PASS`** — **`72` ran, `0` failed** (`0` regressions).

### 2.2 Pure Rust Unit Tests (`native/world-host/src/runtime/`)
Executed standalone with `rustc --edition 2021 --test` (`RUSTUP_TOOLCHAIN=1.96.1`):

| Module | Tests Run | Result | Notes |
|---|---:|---|---|
| `runtime/graphics.rs` | 8 | **PASS (8/8)** | Tier A/B/C classification, `?backend=webgl2` cap, DPR clamp, mobile viewport heuristic |
| `runtime/frame_telemetry.rs` | 4 | **PASS (4/4)** | Bounded window, invalid sample refusal, `DeviceTelemetry` + reading fallback |
| `runtime/admission.rs` | 6 | **PASS (6/6)** | Tier 1/2 mirror ordering, SHA-256 + 5 container magics, traversal/cleartext rejection |
| `runtime/stream/selection.rs` | 6 | **PASS (6/6)** | Frustum hysteresis, Tier A/B/C LOD caps, rhizosphere pairing (`13/13` incl. `unload`) |
| `runtime/stream/unload.rs` | 7 | **PASS (7/7)** | Tier A/B/C byte & triangle budgets, pinned-asset LRU eviction, incremental delta |
| `runtime/instances.rs` | 4 | **PASS (4/4)** | Exact 64-byte `#[repr(C)]` `InstanceState64`, deterministic unit quaternion, ontogenetic scaling |
| `runtime/instruments/species.rs` | 5 | **PASS (5/5)** | 5 fauna locomotion classes, reduced-motion, opt-in spatial `.qac`, rhizosphere X-Ray, escaped HTML card |
| **Subtotal Executed** | **40** | **PASS (40/40)** | **0 failures** |
| `runtime/ocs_frames.rs` | 6 | **BLOCKED ON HOST** | Compiles cleanly in `wasm32-unknown-unknown`; host `cargo test --lib` blocked in upstream `qualia-core-db` (see Open Notes #2) |
| `runtime/locomotion.rs` | 10 | **BLOCKED ON HOST** | Compiles cleanly in `wasm32-unknown-unknown`; host `cargo test --lib` blocked in upstream `qualia-core-db` (see Open Notes #2) |

### 2.3 Q42 Vault Verification (`synth` MCP `query_sparql`)

| Vault Path | Expected Quins | Measured Quins | Result |
|---|---:|---:|---|
| `data/ewm/species-catalog.q42` | 1,228 | **1,228** | **PASS** |
| `data/ewm/cosmos-catalog.q42` | 147 | **147** | **PASS** |
| `data/ewm/places/se-australia-catchment.q42` | 7,332 | **7,332** | **PASS** |

### 2.4 Anti-Monolith Binary Check
- Scanned `C:\github\webizen-website` recursively for any `.hmc` or `.10d` matching the 13 `eco-intent` species slugs (`eucalyptus-camaldulensis`, `eucalyptus-dalrympleana`, `eucalyptus-regnans`, `eucalyptus-melliodora`, `banksia-integrifolia`, `acacia-pycnantha`, `dicksonia-antarctica`, `wallabia-bicolor`, `phascolarctos-cinereus`, `vombatus-ursinus`, `macropus-giganteus`, `ornithorhynchus-anatinus`, `dacelo-novaeguineae`).
- **Result:** **`PASS`** — **`0`** duplicated species `.hmc`/`.10d` binaries found in `C:\github\webizen-website`.

---

## 3. Ephemeral Server (`world_server.py --port 8819`) HTTP / CORS / Range Check

Target: `http://127.0.0.1:8819/intenteco/eucalyptus-camaldulensis/dist/lods/eucalyptus-camaldulensis-lod1.10d`  
(Port `8800` and existing `synth.exe` processes were left untouched; ephemeral server on `8819` was cleanly stopped after capture.)

| Request | HTTP Status | Bytes | `Access-Control-Allow-Origin` | `Accept-Ranges` | Extra Headers | Result |
|---|---:|---:|---|---|---|---|
| `GET` (full) | `200` | `233,172` | `*` | `bytes` | `CORP: cross-origin`, `COOP: same-origin`, `COEP: require-corp` | **PASS** |
| `GET` (`Range: bytes=0-1023`) | `206` | `1,024` | `*` | `bytes, bytes` *(see Note #3)* | `Content-Range: bytes 0-1023/233172` | **PASS** |
| `OPTIONS` | `204` | `0` | `*` | `bytes` | `ACAM: GET, HEAD, OPTIONS`, `ACAH: Range` | **PASS** |

---

## 4. 4-Profile Dual-Backend Browser & Visual Verification

- **Harness:** `tests/ewm-v2/verify_dual_backend.mjs` (Node `v24.22.2` native CDP against `Chrome/157.0.8103.79` headless with WebGPU enabled)
- **Raw Telemetry:** `evidence/ewm-v2/telemetry.json`

| Profile | URL & Viewport | `device_tier` | `requested_backend` | `selected_backend` | Init (ms) | Canvas Backing | Loaded Assets / Refused | Triangles / Vertices | Asset Bytes | Console Errors / Exceptions | Visual Check |
|---|---|---|---|---|---:|---|---:|---:|---:|---:|---|
| **Profile 1: Desktop WebGPU (`TierA`)** | `/?ewm=1` (`1280×720`) | `tier-a` | `webgpu-or-webgl2` | `webgpu` | `8779.3` | `1280×720` | `24 / 0` | `137,048 / 69,018` | `37,451,336` | `1` (`/favicon.ico` 404) / `0` | **PASS** |
| **Profile 2: Desktop WebGL2 Forced (`TierB`)** | `/?ewm=1&backend=webgl2` (`1280×720`) | `tier-b` | `webgl2` | `webgpu` *(see Note #1)* | `1140.8` | `1280×720` | `24 / 0` | `137,048 / 69,018` | `37,451,336` | `0 / 0` | **PASS WITH NOTE** |
| **Profile 3: Mobile WebGL2 Touch (`TierC`)** | `/?ewm=1&backend=webgl2` (`390×844`, `dpr=2`, `mobile`, `hasTouch`) | `tier-c` | `webgl2` | `webgpu` *(see Note #1)* | `829.0` | `448×970` (`dpr` clamped to `1.15`) | `24 / 0` | `137,048 / 69,018` | `37,451,336` | `0 / 0` | **PASS WITH NOTE** |
| **Profile 4: HTML No-GPU Fallback** | `/publication/reading.html` (`1280×720`) | N/A | N/A | N/A | — | N/A | — | — | — | `0 / 0` | **PASS** |

### Visual Inspection Summary (All 4 PNGs Inspected via `read_file`)
1. **`evidence/ewm-v2/profile1-desktop-webgpu.png`:** Full lush woodland clearing rendered cleanly: gold VW Passat B7 wagon with charcoal roof box, 1980s Jayco wind-up camper with popped canvas roof, 3 solar arrays, eucalyptus canopy with dappled ground shadows, dome structure, and blue tidal vessel. Header nav, `Open the proof laboratory`, `Play`, `Show panel`, and bottom locomotion bar (`Observe`, `Walk`, `Fly`, `Rhizosphere X-Ray`, `Inspect`) render sharply with zero clipping.
2. **`evidence/ewm-v2/profile2-desktop-webgl2.png`:** Identical visual clarity and HUD layout; `TierB` budgets (`max_dpr: 1.35`, `max_resident_triangles: 320,000`, `max_gpu_memory_bytes: 201,326,592`, `max_first_paint_bytes: 17,825,792`, `max_texture_dim: 1024`, `preferred_lod: "lod1"`) enforced in `window.__webizenNativeInit`.
3. **`evidence/ewm-v2/profile3-mobile-webgl2-touch.png`:** Clean responsive portrait layout (`390×844`), canvas backing store properly DPR-clamped to `448×970` (`TierC` `max_dpr: 1.15`), locomotion pills wrap neatly into two rows without horizontal overflow. Minor framing note: a foreground eucalyptus trunk sits near center-frame in portrait aspect ratio, partially bisecting the camper behind it.
4. **`evidence/ewm-v2/profile4-html-reading-fallback.png`:** Clean, accessible semantic HTML reading publication (`2,697` text chars, `8` headings, `0` console/network errors).

---

## 5. Open Defects & Notes

1. **`?backend=webgl2` caps tier budgets (`TierB`/`TierC`) but does not force a WebGL2 GPU context when `navigator.gpu` is present (Medium):**
   - `DeviceTier::classify_with_request` in `native/world-host/src/runtime/graphics.rs` correctly enforces `TierB` (desktop) and `TierC` (mobile) capability budgets when `?backend=webgl2` is supplied.
   - However, `qualia-core-db`'s `portal_gpu_backend()` still initializes `"webgpu"` whenever Chrome exposes `navigator.gpu`, so `window.__webizenNativeInit.selected_backend` reports `"webgpu"` instead of `"webgl2"`. A follow-up in `qualia-core-db` (exposing a backend preference parameter to `init_portal_canvas`) is needed to hard-force WebGL2 on WebGPU-capable browsers.
2. **Host x86_64 `cargo test --lib` blocked in upstream `qualia-core-db` (Medium):**
   - Running `cargo test --lib --offline --locked` on Windows x86_64 fails in `C:\github\qualiaDB\crates\qualia-core-db\src\wasm_bridge\engine\mod.rs` with 9 `E0432` unresolved imports (`cas`, `crypto`, `graph`, `linalg`, `numerics`, `stats`, `stats_regression`, `transforms`, `units`) when `features = ["portal"]` is compiled for non-`wasm32` targets.
   - Because of this upstream `cfg` issue, `ocs_frames.rs` (6 tests) and `locomotion.rs` (10 tests) could not be executed via host `cargo test`, though all 40 standalone pure-Rust unit tests passed and the full `wasm32-unknown-unknown` release build succeeded with 0 errors.
3. **Minor Polish Items (Low):**
   - `/favicon.ico` returns `404` on initial page load.
   - `tooling/world_server.py` sends `Accept-Ranges: bytes` in both `do_GET` (`206` branch) and `end_headers()`, producing `Accept-Ranges: bytes, bytes` on range responses.
   - Portrait mobile (`390×844`) initial camera framing places a foreground eucalyptus trunk across the center of the camper.
   - `/publication/reading.html` renders two adjacent `"Explore the source demonstration"` links under each section.
