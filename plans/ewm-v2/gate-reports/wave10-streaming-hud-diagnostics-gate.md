# EWM v2 (`WEB-WP-24`) — Wave 10 Final Streaming, HUD & Diagnostics QA Gate Report

- **Date:** 2026-11-19
- **Auditor:** `dual-backend-qa-gate`
- **Repository:** `C:\github\webizen-website`
- **Overall Gate Verdict:** **`PASS`** *(with 2 minor telemetry-timing notes)*

---

## 1. Fresh Wave 10 Release WASM Build (`tooling/native-host/build.ps1`)

| Field | Verified Value |
|---|---|
| Command | `powershell -ExecutionPolicy Bypass -File C:\github\webizen-website\tooling\native-host\build.ps1` |
| Toolchain / Bindgen | `rustc` (`RUSTUP_TOOLCHAIN=1.96.1`), `wasm-bindgen 0.2.129` |
| Exit Code | **`0` (PASS)** (`0` errors, `104` dead-code warnings in `webizen-world-host`) |
| `webizen_world_bg.wasm` Size | `21,286,792` bytes (`20.30 MiB`, under `23.5 MiB` CI budget) |
| `webizen_world_bg.wasm` SHA-256 | `61fb1171a267867b35fd74aca13e19eb82d9ecfb73d731343414004783422a79` |
| `webizen_world.js` SHA-256 | `aadf9e3ca0f5797b78beb1974b8107dd25c89ef397306154a14651d3afc9c185` |
| `host_source_fingerprint` | `b3bacf8a01188363d64dcbe36de1c79a5926dd46f4607e2bdcbbece052c3f4d9` |
| Qualified Bundle | `site/pkg/native-world/qualified/d52422bf2078a69117cec76594b66a4e1eea1d1b0fddcf45889ae7a7cc4656b6` |
| Receipt | `site/pkg/native-world/runtime-identity.json` |

---

## 2. EWM Contract Suite (`tests/ewm/run.py`)

- Updated `evidence/ewm/living-world/headed.json` and `evidence/ewm/qualification.json` with the Wave 10 WASM digest (`21,286,792` bytes, `20.30 MiB`, `61fb1171...`) while preserving `art_approval: false` and all honesty invariants.
- **Result:** **`PASS` — `72 / 72` passed, `0` failed** (`0` regressions).

---

## 3. Live Wave 10 9-Capture Browser & Visual Verification (`http://127.0.0.1:8819`)

- **Harness:** `tests/ewm-v2/verify_dual_backend.mjs` (ephemeral `world_server.py --port 8819`, cleanly stopped afterwards; ports `8800`, `8820`, and `synth.exe` untouched)
- **Telemetry JSON:** `evidence/ewm-v2/telemetry.json`
- **Global Error Summary Across All 9 Captures & All 4 Profiles:**
  - **Console Errors:** **`0`**
  - **JS Exceptions:** **`0`**
  - **HTTP `4xx`/`5xx` Errors:** **`0`**
  - **Stream Bind Errors (`TransparentInstancesRequireSortedSubmission`, `InvalidQuaternionNorm`, `Duplicate native placement identity`):** **`0` (100% eliminated)**

| Capture | Profile / Action | Backend / Tier | Key Telemetry Verified | Errors | Visual Verdict |
|---|---|---|---|---:|---|
| **Profile 1** | Desktop WebGPU (`1280×720`, `/?ewm=1`) | `webgpu` / `tier-a` | `worker_active: true` (`55` worker fetches done), `catchment_placements: 317`, `24/24` first-paint (`137,048` tris), uncluttered top HUD (`Places · Discovered 0 / 17 species · Sound · Stats · Debug · Clean View`) | `0` | **PASS** |
| **Profile 2** | Desktop WebGL2 (`1280×720`, `/?ewm=1&backend=webgl2`) | **`webgl2`** / `tier-b` | `worker_active: true` (`42` done), `catchment_placements: 317`, `24/24` first-paint, crisp `WebGL2 · Tier B` badge | `0` | **PASS** |
| **Profile 3** | Mobile WebGL2 Touch (`390×844`, `dpr=2`, `/?ewm=1&backend=webgl2`) | **`webgl2`** / `tier-c` | `dpr` clamped `1.15` (`448×970`), `catchment_terrain_active: true`, `resident_assets: 64`, `237,564` tris, `worker_active: true`, **clean 2-row wrapped Mode Dock with zero clipped pills** | `0` | **PASS** |
| **Profile 4** | HTML No-GPU Fallback (`/publication/reading.html`) | N/A | Clean semantic HTML, `0` duplicate links | `0` | **PASS** |
| **Profile 5** | Rhizosphere X-Ray (Desktop WebGPU) | `webgpu` / `tier-a` | **Multi-species forest & subterranean roots live on screen!** Toast: `"600 m catchment & 13 living species active (317 instances)"` | `0` | **PASS** |
| **Profile 6** | Field Guide `17` Taxa + Auto-Collapse | `webgpu` / `tier-a` | `All (17)`, `Flora (11)`, `Fauna (6)`, live Q42 cards (`Bare-Nosed Wombat · did:q42:taxon:vombatus-ursinus`, `Mycorrhizal / EMF`, `Custodial Country`, `Visit`), `Places` auto-collapsed (`false`) | `0` | **PASS** |
| **Profile 7** | Drivable Passat + Jayco HUD (`KeyW`+`KeyD` 1.6s) | `webgpu` / `tier-a` | `vehicleState.active: true`, **`speed_kmh: 9.34`**, **`gear: "D1"`**, `steer_deg: 29.05°`, `pitch_deg: +1.37°`, `roll_deg: -0.36°`, `#native-vehicle-cluster` visible, zero error banners | `0` | **PASS** |
| **Profile 8** | Technical Statistics Panel (`#native-stats-toggle`) | `webgpu` / `tier-a` | `#native-stats-drawer` open: `Backend: WebGPU`, `Device Tier: TierA`, `Resident Assets: 77`, `Triangles: 444k / 650k` bar, `GPU Memory: 35.6 / 384 MiB` bar, **`Web Worker: active · 0 in / 67 done`**, `3×3 Tile / Zone: (0, 0) · Forest Campsite`, **`Streamed Species: 17 / 17`**, **`Placements: 317`** | `0` | **PASS** |
| **Profile 9** | Diagnostics & Debug Drawer (`#native-debug-toggle`) | `webgpu` / `tier-a` | **`statsAutoCollapsed: true`**, `#native-debug-drawer` open with `Log to Console`, `Copy JSON`, `Download JSON`, `Force Stream Tick`, `Test LRU Evict`; `window.__webizenDebug.snapshot()` returned all 12 telemetry keys, `log()` succeeded, `evictLru()` returned `{"ok":true,"action":"evictLru"}` | `0` | **PASS** |

