# Native architecture correction and acceptance contract

Status: implementation correction in progress; not acceptance of the entire website overhaul.

Reviewed 16 November 2026 against the staged sources. Stage root: `C:/github/synth/production/webizen-swarm`. This document changes no runtime, asset, publication, benchmark or deployment file.

## Authoritative requirement

The user requires the experience itself to demonstrate the implemented platform. Synth-produced geometry displayed by an authored JavaScript world is insufficient. The living world must use the generated WASM loader only at the browser application boundary; native Rust/WASM, actual Vibe evaluation, native Q42 data and QualiaPortal/Etalon own its behaviour and rendering.

The permitted page bootstrap is the generated module import and `await init()`. Do not hand-edit generated bindings or put scene construction, camera policy, picking, navigation, animation or a frame loop into authored JavaScript. HTML and CSS remain appropriate for the page shell and accessible presentation. Rust `web_sys`/`js_sys` bindings are browser interoperability, not a substitute JavaScript world.

This correction does not remove the user's broader requirements: an entire site overhaul; an explorable natural world intersecting with the infosphere; human centric computing as an ecosystem; demonstrable support for human rights, needs and purposes; information, socioeconomic and knowledge geometry; coverage of modules across repositories; real demonstrations and benchmarks; extensibility; internationalisation with Australian English (`en-AU`) as default; and cinematic, carefully reviewed artwork. A technically native but visually weak scene does not satisfy the art brief.

## Current source-backed architecture

| Boundary | Current implementation | Required evidence |
|---|---|---|
| Browser entry | `site/native.html` imports `/pkg/native-world/webizen_world.js` and calls `init()`. No authored world controller is imported there. | Inspect the actual served entry and module/network graph, including the intended default route. A separate native page alone does not prove replacement of the previous entry. |
| Native host | `native/world-host/src/lib.rs` starts `runtime::boot`; `runtime.rs` and `runtime/{boot,input,ui}.rs` own loading, controls, input, camera interpolation, frames and receipts. | Generated output matches current source and lockfile hashes; loading and interaction succeed in the browser. |
| Native graph | `graph.rs` opens Q42VOL v4 with `WasmSliceQ42Reader`, reads quins and the embedded lexicon, and derives places, relationships and sanitised journey chapters. `boot.rs` verifies the volume SHA-256 from `/publication/current.json`. | Reject a changed digest, malformed volume, missing/ambiguous properties and unsupported schema. Demonstrate the selected place and purpose route came from this volume. |
| Vibe behaviour | `policy.rs` evaluates actual `PoetSnapshot::eval_fn`; `site/world/living-world.vibe` defines world settings, camera input, place selection and observer paths. Graph quins populate the host snapshot. | Record source digest and evaluation identity; exercise the functions. Vibe evaluation occurs on load/actions, not every rendered frame. |
| Native artwork | `production/` contains source recipes, native exports, `.10d`, HMC packages, production/sealing receipts, exact-face adapters and reviewed previews. The whole-world compiler in `production/native-world/` assembles native assets and native material records. | Follow provenance from source recipe through accepted native faces/materials into the delivered scene. Source/production validation and final assembled visual QA are separate gates. |
| Native scene upload | `boot.rs` preserves authored coordinates, verifies asset hashes, then calls `QualiaPortal::load_scene_assets`. It rejects refused assets and discrepancies between uploaded/admitted and manifest geometry counts. | A full-scene runtime receipt and visual inspection; never describe a silently reduced fallback as the full artwork. |
| Native picking | Q42 place positions become a `Tensor10D` buffer through `write_tensor_buffer` and `upload_tensor_buffer`. Native picking returns a tensor index mapped through the same graph order. Visible marker offset/style come from Vibe. | Click visible markers at more than one place and confirm their Q42 identities and navigation. Test miss/occlusion and keyboard alternatives. |
| Native environment | QualiaPortal receives sky, sun, shadows/AO settings and compiler-supplied water geometry; appearance comes from Vibe settings. | Review the actual active backend and visible result; enabling a setting is not evidence of quality or successful execution. |
| Reading and controls | The host provides native-managed DOM controls and a link to `/publication/reading.html`; `site/native.html` has a skip link, canvas label and no-script reading link. | Keyboard, focus order, mobile layout, reduced motion, text scale and complete non-3D access. |

The current host uses `QualiaPortal::load_scene_assets`, not the legacy `load_q42` CBOR-LD helper. Q42 identity is not encoded into floating-point tensor coordinates. Marker identity stays in the native graph-index mapping. Current mesh picking returns instance semantics; putting IDs solely into MAT1 submesh records does not by itself establish distinct picks for the merged scene.

