# Native Passat campsite asset

This is an authored representation of a gold 2013 Volkswagen Passat B7 Variant, intended for the personal campsite scene. It is not a scan or an assertion of the owner's exact equipment specification. Numberplates are blank. Wheel styling is illustrative.

The body follows Volkswagen's published B7 Variant reference dimensions: 4771 mm long, 1820 mm wide excluding mirrors, 1508 mm high, 2712 mm wheelbase. Authored trim, mirrors and rails extend beyond the nominal body envelope; use the measured bounds in the integration manifest. Metres; Y up; +Z forward; ground at y=0. Do not normalise the car independently of the camper.

Official references:
- https://www.volkswagen-newsroom.com/en/vehicle-data-passat-b7-profile-20037
- https://www.volkswagen-newsroom.com/en/passat-b7-20102014-20036

## Production

`build_car.py` writes explicit VibeScript loft controls and invokes the current Synth `target-prod/debug/synth.exe` in an isolated MCP process. Every part is exported by the native DXF morphism. `car.json` indexes those exact exported faces without constructing substitute browser geometry. The same faces are assembled into a retained OBJ and sealed natively as `.10d`. Both production phases have strict replay receipts and HMC packages.

The first wheel-arch Boolean study left fragments visible in native review. It was rejected. The accepted body uses densely sampled arch-following loft sections. Historical create-only outputs remain as provenance but are not referenced by the integration manifest. `inputs/body-surface-reference.json` retains actual native body/cabin geometry used to fit the glazing and light controls. Those fitted controls are explicit in the final `.vibe`; the final HMC replay does not depend on the Python controller.

Use `assets/car.integration.json` for current public paths, `assets/car.json` for named geometry and material intent, and `assets/car.manifest.json` for full source/native records. Only integration-manifest referenced runs are current.

## Review

Native front, side and three-quarter PNGs are produced by Synth's native preview renderer. The native review uses its uniform preview material; the browser review shows the authored material palette. `review.html` is a separate geometry/material inspection stage, not the campsite environment. `review.cjs` serves it temporarily through a private local server and records front, side, quarter and rear browser views. It reads the existing site's Three.js files without changing them.

`validate_car.py` checks every indexed part against all native DXF faces, finite coordinates, index bounds, part hashes, `.10d`/HMC hashes, source and retained-input hashes, strict replay and catalog identity. Visual review remains separate from those numerical checks.
