# Handover: Webizen Website, GitHub QualiaDB, WebGPU + WebGL2

**Date:** 2026-11-17  
**Packet:** I5 retarget + texel-fit WASM. Not qualified. P00 is the only accepted packet. Pixel 10 Pro XL remains `not_executed`.  
**Repository:** `C:\github\webizen-website` (`https://github.com/mediaprophet/webizen-website.git`)

---

## 1. Current reality (read this first)

The isolated WebGL2 worktrees are **merged and retired as a build source**.

| Former worktree | Fate |
|---|---|
| `C:\github\_worktrees\etalon-webgl2` (`webgl2-storage-emulation`) | Folded into Etalon `0.0.41.3`. HAL texel-fit is `c0bddc9650b9265d1a5578c251af1ab12896601a`. |
| `C:\github\_worktrees\qualiadb-webgl2` (`webgl2-portal`) | Folded into github QualiaDB `0.0.41.3`; current HEAD `f286b01f9dc194906b021f20679a2b7fd2ea94f2` (I0/I1/I2A/I4Q + scene debug). |

The website now path-deps **github QualiaDB**, not the stale worktree:

```toml
qualia-core-db = { path = "C:/github/qualiaDB/crates/qualia-core-db", default-features = false, features = ["portal"] }
vibe            = { path = "C:/github/qualiaDB/crates/vibe" }
imap-proto      = { path = "C:/github/qualiaDB/vendor/imap-proto" }          # [patch.crates-io]
vibescript-abi  = { path = "C:/github/qualiaDB/crates/vibescript-abi" }     # [patch."https://github.com/mediaprophet/vibe-script.git"]
```

`tooling/native-host/receipt.py` fingerprints `C:/github/qualiaDB/crates/qualia-core-db` and requires local Etalon HEAD `c0bddc9650b9265d1a5578c251af1ab12896601a`. World-host `[patch."https://github.com/mediaprophet/etalon.git"]` path-patches every etalon/naga crate to `C:/github/etalon`. Measured cargo-metadata (wasm32-unknown-unknown, 2026-11-17): **24** resolved `etalon*`/`naga`/`naga-types` crates with `source: null` under `C:/github/etalon`, and **0** `etalon.git` lock providers.

QualiaDB HEAD `f286b01f9dc194906b021f20679a2b7fd2ea94f2` carries the I0 pin, I2A live-backend getter (`portal_gpu_backend`), and scene/water debug snapshots. I5 compiles against those bytes.

---

## 2. Backend policy (honest)

**WebGPU is preferred. WebGL2 is the required fallback.** `?webgl2=1` is a **force-GL probe**, not the default path.

QualiaDB `PortalGpu::try_new_async` (`crates/qualia-core-db/src/render/gpu/portal_gpu/constructors.rs`):

1. **`?webgl2=1` in the URL** → GL-only instance. No WebGPU attempt and no WebGPU retry.
2. **Default** → WebGPU-only instance (`Backends::BROWSER_WEBGPU`). Adapter/device run **before** claiming the presentation canvas.
3. If that WebGPU adapter/device/surface path fails **and** abort is not set → **one** GL-only retry (`Backends::GL`), using a canvas surface first (WebGL2 requires it) and `adapter.limits()` (pairs with Etalon `DownlevelFlags::READ_ONLY_STORAGE_EMULATION`).
4. Abort flags still win at every step.

The website does **not** guess the backend from `navigator.gpu`.

- I2A: `portal_gpu_backend()` reads the pending/adopted `PortalGpu` adapter info and returns `"webgpu"`, `"webgl2"`, or `None`.
- I2B: `native/world-host/src/runtime/graphics.rs` copies that getter into `selected_backend` after `portal_init_webgpu` returns `Ok(true)`. Anything else / `None` is JSON `null`.
- `boot.rs` uses `graphics_init["selected_backend"].as_str().unwrap_or("")`. A null getter does **not** name WebGPU. The mobile-weight scene loads when the honest backend is `"webgl2"` **or** viewport `max-width: 900px`.

QualiaDB `portal` enables Etalon `"webgpu"` **and** `"webgl"` (plus `"fragile-send-sync-non-atomic-wasm"`). Storage-buffer vertex reads on GL go through Etalon read-only storage emulation.

---

## 3. Scenes

[`site/index.html`](../site/index.html) canvas:

- `data-scene="/production/lush-woodland/scene.json"` — desktop / WebGPU.
- `data-scene-mobile="/production/lush-woodland/scene-mobile.json"` — WebGL2 or `max-width: 900px`.