`native/world-host/README.md` and `tooling/native-host/build-receipt.json` describe the build. The inspected receipt identifies Rust 1.96.1, wasm-bindgen 0.2.129 and output/source hashes, and explicitly says `runtime_visual_acceptance: pending browser QA`. These are build facts, not claims of runtime acceptance. At this review, the final `production/native-world/scene.json` was not yet present at its configured staging path; the compiler's source and intermediate outputs were present. Recheck the final manifest during integration rather than treating this snapshot as a permanent absence.

## Correction scope and truthful status

The old files under `site/world/` and `site/lab/` include authored JavaScript presentation. They remain migration/reference material until their disposition is recorded. Their presence on disk does not make them part of the native entry, but the production entry must demonstrably stop loading the old world.

The earlier `plans/swarm/IMPLEMENTATION-STATUS.md` P06 statement that the main scene uses Three.js describes the pre-correction implementation. The integrator should update that ledger only after confirming the served entry and accepted native scene. Preserve the history and do not relabel old Three.js screenshots as native evidence.

None of this completes all 18 work packages. In particular, the full route migration, complete module explanations, human-facing audience journeys, all technical exhibits, native laboratory, complete localisation, accessibility qualification, performance/device matrix, final art approval and publication remain independently gated. P02 acceptance and individual asset acceptance do not imply acceptance of P03/P06/P10/P13/P14/P16/P17/P18.

Keep the dedicated website repository decision and dependency provenance explicit. Staging in Synth is production/workspace organisation; it does not prove that `webizen-website` has been created remotely, that sources are cleanly versioned there, or that `webizen.au` has been published. No external publication is part of this document.

## Preserve demonstrations and benchmarks during the correction

The preservation baseline is authoritative for what must survive the overhaul:

- `migration/baseline/routes.json`: 204 HTML source routes, with disposition and proposed replacement records. This count includes historical/design documents and generated previews; it is not 204 functioning demonstrations.
- `migration/baseline/tests.json`: 44 suite registrations, not individual test totals or passes.
- `migration/baseline/runners.json`: 15 captured runner source files.
- `migration/baseline/benchmarks.json` and `evidence/baseline/benchmarks/`: five immutable historical datasets stored by SHA-256. Historical engine labels and measurements remain historical evidence, not fresh comparisons.
- `evidence/baseline/summary.json`: source coverage, 16 module groups and 114 unresolved source-reference candidates. A candidate is not automatically a broken public route; built/runtime triage is required.

Run the existing baseline checker at acceptance: `python tooling/baseline/build_baseline.py --check`. It checks source coverage, recorded hashes and preserved benchmark bytes. Do not regenerate away an unexplained difference. An intentional new source snapshot must retain the prior evidence.

Legacy tests and benchmark runners may remain on their separately identified preserved routes while the new native world is integrated. The loader-only requirement applies to the new world architecture; it is not permission to delete existing working JS/WASM benchmark harnesses before replacements are qualified. Label retained legacy runners accurately. Linking to one from the world is preservation/access, not evidence that the runner has been recreated as a native spatial exhibit.

The current `site/lab/runners/graph-benchmark.js` performs actual QualiaDB WASM queries, validates the full input and operations, uses ten warm-ups and thirty timed samples, and records raw samples, correctness, runtime and input hashes. Its synthetic IRI adapter repairs the canonical 60-bit hash input and records original and corrected hashes. Its two-hop workload is two sequential linked point queries, not a SPARQL join. Its timed scope includes synchronous query execution and JSON decoding; dataset preparation and correctness checking are outside the intervals. Preserve these distinctions.

The corrected browser series is separate from historical, daemon, CPU and GPU series. Do not compare incompatible results on a single leaderboard without matching dataset, query semantics, engine build, preparation, timing boundaries and environment. Do not create competitor values or infer performance from scene frame rate.

A future native Infosphere Lab must call actual native test/query/production capabilities and retain workload identity, source/runtime versions, raw evidence, correctness, cancellation and explicit unsupported/skipped results. An instrument-shaped sculpture is artwork until it invokes a real operation. Moving a recorded chart into the world is presentation of recorded evidence, not a live rerun. Preserve download/readable access and isolate benchmark execution from scenic rendering and competing device workloads.

## Acceptance gates and handoff ownership

