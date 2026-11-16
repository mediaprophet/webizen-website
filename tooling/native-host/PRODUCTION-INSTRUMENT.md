# Native production instrument

The sixth laboratory tab, Make with Synth, invokes the same-origin native Synth service. No authored JavaScript implements production, scene controls or orchestration.

Client contract: POST `/api/produce` with `source`, `profile: files`, `include_html: false`, `include_resources: true`, `include_package: true`. The client requires strict replay, checks every resource SHA-256 and length, checks HMC SHA-256 and length, then offers PNG/SVG previews, explicit WAV playback and downloads. Unknown resource kinds remain downloadable bytes. Returned HTML is not inserted.

Bounds: 64 KiB edited source, 32 MiB response, 32 resources, 8 MiB per resource, 16 MiB combined resources and 16 MiB package. Discard invalidates the pending client result; it does not pretend to interrupt server computation.

Four recipes exercise ten actual morphisms. Each default recipe was produced twice with identical HMC hashes. Each changed-parameter variant also completed through native production. `data/publications/aaa-production/validation.json` records native outputs; `parameter-validation.json` records changed inputs. The default sound is exactly one second at 48 kHz, with 440 positive zero crossings. The catchment is explicitly illustrative geometry, not a measured boundary.

Browser qualification after the joint build:

1. Open the laboratory and `native-lab-tab-production` on the same-origin native website.
2. Choose each `native-production-recipe` and click `native-production-run`; verify actual POST response, receipt source/digests and resource previews/downloads. Expected file counts: catchment 2, lettering 1, tone 1, surface 3.
3. Change a parameter, produce again, and compare the receipt/source and output digests. Audio must not play automatically.
4. Expand source, enable `native-production-custom`, and exercise an invalid source; expect an intelligible production refusal without replacing the last valid output or losing world navigation.
5. Use `native-production-cancel` during a pending request; the status must say result discarded, not server cancelled.
6. Confirm generated WASM loader only, actual HMC downloads, and world camera interaction after production.

Selective website data handoff: `production-handoff.json`. The recipe files and recorded validation are proof fixtures; live browser previews always come from the newly executed POST response, never these saved files.
