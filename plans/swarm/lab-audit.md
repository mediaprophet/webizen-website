# Infosphere Lab: source audit and implementation packages

Planning audit: 16 November 2026. Read-only source inspection, not a runtime qualification or new benchmark run. Paths below are relative to `C:/github/qualiaDB` unless stated otherwise. This supplement is intended for the website implementation swarm.

## What must survive the redesign

`C:/github/webizen-website/data/demos.json` inventories **202 HTML source files**. Its rows explicitly say `source-inventory-only; not runtime-qualified`; runtime profiles and capability/owner lists are presently unassigned. The inventory includes 63 anatomy production previews, 37 historical/design pages, 11 release pages, a shared template and utility pages. Do not advertise this as 202 working demonstrations.

| Inventory group | Routes |
|---|---:|
| Anatomy production previews | 63 |
| Historical and design material | 37 |
| Knowledge and reasoning | 14 |
| Releases | 11 |
| AI and compute | 9 |
| Benchmarks | 8 |
| Worlds and science | 8 |
| Agency and cooperation | 7 |
| Developer tools | 7 |
| VibeScript | 7 |
| Ecosystem entry points | 6 |
| Networking and identifiers | 6 |
| Authoring and applications | 5 |
| Documentation and standards | 4 |
| Fields and acoustic media | 3 |
| Participation and economics | 3 |
| Utility and discovery | 3 |
| Shared templates | 1 |

Every row needs a stable identifier, canonical replacement or archive URL, owner, source revision, classification, dependency manifest and migration outcome. Inventory completeness is independent of demo qualification. Historical output and generated previews may be archived with context rather than promoted into experiments; no silent deletion or fake replacement.

`data/modules.json` contains 15 proposed module groups, all `not-yet-qualified`. Webizen and Poet have unresolved repository ownership. Lab adapters must reconcile ownership before publishing claims rather than attributing every operation to QualiaDB.

## Lab experience

Build an observatory and workshop within the natural world: daylight, material surfaces, specimens, instruments, listening spaces and views back to the landscape. Physical phenomena and human purposes provide the entry questions; the infosphere becomes visible through evidence, transformations and relationships. Avoid a wall of floating graph nodes as the entire experience.

Use one experiment registry to generate both spatial stations and a fast searchable HTML catalogue. Visitors may enter through a river observation, an artwork, a community decision or a developer test. Every station answers: what question is being examined; what the user controls; what executes; what the output means; what the method cannot establish; how to inspect or reproduce it. Keyboard, touch, reduced-motion and reading routes offer the same essential controls and results without requiring navigation through 3D.

Suggested rooms are navigation facets, not separate implementations:

1. **Matter, life and landscape:** terrain, geometry, botanical/anatomical works, fields and scientific solvers.
2. **Light, sound and expression:** EMF/QAC, image and audio production, typography, Vibe/Synth/Hypermedia recipes, Etalon rendering.
3. **Knowledge and reasoning:** graph queries, ontology alignment, epistemic/paraconsistent/temporal reasoning, SHACL and inference.
4. **Agency and cooperation:** purposes, support, permissions, obligations, provenance, correction and delegation.
5. **Connections and computation:** Q42/QBDL, streaming, storage, networking, device routing and AI workloads.
6. **Measurement and assurance:** test suites, benchmark comparisons, resource budgets and reproduction records.

Tests can all be **represented** here, but not all must run in a public browser. Classify each as browser-runnable, local-native opt-in, CI-only, hardware-dependent, destructive/private-environment excluded, or historical evidence. Show prerequisites and a reproduction command for non-browser cases. Never make a server execute arbitrary commands supplied by visitors.

## Preservation targets grounded in current source

| Existing entry point | Required preservation/improvement |
|---|---|
| `docs/benchmark.html` | Platform evidence plus live graph, geometry/renderer and Vibe Present probes; retain separate claim planes and their clocks. |
| `docs/comparative_benchmarks.html` | Dataset profiles, actual engine/format paths, topology and distribution statistics; preserve synthetic 10K, Schema.org and WordNet comparisons. |
| `docs/playground/benchmark.html` | Transparent browser harness, dataset loading, real Oxigraph WASM where available and explicitly labelled BTree proxy; proxy timings cannot be renamed competitor measurements. |
| `docs/benchmark_visualizer.html` | Import and inspect telemetry and benchmark output; preserve units and data download. |
| `docs/auto-bench.html`, `docs/wasm-bench-compare.html` | Inventory exact workloads and runtime dependencies before consolidation; preserve direct URLs until equivalent controls are qualified. |
| `docs/vibe/benchmarks.html` and `docs/js/vibescript-bench*.js` | Language workload and Present timing separation; preserve worker/runtime identity. |
| `docs/benchmarks.html` and `docs/js/wasm-llm-benchmarks.js` | Browser LLM comparison; source explicitly labels a Qualia adapter slot as a placeholder. Do not convert that into a working-capability claim. |
| `docs/tests/index.html`, `docs/tests/suite-registry.js` | Preserve logic, WASM, native and both modes and all registered suites. Generate UI counts from the registry/results rather than copying numeric marketing claims. |
| `docs/playground/index.html`, `docs/graph.html`, `docs/playground/wordnet-demo.html`, `docs/playground/compare.html` | Real query execution, input/output inspection, RDF/Q42 conversion provenance and dataset qualification. |
| `docs/playground/etalon-vibe-harness.html` | Device capability checks and evidence for actual selected execution backend. |
| `docs/hypermedia-studio.html`, `docs/playground/living-world-showcase.html` | Operation-to-recipe links, deterministic production receipts and visually inspected resources. |
| `docs/emf-qac-showcase.html`, `docs/emf-qac-architecture.html`, `docs/audio-showcase.html` | Current field/sonic contracts, physically qualified units, synthetic/measured distinction and actual playable/visible output. |
| `docs/playground/anatomy.html` and anatomy preview inventory | Preserve authored assets, clinical/educational scope, mobile fallback and existing deep links. Generated preview existence does not establish anatomical accuracy. |
| `docs/science-playground.html`, `docs/domains-solvers.html` | Audit and retain individual solver inputs, units, outputs and assumptions rather than replacing them with animations. |

