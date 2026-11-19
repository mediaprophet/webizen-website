# EWM v2 (`WEB-WP-14`) — Wave 6 Final Live-World Dual-Backend Visual & Publishing Gate Report

- **Date:** 2026-11-19
- **Auditor:** `dual-backend-qa-gate`
- **Repository:** `C:\github\webizen-website`
- **Overall Gate Verdict:** **`PASS WITH NOTES`**

---

## 1. Fresh Release WASM Build (`tooling/native-host/build.ps1`)

| Field | Verified Value |
|---|---|
| Command | `powershell -ExecutionPolicy Bypass -File C:\github\webizen-website\tooling\native-host\build.ps1` |
| Toolchain / Bindgen | `rustc` (`RUSTUP_TOOLCHAIN=1.96.1`), `wasm-bindgen 0.2.129` |
| Exit Code | **`0` (PASS)** (`0` errors, `13` dead-code warnings in `webizen-world-host`) |
| `webizen_world_bg.wasm` Size | `21,012,937` bytes (`~20.04 MiB`) |
| `webizen_world_bg.wasm` SHA-256 | `26a918ae0b4efdb6d235721838a98fb0952bd9f5a0a2780113f4a3b6d0538cd9` |
| `webizen_world.js` SHA-256 | `558eb620734e64ed367730d4e8b98751b419717003c720eaa466a1d062bcfe4b` |
| `host_source_fingerprint` | `843c22a7c91ea7af27efc2dbcd87434573ebf9f71d39f5f191555918035933e0` |
| Qualified Bundle | `site/pkg/native-world/qualified/a07f472ab168164b73a42a8b291683ba47cb277808c7594875a182b25cddff08` |
| Receipt | `site/pkg/native-world/runtime-identity.json` |

---

## 2. Automated Test Suites & Publish Verifier