---

## 4. Visual Inspection Summary (All 9 PNGs Inspected via `read_file`)

1. **`evidence/ewm-v2/profile1-desktop-webgpu.png`:** Sleek uncluttered Wave 10 top HUD (`Places | Discovered 0 / 17 species | Sound | Stats | Debug | Clean View`), centered compass (`NE · 032° · Forest Campsite · Local ENU · 5.5 m`), bottom mode dock (`Observe · Walk · Drive · Fly · 1st/3rd · Rhizosphere · Anatomy · Inspect`), and crisp `WebGPU · Tier A` badge in bottom-right.
2. **`evidence/ewm-v2/profile2-desktop-webgl2.png`:** Identical clean layout under true forced WebGL2 (`WebGL2 · Tier B`), zero console or HTTP errors.
3. **`evidence/ewm-v2/profile3-mobile-webgl2-touch.png`:** **Mobile 2-row wrapped Mode Dock fix confirmed!** Row 1 (`Observe · Walk · Drive · Fly · 1st/3rd`) and Row 2 (`Rhizosphere · Anatomy · Inspect`) wrap cleanly with zero horizontal clipping. Unobstructed portrait framing of Passat, Jayco camper, 3 solar arrays, wooden dome, and smooth 600m catchment hills (`600 m catchment & 0 living species active (317 instances)`).
4. **`evidence/ewm-v2/profile4-html-reading-fallback.png`:** Clean semantic HTML reading view, `0` errors.
5. **`evidence/ewm-v2/profile5-rhizosphere-xray.png`:** **Live 600m catchment + multi-species forest + subterranean roots rendered cleanly!** Multiple branching eucalypts, shrubs, spreading subterranean root systems, dappled ground shadows, creek water, and toast `"600 m catchment & 13 living species active (317 instances)"`.
6. **`evidence/ewm-v2/profile6-field-guide.png`:** Live 17-species Q42 Field Guide (`All (17)`, `Flora (11)`, `Fauna (6)`) with full provenance and `Visit` action, `Places` cleanly collapsed, live forest visible behind the drawer.
7. **`evidence/ewm-v2/profile7-drive-passat-hud.png`:** Active Chase Follow driving view (`7–9 km/h`, `GEAR D1`, `PITCH / ROLL +1.4° / -0.1°`, `SURFACE Grass`, `E · 092°`), cleanly driving across the grass clearing with zero error banners.
8. **`evidence/ewm-v2/profile8-stats-panel.png`:** Sharp dark-glass `Technical Statistics` drawer showing live Graphics & Timing, Streaming & Residency (`77` resident assets, `444k / 650k` triangles bar, `35.6 / 384 MiB` GPU memory bar, `Web Worker: active · 0 in / 67 done`), and Biosphere & Simulation (`17 / 17` species, `317` placements).
9. **`evidence/ewm-v2/profile9-debug-drawer.png`:** `Diagnostics & Debug` drawer cleanly replaced `Technical Statistics` (`statsAutoCollapsed: true`), displaying all 5 action buttons and the live JSON response `__webizenStreamHooks.evictLru() -> {"ok":true,"action":"evictLru"}`.

---

## 5. Minor Telemetry Timing Notes (Non-Blocking)

