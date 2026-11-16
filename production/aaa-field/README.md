# Native light-field proof

Run `python build.py`, then `./build-admission.ps1`, then `python finalise.py` from this directory. The first step invokes the actual Synth production host and strict replay. All outputs use fresh production paths. The second compiles the current native Portal decoder source with existing QualiaDB codec libraries, admits the actual file, checks exact-source preservation and refuses a corrupt payload. No engine code is changed.

`manifest.json` is the public integration contract. Promote only that file, `light-field.vibe`, `admission.json` and the five files in `assets/`. Production receipts, HMC and native validation sources remain reproducibility evidence.

This 24³ field is an authored microscopic model, not campsite measurement. Input wavelengths are 450/530/650 nm, spacing125 nm. The native snapshots are typed v4 E/B vectors. Current spectral-slice and volume previews are electric-field magnitude grayscale, not physical colour. All four PNGs were visually reviewed: coarse cell blocks are intentionally visible. Do not present this bounded scientific proof as finished cinematic artwork, a Maxwell solver, calibrated spectrum or sonification.