### 2.1 Host `x86_64` `cargo test --lib --offline --locked`
- **Command:** `cargo test --manifest-path C:\github\webizen-website\native\world-host\Cargo.toml --lib --offline --locked`
- **Result:** **Exit `0` (Compiled Cleanly; `0` tests executed — see Note #1)**
  - `WEB-WP-11`'s `qualia-core-db` fix completely unblocked host `x86_64` compilation of `qualia-core-db` and `webizen-world-host` (`0` errors).
  - However, `native/world-host/src/lib.rs:1` has crate-level `#![cfg(target_arch = "wasm32")]`, so host `x86_64` `cargo test --lib` builds an empty test harness (`running 0 tests`) until `#![cfg(target_arch = "wasm32")]` is narrowed to wasm-only items.
  - Standalone `rustc --edition 2021 --test` verification from Wave 4 (`graphics.rs` 8, `frame_telemetry.rs` 4, `admission.rs` 6, `stream/selection.rs` 6, `stream/unload.rs` 7, `instances.rs` 4, `instruments/species.rs` 5) remains **`40 / 40 PASS`**.

### 2.2 Existing EWM Contract Suite (`tests/ewm/run.py`)
- **Result:** **`70 / 72 PASS` (`2` stale pre-Wave-5 WASM digest assertions)**
  - `FAIL test_headed_evidence.py::test_headed_ledger_records_wasm_and_refuses_art_acceptance` — pre-Wave-5 headed evidence ledger records an older `webizen_world_bg.wasm` SHA-256.
  - `FAIL test_download.py::test_qualification_stays_honest` — pre-Wave-5 qualification JSON records an older `webizen_world_bg.wasm` byte length (`bytes_today`).
  - All `70` functional, Q42, Range, OPFS, admission, selection, and rollback contract tests passed with **`0` regressions**.

### 2.3 Species Cohort Publish Verifier (`production/species-cohort/verify_intenteco_publish.py`)
- **Result:** **`PASS`** (exit `0`)
  - **`13/13 repos verified | 53/53 LODs verified | q42=OK | anti-monolith=OK`**
  - All 7 flora repos (`5/5` LODs each, `1.44–1.53 MiB` total, max LOD `908–996 KiB`) and 6 fauna repos (`3/3` LODs each, `420–726 KiB` total, max LOD `296–593 KiB`) verified clean; git state reports `git-init-needed` (ready for publishing).

---

## 3. Ephemeral Server (`world_server.py --port 8819`) HTTP / Header / Favicon Check

(Port `8800` and existing `synth.exe` processes were untouched; ephemeral server on `8819` was cleanly stopped after capture.)

| Endpoint & Method | HTTP Status | Bytes | `Access-Control-Allow-Origin` | `Accept-Ranges` | Extra Headers | Result |
|---|---:|---:|---|---|---|---|
| `GET /favicon.ico` | **`200`** | `5,430` | — | `bytes` | `Content-Type: image/x-icon` | **PASS** |
| `GET /intenteco/eucalyptus-camaldulensis/dist/lods/eucalyptus-camaldulensis-lod1.10d` | **`200`** | `233,172` | `*` | `bytes` | `CORP: cross-origin`, `COOP: same-origin`, `COEP: require-corp` | **PASS** |
| `GET` same (`Range: bytes=0-1023`) | **`206`** | `1,024` | `*` | **`bytes`** *(single header!)* | `Content-Range: bytes 0-1023/233172` | **PASS** |
| `OPTIONS` same | **`204`** | `0` | `*` | `bytes` | `ACAM: GET, HEAD, OPTIONS`, `ACAH: Range` | **PASS** |

---

## 4. Live 4-Profile + 2 Interactive Bonus Browser & Visual Verification

- **Harness:** `tests/ewm-v2/verify_dual_backend.mjs` (Node `v24.22.2` native CDP against `Chrome/157.0.8103.79` headless with WebGPU enabled)
- **Telemetry JSON:** `evidence/ewm-v2/telemetry.json`

| Profile | URL & Viewport | `device_tier` | `requested_backend` | `selected_backend` | Init (ms) | Canvas Backing | Assets / Refused | Triangles / Vertices | Console Errors / Exceptions | Visual Check |
|---|---|---|---|---|---:|---|---:|---:|---:|---|
| **Profile 1: Desktop WebGPU** | `/?ewm=1` (`1280×720`) | `tier-a` | `webgpu-or-webgl2` | **`webgpu`** | `9754.8` | `1280×720` | `24 / 0` | `137,048 / 69,018` | `4` *(Tier-2 DNS offline)* / `0` | **PASS WITH NOTE** |
| **Profile 2: Desktop WebGL2 Forced** | `/?ewm=1&backend=webgl2` (`1280×720`) | `tier-b` | `webgl2` | **`webgl2`** | `12641.1` | `1280×720` | `24 / 0` | `137,048 / 69,018` | **`0 / 0`** | **PASS** |
| **Profile 3: Mobile WebGL2 Touch** | `/?ewm=1&backend=webgl2` (`390×844`, `dpr=2`, `mobile`, `hasTouch`) | `tier-c` | `webgl2` | **`webgl2`** | `2049.3` | **`448×970`** (`dpr` clamped `1.15`) | `24 / 0` | `137,048 / 69,018` | `5` *(Tier-2 DNS offline)* / `0` | **PASS WITH NOTE** |
| **Profile 4: HTML Reading Fallback** | `/publication/reading.html` (`1280×720`) | N/A | N/A | N/A | — | N/A | — | — | **`0 / 0`** | **PASS** |

### Visual Inspection of All 6 Captured PNGs (Read via `read_file`)
1. **`evidence/ewm-v2/profile1-desktop-webgpu.png`:** Lush woodland clearing rendered via WebGPU (`WebGPU · Tier A` badge). Gold VW Passat B7 wagon, 1980s Jayco wind-up camper, 3 solar arrays, dome, tidal vessel, and eucalyptus woodland canopy with ground shadows. All Wave 5 HUD controls visible: `Explore Places` tour menu (6 stops: `Campsite Clearing`, `River Red Gum & Platypus Creek`, `Mountain Ash Canopy`, `Rhizosphere X-Ray`, `Kangaroo Meadow`, `Earth & Solar Orbit`), `Discovered 0 / 13 species`, `Sound: Muted`, `Play`, `Show panel`, and bottom locomotion bar (`Observe`, `Walk`, `Fly`, `Rhizosphere X-Ray`, `Inspect`). Zero `/favicon.ico` 404s.
2. **`evidence/ewm-v2/profile2-desktop-webgl2.png`:** **TRUE FORCED WebGL2 verified!** Badge reads `WebGL2 · Tier B`, `window.__webizenNativeInit.selected_backend === "webgl2"`, `0` console errors, `0` JS exceptions, crisp rendering of the campsite and woodland clearing.
3. **`evidence/ewm-v2/profile3-mobile-webgl2-touch.png`:** **TRUE FORCED WebGL2 Mobile Tier C verified!** Badge reads `WebGL2 · Tier C · Mobile`, `selected_backend === "webgl2"`, canvas backing store clamped to `448×970` (`dpr 1.15`). **Portrait framing fix confirmed:** the obstructing foreground eucalyptus trunk from Wave 4 is completely gone—the Jayco camper, 3 solar panels, gold Passat wagon, and wooden dome are cleanly framed. Time-of-day pills (`Dawn`, `Midday`, `Golden Hour`, `Dusk`), `Explore Places`, `Discovered 0 / 13 species`, `Sound: Muted`, and locomotion bar all render legibly.
4. **`evidence/ewm-v2/profile4-html-reading-fallback.png`:** Clean semantic HTML reading publication, `0` console errors, `0` HTTP errors, **`0` duplicate adjacent `"Explore the source demonstration"` links** (now clearly paired as `Explore the source demonstration · Living-world showcase` and `Explore the source demonstration · Knowledge graph`).
5. **`evidence/ewm-v2/profile5-rhizosphere-xray.png` (Bonus Interactive):** Clicking `Rhizosphere X-Ray` highlighted the pill in active teal and triggered scene view update (`"Updating this view of the place — 7 of 64"`).
6. **`evidence/ewm-v2/profile6-field-guide.png` (Bonus Interactive):** Clicking `Discovered 0 / 13 species` opened the `SE Australia Field Guide` drawer with `All (13)`, `Flora (7)`, `Fauna (6)`, and `Close`. (Minor polish: `Explore Places` stayed open underneath, peeking out to the right of the drawer; drawer text showed `"Loading admitted SE Australian species catalog from Q42..."` while background streaming was in progress.)

---

## 5. Open Notes (Why `PASS WITH NOTES`)

1. **Crate-level `#![cfg(target_arch = "wasm32")]` in `native/world-host/src/lib.rs:1` (Medium):**
   - `WEB-WP-11` fixed `qualia-core-db` host compilation (`cargo test --lib` exits `0`), but `lib.rs:1` still gates the entire crate to `wasm32`, causing `cargo test --lib` on `x86_64` to report `running 0 tests`. Moving the `cfg(target_arch = "wasm32")` attribute off the crate root onto `start()` and wasm-only modules will enable all Rust unit tests (including `ocs_frames.rs` and `locomotion.rs`) to execute under `cargo test --lib`.
2. **Post-First-Paint 600m Catchment Streaming & Offline Tier-2 Mirror DNS Noise (Medium):**
   - In local offline/dev testing (`127.0.0.1:8819`), `window.__webizenEwmState` remained in `"streaming_phase": "first-paint"` (`catchment_terrain_active: false`, `streamed_species_count: 0`), and Profiles 1 & 3 logged `net::ERR_NAME_NOT_RESOLVED` when `admission.rs` fell back to Tier-2 remote mirrors (`https://assets.webizen.au/production/terrain-v2/catchment-600m-lod*.10d`, `https://r2.intenteco.org/...`) because those `terrain-v2` `.10d` files were not present under local `/production/terrain-v2/`.
3. **Stale Pre-Wave-5 Evidence Ledger Digests in `tests/ewm/run.py` (Low):**
   - `test_headed_ledger_records_wasm_and_refuses_art_acceptance` and `test_qualification_stays_honest` fail solely because the legacy evidence/qualification JSON files still record the pre-Wave-5 WASM SHA-256/byte size (`70/72` contract tests pass).
4. **Minor UI Polish (Low):**
   - Opening `SE Australia Field Guide` while `Explore Places` is open causes the `Explore Places` strip to peek out behind the drawer (`profile6-field-guide.png`). Auto-collapsing `Explore Places` when opening the Field Guide drawer will eliminate the overlap.
