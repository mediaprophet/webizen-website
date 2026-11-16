# Implementation evidence — 16 November 2026

**Superseded acceptance plan:** use [the complete native overhaul plan](../aaa-overhaul/MASTER-PLAN.md). This earlier ledger remains historical evidence. The current preview failed the required visual standard; its functional results do not constitute acceptance of the overhaul.

Local review: http://127.0.0.1:8800/

The overhaul is in progress. The living-world architecture has been replaced with native QualiaPortal, Q42 and VibeScript, using generated WASM bindings only. Browser checks verify the full scene and interactions. Cinematic art direction, ecological data acquisition and the wider site migration remain unfinished.

Read [the architecture correction](NATIVE-ARCHITECTURE-CORRECTION.md) and [the Ecopunk GIS/botany source map](ECOLOGICAL-DATA-SOURCES.md). The preserved Lab instruments still use their existing browser interfaces; they are not represented as fully migrated native UI.

## Review entry points

- Field studio: `/?chapter=southern-tablelands#field-studio`
- Journey: `/journey/`
- Real tests, production and benchmarks: `/lab/`
- Ecosystem module profiles: `/atlas/`
- Accessible graph publication: `/publication/reading.html`

## Work packages

| Package | Status | Evidence and remaining work |
|---|---|---|
| P01 | implemented; acceptance review ongoing | 204 source routes inventoried, 44 registered suites and five immutable benchmark datasets retained. 114 candidate source-reference issues require runtime triage. |
| P02 | accepted | Versioned publication schemas and fail-closed validation; 23 contract tests pass. |
| P03 | in-progress | Living catchment and field studio artwork, broad regional journey palettes and native asset review. Final cinematic direction is still subject to user review. |
| P04 | implemented; qualification ongoing | Actual native Q42 writer/reader, embedded lexicon and verified public graph; four purpose-dependent routes. Semantic roundtrip verified; native header timestamps prevent byte-identical publication replay. |
| P05 | in-progress | Extensible world/exhibit registry, en-AU default and locale fallbacks. Lab and journey translation coverage remains incomplete. |
| P06 | native world implemented; visual refinement ongoing | Actual Synth production, 85 provenance-bearing native assets, Q42VOL v4 decoding and PoetSnapshot Vibe evaluation. QualiaPortal WebGPU admits 433,225 vertices and 597,854 triangles without substitution. Generated loader only; native browser routes, chapters, keyboard and zoom checked. |
| P07 | in-progress | Original logic/WASM runners adapted with real worker execution and cancellation. Native profiles remain linked to source rather than all recreated. |
| P08 | in-progress | 588-test catalogue; actual logic317pass/0fail and WASM505pass/0fail/11skip. Current browser binary reports0.0.40; native source version is distinguished. |
| P09 | in-progress | Five hash-verified historical records preserved. Real1K/10K benchmark runs with correctness, warm-up and30samples; no fabricated competitors. Full native/comparison workload migration remains. |
| P10 | in-progress | Four preserved functional Lab instruments. The Three.js artwork is replaced by a native-produced still and entry to the native field station. Full native instrument UI remains to be implemented. |
| P11 | in-progress | Q42-derived declared-purpose paths, reduced motion, large text and reading mode. Purpose changes presentation and never grants authority. |
| P12 | in-progress | 16 human-facing module profiles and source links authored; independent capability/editorial qualification remains. |
| P13 | in-progress | Native campsite assets, terrain clearing and seven broad journey chapters implemented. Additional ecosystem environments remain. |
| P14 | in-progress | All204 inventoried source routes remain reachable; full replacement UI migration is incomplete. |
| P15 | in-progress | Audience invitations and module atlas available; complete task-oriented audience journeys remain. |
| P16 | in-progress | Keyboard, mobile, reduced-motion and fallback slices checked; full accessibility and translation qualification remains. |
| P17 | in-progress | Selected real correctness/performance evidence collected; device matrix and sustained reliability gates remain. |
| P18 | in-progress | Local preview on127.0.0.1:8800. No public deployment or remote repository creation has occurred. |

## Journey handling

Only seven broad regional chapters and month bands enter the website. The original timeline remains at its user-provided location. Longer visit segments are not asserted to be individual campsites. The scenery and colour palettes are artistic interpretations, not geospatial reconstructions.

## Evidence locations

`evidence/baseline/`, `evidence/integration/`, `evidence/campsite/`, `tests/lab/runner-results.json`, `production/campsite/`, `publication/generated/q42-receipt.json`, `tests/journey/`.

Native correction: `evidence/native-world/`, `native/world-host/`, `production/native-world/integrity.json`, `production/native-world/provenance/`. Ecopunk sources are located and mapped; no external dataset has yet been acquired for this scene. Current terrain and botanical forms remain authored interpretations.

## Known technical findings

The legacy browser loader used full-width hashes for synthetic subject/predicate terms while the native query path uses canonical60-bit hashes. The benchmark adapter corrects the synthetic terms and records before/after hashes; source demos are preserved. This is not a claim that all legacy demonstrations have been repaired.

Native Q42 graph publication verifies decoded content, digest and blocks. Its current writer includes timestamps, so semantic reproducibility and byte reproducibility are reported separately. Native Synth production receipts for the authored assets verify strict replay.