| Gate | Required result | Owner/reviewer |
|---|---|---|
| Architecture | Served default/review world has generated loader bootstrap only; native source owns controller/render lifecycle; no imported Three.js/old world path. | Native host agent; integrator verifies served resources. |
| Inputs and replay | Actual Q42 volume/digest and Vibe evaluator verified; asset hashes and strict Synth replay receipts validated. Q42 semantic reproducibility is distinguished from timestamp-dependent byte reproducibility. | Contract/data agent plus independent verifier. |
| Complete scene | Final manifest exists; all assets admitted; manifest counts equal runtime counts; MAT1 identities do not collide; reviewed native props and campsite assets are included. | Native compiler agent; host agent verifies admission. |
| Interaction | Native markers select correct Q42 places; declared-purpose paths retain public places and never grant authority; input, resize and miss handling work. | Host agent; browser reviewer. |
| Visual experience | Fresh native desktop/mobile screenshots show recognisable campsite, natural woodland/catchment, coherent architecture, readable materials, useful camera framing, depth and lighting. User review remains substantive. | Art reviewer and integrator; user artistic review. |
| Preservation | Baseline check passes; routes/dispositions and workload links remain available; known triage stays visible; historical records remain unchanged. | Preservation agent; integrator. |
| Real laboratory | Each migrated instrument produces actual results and evidence; original correctness semantics survive; no mock timings, simulation claims or replaced workloads. | Demo/benchmark agents; independent result review. |
| Human access | Useful ordinary-language content, keyboard and reading path, reduced motion, text scaling, mobile usability and locale fallback; `en-AU` default. RTL and full translation coverage remain explicit work. | Accessibility/editorial/localisation agents. |
| Reliability | Cold start, repeated navigation, sustained rendering, GPU loss and low-resource devices tested; fallback is honest and usable; performance measurements have recorded conditions. | Integration/performance agent. |
| Release | Route migration and repository/deployment provenance reviewed, approved final build identified, no unintended exposure of private journey information. | Integrator; release authorisation as required. |

A handoff includes changed owned paths, source/build/output digests, executed checks, current browser evidence, remaining failures and exact package status. Compile success, HTTP 200, an asset count or a beautiful isolated preview cannot substitute for these gates.

## Independent visual risks from the inspected source

1. **Memory and startup:** the host fetches assets sequentially, permits 64 MiB per asset and 256 MiB aggregate, and creates JS typed-array copies before combined native upload. Decoded geometry, normals, materials, staging and GPU storage add to encoded bytes. A bounded byte budget does not guarantee mobile admission or acceptable opening time. The strict count check correctly refuses reduced geometry; a smaller separately authored native scene may be needed for constrained devices.
2. **Backend differences:** WebGPU initialisation falls back to WebGL2, but the same UI settings are not proof of identical lighting, AO, water, picking or quality on both. Capture and qualify each actual backend; do not claim parity from the fallback branch alone.
3. **Material and composition:** the native assembly must preserve the palette/MAT1 table and distinct surface response. Individual `.10d` neutral previews establish shape, not the final world colour or cinematic lighting. Check gold bodywork, charcoal roof box, pale eucalyptus bark, leaf transparency/coverage, water and terrain at normal viewing distance.
4. **Discoverability and occlusion:** native markers are small depth-tested projected points. A uniform three-metre offset can still sit inside tall canopy geometry or behind terrain. Test each place from its practical approach; retain native controls and reading navigation. Merged-mesh instance picks do not automatically give per-object identity.
5. **Camera and mobile framing:** current Vibe settings provide one initial camera, with an initial canvas minimum of 640 by 480 before resize. CSS panels occupy a substantial fraction of a small display. Verify that the camera, marker target, canvas aspect and overlays leave the campsite and navigation visible on initial mobile load.
6. **Motion and stability:** camera interpolation and native animation honour the reduced-motion flag, but water/other shader motion and sustained frame behaviour need separate observation. Device interruption currently stops the loop with a reload/reading message; recovery is not yet seamless.
7. **Human language and accessibility:** a native engine does not make the experience understandable. The current English controls and fallback are a starting point, not all-language coverage or a completed accessibility review. Technical receipts belong in optional evidence views, not the primary explanation of human purposes.

## Sources inspected

Paths below are relative to the stage root unless qualified:

- `site/native.html`, `site/world/native.css`, `site/world/living-world.vibe`.
- `native/world-host/README.md`, `src/lib.rs`, `src/runtime.rs`, `src/runtime/boot.rs`, `src/graph.rs`, and build receipt `tooling/native-host/build-receipt.json`.
- `plans/swarm/IMPLEMENTATION-STATUS.md`, `plans/swarm/work-packages.json`.
- `migration/baseline/README.md`, `migration/baseline/benchmarks.json`, `evidence/baseline/summary.json`.
- `site/lab/runners/graph-benchmark.js`.
- QualiaDB: `crates/qualia-core-db/src/render/portal/mod.rs`, `render/portal/scene_mesh.rs`, `render/gpu/portal_gpu/{picking,tensor_data}.rs`, `tensor/buffer_export.rs`.
- Etalon: `crates/etalon-render/src/shaders/{projector,mesh}.wgsl` for native marker and instance-picking behaviour.
