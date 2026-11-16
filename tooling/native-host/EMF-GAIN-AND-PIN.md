# EMF diagnostic gain and renderer pin

The field instrument loaded actual typed EMF cells, but its mode-1 shader returned the precomputed greyscale amplitude unchanged. The gain slider changed its receipt without changing pixels. Etalon now applies `clamp(amplitude * amplitude_scale, 0, 1)` in both the shader and CPU oracle. This changes display brightness only. It preserves scientific E/B values, valid-zero opacity and missing-sample transparency.

For the typed magnitude display the effective formula is `clamp(gain * (1 - exp(-|E| / (1 V/m))), 0, 1)`. This is a greyscale diagnostic, not calibrated spectral colour. QualiaPortal's preview receipt now describes this route accurately.

The four-file Etalon fix was committed and pushed as `785ab9c1b7d729db9afcd9b0f3fd70344b37be1e` on `codex/production-integration-20261113`. Remote verification returned the same revision. Main QualiaDB dependencies, sensing bridge and accelerator harness pins are aligned. The website host has no local Etalon path override. Its lock contains 30 provider packages across platforms; WASM metadata selects 23, all from the pushed revision.

Validation:

- `cargo test --offline -p etalon-render --lib`: 583 passed, zero failures.
- Focused actual-source CPU/ABI/shader checks: seven passed, including WGSL parsing/validation.
- QualiaDB workspace, sensing bridge and accelerator harness metadata pass with `--locked --offline`.
- Accelerator harness pin/selection regression passes.
- Full `cargo xtask test` was attempted. An escalated retry downloaded WARP/Agility dependencies successfully, then stopped because `cargo-nextest` is not installed. The official nextest runner was installed and the full suite rerun. All-feature linking then failed because MSVC ATL `atls.lib` is absent. Official ATL installation required Windows elevation, which the user cancelled; no retry or bypass was attempted. Full GPU-suite coverage is not claimed.
- Browser qualification of the new pinned WASM remains a separate gate: different gain values must change canvas pixels while exported field values remain identical; world camera interaction must still work after the second field canvas is used.

`etalon-pin-evidence.json` records exact pin files and validation logs. `build-receipt.json` records the final WASM/loader digest and complete immutable bundle directory after the build finishes. Browser acceptance is not inferred from compilation.

Pinned WASM build completed: 16,501,165 bytes, SHA-256 `6f508880c0b696c4a92d5fe91410bd137f8aec093c4c9ab6d6102c1b4698f2cf`. Runtime metadata validates the pushed provider revision. Browser pixel qualification remains pending.

Subsequent authorised validation update: the user approved retrying ATL installation. The official installation succeeded (exit 0), the previous ATL-dependent targets link, and the unchanged full GPU suite is running. This supersedes the earlier cancelled-installation blocker; suite results remain pending.