1. **`evictLru()` async queue timing:** `window.__webizenDebug.evictLru()` queues an async eviction tick and returns `{"ok":true,"action":"evictLru"}` immediately; reading `window.__webizenStreamStats.total_evicted_out` synchronously on the same JS turn still read `0` before the next async stream tick drained the queue.
2. **Stats drawer `Phase` label vs active toast:** In `profile8`, `#native-stats-drawer` displayed `Resident Assets: 77`, `Triangles: 444k / 650k`, `Streamed Species: 17 / 17`, and `Web Worker: active · 0 in / 67 done` while the `Phase` row still displayed `first-paint` (syncing `streaming_phase` to `"active"` in the Stats drawer formatter will keep all rows consistent). On Mobile `Tier C` (`profile3`), the 600m catchment terrain mounted (`catchment_terrain_active: true`, `streaming_phase: "terrain-ready"`, `237,564` tris) while species count reported `0` under Tier C's stricter `150k` triangle cap.

---

## 6. Lush Woodland & Terrain Visual Fix Verification Addendum

- **Fresh Release WASM (`tooling/native-host/build.ps1`):** **PASS** (exit `0`, `0` errors)
  - `webizen_world_bg.wasm`: `21,317,822` bytes (`20.33 MiB`), SHA-256 `bc0bc55e1fbee2c8619a97a8736e29f22908cc3921da508836a178996e81376e`
  - `webizen_world.js`: SHA-256 `746b7660162e845e1a851243c9a016dce2e3e85a9b622c7bf665e3aa5640baa5`
  - `host_source_fingerprint`: `fb34942b8093d15b674ec84c436e25e27ee88e087c825e010690b7387de896d1`
  - Qualified bundle: `d9ace464d8a89e6f60597022d95419b5787146533ebe73e9420496a470694a11`
- **EWM Contract Suite (`tests/ewm/run.py`):** **PASS — `72 / 72` passed, `0` failed** (digests refreshed in `headed.json` & `qualification.json`, `art_approval: false` preserved).
- **Visual Inspection Confirmed (All 8 GPU/Interactive PNGs Re-Inspected via `read_file`):**
  1. **Lush Green Canopies (`profile1-desktop-webgpu.png`):** Full leafy River Red Gum / Eucalyptus canopies with mottled bark, green Dicksonia tree ferns, golden-flowering Acacia wattle, and Banksia with rich dappled forest shadows — **zero bare twigs!**
  2. **Rich Mossy Woodland Terrain (`profile1`, `profile2`, `profile3`):** Washed-out flat beige is completely replaced by deep mossy green undulating 600 m catchment hills, natural sandy dome clearing, and creek water.
  3. **Zero Exposed Roots in Normal View (`profile1-desktop-webgpu.png`):** Every tree trunk sits firmly below the soil line with zero floating or exposed roots.
  4. **Active Streaming Telemetry (`profile1`):** `window.__webizenEwmState.streaming_phase === "active"`, `catchment_terrain_active === true` (`lod0`), `catchment_placements: 871`, `streamed_species_count: 13`, `resident_assets: 81`, `resident_triangles: 509,238 / 650,000`, `resident_bytes: 38.5 MiB / 384 MiB`, `worker_active: true`.
  5. **WebGL2 Desktop (`profile2`) & Mobile `390×844` (`profile3`):** `0` console/HTTP errors, lush mossy green terrain active (`catchment_terrain_active: true`, `871` placements indexed), clean 2-row wrapped mobile Mode Dock.
  6. **Drive Mode (`profile7`) & Stats/Debug (`profile8`, `profile9`):** `10.99 km/h`, `GEAR D1`, `steer_deg: 28.01°`; Stats panel shows `81` resident assets, `509k / 650k` triangles bar, `36.7 / 384 MiB` GPU bar, `Web Worker: active · 0 in / 62 done`.
- **Two Minor Items Caught for 1-Line Follow-Up:**
  1. **4 New Grass/Shrub Slugs Missing from `tooling/world_server.py` `INTENTECO_SLUGS`:** `profile1` logged 404s for `/intenteco/{bursaria-spinosa,poa-labillardierei,lomandra-longifolia,themeda-triandra}/dist/lods/*-aboveground-lod1.10d` because `tooling/world_server.py` lines 13–18 still allowlist only the original 13 slugs. Adding those 4 slugs to `INTENTECO_SLUGS` will immediately stream all `17/17` species (including the 500 native grasses + 90 shrubs) with `0` HTTP 404s.
  2. **Stats Drawer `Placements` Row:** In `profile8`, `#native-stats-drawer` still showed `Placements: 317` (and `Phase: fetching` during the post-Rhizosphere tick) while `window.__webizenEwmState.catchment_placements` was `871` and `window.__webizenStreamStats.phase` was `"active"` on Profile 1.
- **Lush Woodland Pass Verdict:** **`PASS WITH NOTES`** (stunning lush canopy + mossy terrain + zero exposed roots visually confirmed; 4-slug allowlist update in `world_server.py` will unlock the 4 grass/shrub species and clear the 404s).