The exact route mapping must be regenerated from the whole inventory, not inferred from this representative list.

## Evidence that already exists

The current `docs/comparative_benchmark_results.json` uses schema version 2 and is dated `2026-11-15T14:32:56.206719Z`. Its Qualia synthetic 10K profile identifies `daemon_http_query`, daemon 0.0.41, single-cell fixed-pool topology, a 512 MB memory ceiling, and latency distributions with 30 samples and 10 warmups. Preserve this dated artifact; do not edit historical measurements to match a new UI. Additional artifacts exist for WordNet 3.1, Schema.org and Q42 formats, plus `docs/llm_benchmark_results.json`.

`benchmarks/harness.py` already normalizes dataset profiles and distinct measurement paths including daemon HTTP, in-process Q42 SuperBlock, N-Triples parsing, WASM subprocess and Node in-process. It records dataset file metrics through shared helpers. Improvements should build on these contracts and produce a new dated run, not throw away the harness.

`docs/js/benchmark-shared.js` already groups graph primitives, Sentinel/governance, lazy/streaming, and ontology/SHACL workloads. Metric families include point/two-hop/filter/ingestion/cyclic/jitter; intercept/provenance/obligation escrow/NYM; first query/CRDT sync/WordNet streaming/P2P/compression; SHACL/defeasible reasoning. Preserve their semantics when making the presentation human-readable.

`docs/tests/suite-registry.js` is shared by browser and headless runners. It imports 24 logic registrations, 14 WASM registrations, five native registrations and a comparison registration used for `both`. These are registration counts from source, not passing-test totals. The runner deliberately treats zero-assertion tests as skipped and supports explicit skip reasons; preserve both safeguards. Existing suites cover reasoning, agency, deontics, CRDT, RDF, ontology, biology, clinical, chemistry, economics, governance, ingestion, formats, resources, solvers and native services.

`docs/js/benchmark-environment.js` records runtime/topology metadata but uses user-agent heuristics for host class and reports NPU presence as false. Improvement: label unknown/unprobed hardware explicitly; do not present guessed architecture or unprobed absence as measured hardware facts. Collect only consented information necessary to interpret an opted-in run; avoid persistent visitor fingerprinting.

## Work packages for bounded specialist agents

All packages require a clean handoff report with changed files, source revisions, runnable commands, evidence paths, known limits and remaining migration rows. Frontend agents own presentation adapters; engine changes remain scoped to their source owner and receive separate review.

### LAB-01 — Inventory and migration registrar

- Inputs: website `data/demos.json`, `data/modules.json`, all source HTML/assets, release page and archive lists.
- Owns: proposed `publication/catalog/`, `migration/routes.json`, classification and dependency scanners. Does not edit benchmark results or source engines.
- Deliver: exhaustive route/dependency inventory; stable experiment/test/benchmark IDs; canonical URL/fragment mapping; status, owner, source revision, archive decision and qualification task per row.
- Accept: every current source route has a disposition; all referenced local assets are resolved or explicitly defective; zero silent disappearance; unresolved ownership remains visible. Drift check fails on newly added unmapped source routes.

### LAB-02 — Experiment runner and legacy adapter engineer

- Depends on LAB-01 and agreed experiment schema; owns `site/lab/adapters/`, fixed bounded runner APIs and per-experiment state transitions.
- Deliver: initial adapters for graph, Vibe, Synth production, fields/audio, geometry, biology, networking and device qualification; standard load/run/cancel/reset/export lifecycle; immutable raw result access and qualified backend labels.
- Accept: same fixtures and essential controls before/after; visible failures and unsupported prerequisites; no rendering animation substituted for engine execution; one deliberately broken fixture proves failures surface. Isolate long work from UI and define admission/time/byte limits.

### LAB-03 — Benchmark methodology and evidence engineer