| Scene | Assets | Bytes | Source triangles | Expanded triangles |
|---|---:|---:|---:|---:|
| `scene.json` | 77 | 58,637,584 | 440,626 | 7,593,414 |
| `scene-mobile.json` | 76 | 47,166,968 | 309,098 | 1,646,594 |

The mobile scene drops the 131k-triangle near eucalyptus; it is the intended GL / small-viewport weight, not a second renderer.

---

## 4. I5 WASM rebuild

`site/pkg/native-world/` is rebuilt in this packet from `native/world-host` against github QualiaDB:

- `CARGO_TARGET_DIR=C:\github\webizen-website\target-production-core-wasm`
- `CARGO_BUILD_JOBS=1`
- `cargo build --target wasm32-unknown-unknown --release --offline --locked` (lock rewritten to QualiaDB path deps and local Etalon path patches at `c0bddc9`)
- wasm-bindgen **0.2.129** at `C:/Users/Hp/AppData/Local/.wasm-pack/wasm-bindgen-06a9b89f25ab66ba/wasm-bindgen.exe`
- `python tooling/native-host/receipt.py --output site/pkg/native-world --bindgen <that exe>`
- `python tooling/copy_aaa_review.py --check`

Do **not** use `tooling/native-host/build.ps1` as-is: it still points `CARGO_TARGET_DIR` at `C:/github/qualiaDB/target`, which must stay untouched. `C:/github/synth/target` and `target-studio` are also off-limits.

Served identity is `site/pkg/native-world/runtime-identity.json`. Browser QA must compare served WASM bytes with that digest.

Served identity after texel-fit (not qualified):

- `webizen_world_bg.wasm` **21,625,060** bytes, SHA-256 `33773443af31ec31b2d87605435ca99d138125a92fb49df8ffd20ac138473ab1`
- Etalon `c0bddc9650b9265d1a5578c251af1ab12896601a` via local path patches (`source_kind` records that; 24 wasm providers / 0 git lock providers)
- QualiaDB HEAD `f286b01f9dc194906b021f20679a2b7fd2ea94f2`
- `?gl-debug=1` loads `/world/gl-debug.js` before WASM (integer counters after a 64-upload / 32-draw sample cap)
- Desktop `?webgl2=1` after texel-fit presents the habitat (trees, camper, solar, benches, river). Pixel 10 Pro XL remains `not_executed`.

This packet does **not** mark the bundle qualified.

---

## 5. Already on `main` (historical, still true)

GitHub Pages deploy, Synth-authored creativity loader, and lush woodland homepage scene remain as previously pushed. Loader is driven by `#native-world-status`; Skip (`Escape`) and `prefers-reduced-motion` work. Local review used `http://127.0.0.1:8811/index.html`. Provenance pins are still enforced by [`tooling/copy_aaa_review.py`](../tooling/copy_aaa_review.py) (`--check`) and [`.gitattributes`](../.gitattributes) (`* -text`).

---

## 6. Remaining work — I6 mobile evidence (Pixel 10 Pro XL)

**I6 target device:** **Pixel 10 Pro XL, Chrome** (WebGPU disabled on that device). That is the live WebGL2 evidence target. Desktop Chrome headless has previously kept `navigator.gpu` active despite `--disable-features=WebGPU`, so it is not a substitute for the phone.

I6 should:

1. Serve the I5 `site/pkg/native-world/` bundle (GitHub Pages or local `tooling/world_server.py`).
2. Confirm default boot prefers WebGPU on a WebGPU browser and records `selected_backend: "webgpu"`.
3. Confirm `?webgl2=1` forces GL (`selected_backend: "webgl2"`) and loads `scene-mobile.json`.
4. On Pixel 10 Pro XL Chrome: default path should fail WebGPU, take the one GL retry, report `"webgl2"`, load the mobile scene, and render without a fabricated backend.
5. Capture `__webizenNativeInit` and the served `runtime-identity.json` WASM digest. Do not mark qualified from this handover.

Known caveats for I6 (do not paper over):

- If WebGPU `request_device` succeeds and **then** surface configure / `from_device` fails, the presentation canvas already has a WebGPU context; the GL retry will typically see a null `webgl2` context. Adapter/device failure *before* claim is the retried case.
- After `QualiaPortal` adopt, `portal_gpu_backend()` uses the stash-time label. Recovery that drops `self.gpu` without `portal_abort_webgpu` can still report the last armed backend.
- `~7.6M` expanded triangles remain heavy on desktop WebGPU; the mobile scene is `~1.65M`. Frame-time on Pixel 10 Pro XL is evidence, not a pass/fail invented here.
- P00 only. Not qualified.