- Owns benchmark schemas/adapters, new reports and targeted harness improvements in a separately scoped source branch. Does not own art or general navigation.
- Deliver: frozen baseline manifest/hashes; workload definitions; correctness-before-timing checks; environment and dataset digest; cold/warm separation; samples, distributions, units, overhead scope and reproducible commands; raw JSON/CSV download; compatibility keys for comparisons.
- Accept: historical records unchanged; comparable operands required for ratios; actual measurements, reference timings, proxies, simulations and unavailable data remain distinct. Headline speedup without a matching workload/environment basis fails review. Native and browser/GPU/in-process/network paths cannot be silently pooled.
- Re-run priority: currently published graph profiles and platform workloads first, then Vibe/geometry/render/signal/AI workloads. Record regressions, successes, skips, OOM and timeouts. Avoid unbounded scope of running every hardware profile before the first website release.

### LAB-04 — Test registry and assurance curator

- Owns `publication/tests/`, suite result adapters and Lab assurance panels; existing source test owners retain their suites.
- Deliver: import existing logic/WASM/native suite catalogue plus source-repo CI suites with stable IDs; source links, prerequisites, assertion/run counts and result provenance; selected local/browser execution and recorded CI evidence.
- Accept: no skip counted as pass; not-run/unsupported/inconclusive differ from fail; no fake-green empty suite; browser success never presented as native verification. Users can move from a reassuring summary to failing assertion and reproduction command.

### LAB-05 — Infosphere Lab spatial and reading presentation

- Depends on versioned registry and runner interfaces; owns `site/environments/infosphere-lab/` and `site/lab/views/` only.
- Deliver: instrument stations, purposeful visual explanations and cinematic transitions; shared HTML catalogue/search/filter/detail/results; deep links preserving place, experiment and chosen view.
- Accept: keyboard/touch/RTL/reduced-motion/read-only routes reach same controls/results; visual effects never obscure units, failures or qualifications; direct technical path takes at most two navigation actions from Lab landing; large result tables remain usable without 3D.

### LAB-06 — Independent parity and release examiner

- Starts after LAB-01 baseline; owns qualification evidence and regression checks, not implementation it reviews.
- Deliver: per-route migration parity matrix; representative browser/mobile/backend results; fresh-link audit; visual inspection of generated outputs; benchmark evidence reproducibility sample; rollback rehearsal.
- Accept: all release-blocking routes qualified or kept available at old route; old bookmarks/anchors and input fixtures retained; no asset/WASM/worker/CORS/COEP regressions. Defects reported with reproduction steps and assigned owner, rather than diluted into a global score.

Suggested concurrency: LAB-01 first; LAB-02, LAB-03 and LAB-04 run concurrently after contracts freeze; LAB-05 builds against fixtures while adapters mature; LAB-06 checks each merge and release candidate. One integrator owns shared schema and routing files to avoid conflicting edits.

## Result semantics and minimum data contract

Separate `availability` (supported, unsupported, needs-local-host), `execution` (not-run, queued, running, passed, failed, skipped, cancelled, timed-out), `evidence_class` (live, recorded-measured, reference, proxy, synthetic, illustrative), `verification` (unreviewed, reviewed, reproduced) and `freshness`. These dimensions cannot be collapsed into a single green status.

Minimum experiment record: stable ID, localized title/question/purpose, owning module/repository/revision, capability IDs, runtime requirements, input schema/units/bounds, fixture IDs/digests, adapter/version, actual backend, output schema, expected invariants, content license, accessible representation, resource limits, source/reproduction links, route aliases and evidence records.

Minimum run record: run ID/time; exact engine and binary/WASM digest; runtime/driver/browser where applicable; dataset/fixture digest and licensing; command/configuration/seed; measurement path and clock; warmup/sample/concurrency policy; assertion/checksum results; units and distributions; failure/skip reason; artifacts/digests; producer/review provenance. Include hardware details only to the extent actually observed or explicitly declared. Expose raw evidence without secrets or personally identifying filesystem paths.

## Migration and release gates

1. **Freeze baseline:** hashes, functionality checklist, dependency map and representative outputs recorded before UI replacement. Existing result artifacts remain immutable.
2. **Schema gate:** registry/result contracts validated; compatibility migrations tested; unknown schema versions fail clearly.
3. **Parity gate per experiment:** baseline input produces equivalent validated result, or an explained improvement with source evidence. Old route stays reachable until replacement passes.
4. **Presentation gate:** human-readable explanation plus actual technical controls; no misleading buttons; units, assumptions and failure state visible; visuals inspected, not inferred from HTTP 200.
5. **Accessibility/internationalisation gate:** equivalent keyboard/reading interaction, locale-aware formatting, bilingual fallback marked, RTL and long strings tested; evidence IDs/hashes never translated.
6. **Benchmark gate:** correctness passes before timing, reproducibility metadata complete, comparison rules enforced; updated runs date-stamped and previous runs retained.
7. **Release gate:** entire inventory disposition complete, all promoted demos runtime-qualified in declared profiles, blocking regressions resolved, redirects/deep links tested, public-host headers and worker/WASM paths checked, reversible release bundle ready. Publish only under the separately authorised rollout workflow.

The plan should promise preservation of access, evidence and useful functionality—not that every legacy archive or generated anatomical preview must become a new bespoke 3D exhibit.
