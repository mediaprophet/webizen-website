# Webizen website: implementation and swarm delivery plan

16 November 2026 · default locale **en-AU, Australian English** · implementation planning baseline, not a claim of production completion.

The deliverable is a complete public home for a human-centric computing **ecosystem**: an original, explorable natural world; excellent readable pages; practical tools and explanations; and an inspectable **Infosphere Lab**. Its subject is human life within the natural world and infosphere. The technology is demonstrated through useful experiences, with evidence available at the point of a claim.

This plan supplements the existing renewal, world-design, human-support and observer-path plans. It controls implementation sequencing and delegation. The current forest-valley prototype is a review specimen, not the accepted quality ceiling. This planning request does not publish the site or waive the previously requested visual-review gate.

## 1. What exists, what must change

The dedicated local repository is `C:/github/webizen-website`; implementation repositories retain their own engines, tests, normative specifications and benchmarks. The website branch is `codex/website-foundation`; no remote or production migration is implied. Use a pinned cross-repository release manifest, not siblings' moving working trees, for production builds.

| Baseline | Verified scope | Implementation consequence |
|---|---|---|
| Revised world at port 8800 | Continuous 3D valley, six places, relationship layers, reading/support controls | Keep useful interaction learning; replace or develop assets against an explicit art-direction gate |
| Main world renderer | Three.js presentation | Do not describe the whole world as Etalon-rendered; establish and prove the production rendering contract |
| Native production | Current private Synth MCP reports 150 operations; terrain, EMF and WAV recipes execute with replay receipts | Keep native assets, actual receipts and source recipes; upgrade the visual experience around them |
| Native terrain | Synth imports and seals the landscape as `.10d`; QualiaPortal renders it | Retain geometry inspection; its receipt also reports no governance attestation, which is not an authority grant |
| Graph/path prototype | Authored TTL loaded into QualiaDB; public graph queried; authored route packaged by Vibe | Durable `.q42` publication and graph-derived observer selection remain implementation work |
| Rights prototype | Two synthetic norms evaluated by QualiaDB, server applies allow/refuse result | Add positive support, delegation, correction and contestability; this example is not general rights compliance |
| Existing site inventory | 202 HTML source entries, including archives/layouts/generated previews | Account for every entry; do not call them 202 working demos |
| Language prototype | Six draft language packs, RTL, explicit fallback and locale formatting | en-AU default; complete message extraction and review before claiming translated coverage |

Baseline counts and commits must be regenerated at implementation kickoff. Source-derived counts are facts about a revision, not evergreen marketing constants. A stale connected MCP catalog is not evidence that a source morphism is absent.

## 2. Visitor experience and information architecture

The world has coherent spatial relationships, scale, lighting, environmental sound, authored viewpoints and meaningful actions. It must feel like a place worth exploring. Navigation is also available through ordinary links, a map, search and readable pages. No task requires gaming skill, sound, animation, a GPU or a lengthy introduction.

| Place | Human question and subject | Actual experience | Ecosystem connection |
|---|---|---|---|
| Living catchment | How do water, landscape and shared decisions interact? | Shape terrain, inspect assumptions, compare scenarios | Geodesy, scientific computation, provenance and stewardship |
| Forest and biosphere | How do living structures grow and relate? | Inspect authored botanical structures at several scales | Geometry, botanical production, knowledge representation |
| Living body | How do we understand and support living beings? | Spatial anatomy, sourced explanations, calibrated domain examples | Anatomy, QualiaDB, Etalon, appropriate epistemic limits |
| Commons | How can people participate with agency and support? | Purpose-aware sharing, assistance, correction, delegation and refusal | Web Civics, host authority, cooperation, observer/circumstance |
| Atelier and cultural spaces | How do people make, interpret and share work? | Produce and inspect an image, sound or spatial work | Synth, Hypermedia, Poet, VibeScript, liberal arts |
| Field station / Infosphere Lab | How do we know what this technology can do? | Run experiments, compare measured evidence, inspect tests | All modules, runtime profiles, workloads and provenance |

The Lab may be reached from every place and directly at a stable URL. A demonstration explains **what changes, why it matters, what ran and what its limits are**. Technical details unfold progressively; the first screen is not an audit log.

Every core module gets a complete profile: purpose in everyday language, distinctive approach with evidence, human significance, relationships with other modules, availability by runtime, working examples, sources and limitations. Cover QualiaDB, Webizen applications/runtime, Poet, VibeScript, Synth, Hypermedia production, Etalon, Anvil, Aether, Chora, Webizen Host, Web Civics/vocabularies, cooperation/Wellfare, Anatomy and Ecopunk. Reconcile uncertain repository ownership rather than inventing repository names. Newly discovered modules enter the same contract.

Audience routes serve ordinary visitors, developers, researchers, government/public institutions, UN/civil-society participants and investors. These are selectable reading intentions, not inferred personal classifications or authority levels. The investor route separates evidence, readiness, dependencies and opportunities; institutional routes demonstrate public value, accessibility and accountability without asserting certification.

## 3. Infosphere Lab: tests, demonstrations and benchmarks

The Lab is a spatially navigable instrument space, with an equivalent searchable catalogue. Instruments sit in recognisable bays: knowledge/graphs; geometry/matter; light/signals; sound/time; language/meaning; cooperation/authority; devices/runtime; reliability/performance. Relationships connect experiments to their modules, source, prerequisites and human use.

Three distinct things can be discovered together but must not be conflated:

1. **Demonstration:** an interactive explanation or capability exercise. Show artistic illustration, synthetic model, recorded result and live execution distinctly.
2. **Test:** a named assertion with revision, inputs, environment, expected outcome and result. A missing run is “not run”, not a pass. Represent unit, contract, integration, browser, accessibility, visual, lifecycle and end-to-end suites; allow suite drill-down without flooding the world with thousands of icons.
3. **Benchmark:** a defined measurement of a named workload and configuration. Raw records, methodology, errors and uncertainty remain inspectable. A screenshot of a chart is not benchmark evidence.

An instrument provides an approachable introduction, preset and advanced inputs, execution requirements, run/cancel/reset, progress, interpretable results, accessible output, evidence drawer and related work. Heavy native/GPU jobs require an explicit run; historical results remain browsable without installing a runtime. Do not expose arbitrary source execution through a public web endpoint.

Represent all discoverable test suites through generated manifests from owning repositories. Expose safe public test metadata and sanitised results, not secrets, private fixtures or machine paths. Browser-safe tests can run locally; privileged/native suites run in controlled workers or show reproducible recorded runs. Clearly label the execution location and time. The Lab does not reimplement test runners in scenery.

### Preservation and improvement contract

- Create a route-by-route migration ledger with original URL/fragment, source hash, owning repo, existing workload, replacement ID, destination, migration disposition, evidence and reviewer. Retain original implementations until equivalence is demonstrated.
- Separate the runner/codec/workload from presentation. Initially host proven runners behind adapters or isolated same-origin routes; refactor only when behavioural comparisons exist.
- Capture baseline inputs, outputs, binary/WASM identity, warnings and supported failure paths. New UI must preserve parameters, output meaning, downloads and inspection controls, not just a link to the old page.
- Every demo is either preserved, demonstrably improved, retained as an accurately labelled archive, or explicitly unavailable with reason and source. A replacement cannot silently discard scientific assumptions or narrow supported input ranges.
- Tests and benchmarks must be reachable without entering the 3D world. Preserve external deep links and important fragments. No generic redirect of every former destination to the homepage.
- Re-run representative old/new workloads against the same artefact and inputs. Compare exact outputs where deterministic; use justified tolerances otherwise. Visual similarity is insufficient.
- Existing competitors' historical reference measurements remain labelled as historical. Separate native/browser, cold/warm, CPU/GPU, device profiles and build settings. No rankings across incompatible conditions.
- Report repeated samples, aggregation, variation, warm-up, synchronisation, dataset size/shape, run failures, transfer/compile cost and memory measurement method. Keep failed samples and exclusions with reasons.
- Refresh measurements only with actual executions. Preserve old records immutably; record “not rerun” when hardware, provider or source is unavailable. Present limitations beside results.
- Benchmark the renderer separately from scientific workloads. Disable or isolate scene animation while timing; capture background/device conditions. Lab scenery must not contaminate the claimed engine comparison.

## 4. Publication architecture and extension contracts

Component repositories publish immutable bundles identified by commit, content hashes and schema version. Website ingestion validates and assembles them into the content graph and `.q42` publication, then emits readable HTML, structured downloads, navigation and world bindings. Vibe recipes select/compose representations and produce packaged media with recorded native receipts. Verify `.q42` round trips using the actual QualiaDB codec and reader; JSON renamed to `.q42` is not acceptable.

The application boundary has replaceable renderer and experiment adapters. Main world, native viewers, reading pages and Lab instruments share stable entity IDs. A renderer failure does not remove the underlying content or navigation. Capability detection offers an accurate choice rather than silently replacing a native computation with a look-alike animation.

Proposed versioned contracts (the schema agent owns definitions; other agents consume them):

| Record | Required substance |
|---|---|
| ModulePublication | module/repository/revision, ownership, human explanation IDs, capabilities, supported runtimes, evidence and related modules |
| Place / Exhibit | stable ID, content keys, environment/pose, module references, experiment ID, readable route, asset manifest and capability requirements |
| Experiment | ID/version, runner contract, bounded input schema, outputs/units, reset/cancel, epistemic class, source and accessibility equivalent |
| TestSuite / TestRun | suite/test IDs, assertions, source commit, runner/environment, fixtures or safe digests, outcomes, timings, logs and exclusions |
| BenchmarkWorkload / Run | workload/version, dataset/hash, semantics, device/build/runtime, samples, method, units, uncertainty, failures and reproducibility artefacts |
| Claim / Evidence | precise scoped statement, revision, supporting/refuting evidence, freshness and limitations; no unattended blanket claims |
| ObserverContext / PathDecision | declared purpose and support preferences, separately supplied verified authority, graph/policy versions, chosen representations, reasons and reset |
| LocalePublication | BCP 47 ID, source revision, message/translation state, reviewer, fallback ancestry, fonts, direction and coverage |
| Asset / Scene | recipe/source dependencies, producer binary/catalog, content hash, licence/attribution, preview, quality tier and replay receipt |
| MigrationEntry | legacy route/fragment, current owner, destination/disposition, runner equivalence evidence and redirect test |

Extensions register through explicit typed contracts. New modules, places, experiment families, language packs and renderer providers must be addable without editing the central navigation or copying a page template. Version skew, invalid links, duplicate IDs and undeclared capabilities fail validation. Use trusted build-time plugins; do not fetch executable plugin code from arbitrary content records.

## 5. Australian English, internationalisation and inclusion

**Default:** Australian English (`en-AU`) for initial display, editorial source, formatting and metadata. A saved explicit visitor selection overrides it. Do not silently choose a different language from browser geolocation or navigator language. Existing generic `en` URLs may be compatibility aliases; source ownership and fallback must remain clear.

Support Unicode and valid BCP 47 language, script and region identifiers throughout. Resolve exact locale → language/script → base language where appropriate → en-AU. Script differences such as Serbian Latin/Cyrillic or simplified/traditional Chinese need distinct packs; do not assume every regional variant can reuse all base-language content.

Extract messages, labels, validation errors, tooltips, units, transcripts, figure descriptions, metadata and experiment explanations. Keep stable identifiers and scientific symbols separate from translated prose. Use plural/select message rules and locale-aware number/date/time/unit formatting. Store canonical values with explicit units and time zones.

Use logical CSS, BiDi isolation for mixed technical text, script-capable fonts and shaping, flexible layouts, mirrored navigation where meaningful, and untranslated fallback tagged with its actual language. Do not mirror scientific axes, source code or media indiscriminately. Localised URLs, `hreflang`, canonical links, indexing, search and language switching retain entity identity and current context.

“All languages” is an extensible architecture and translation programme, not a claim that all content is already translated. Display per-locale and per-section coverage and review state. Draft machine translations need qualified review for public promotion, particularly rights, scientific and institutional content. Target language cohorts and reviewer capacity are publication decisions; en-AU remains complete and maintained throughout.

Accessibility is part of every package: semantic HTML, keyboard operation, visible focus, screen-reader equivalents, text enlargement, contrast, reduced motion, no forced audio, captions/transcripts and meaningful descriptions. World-only interactions need equally capable conventional controls. Low-bandwidth and no-GPU users retain the content, evidence and useful workflows. Human testing complements automated checks.

## 6. Observer paths and demonstrated human support

Adapt ordering, level of explanation, scale, media and useful next actions from declared purpose and support requirements, with a visible “why this path” explanation and reset. Do not infer sensitive status, obscure evidence, manipulate conclusions or treat a reading preference as permission. Authority is evaluated separately by the applicable host/instrument contract.

Required scenarios: a person receiving accessible assistance; a community researcher receiving permitted aggregate information while individual records remain withheld; bounded delegation with revocation; correction of a source record and consequent representation updates; contesting a refusal; and a return to an unpersonalised public view. Use synthetic data in public demonstrations. Record graph/policy/observer versions and explain exactly which parts are executable versus illustrative.

## 7. Swarm operating model

One **integrator** owns shared contracts, dependency resolution, release decisions and the integration branch. Specialist sub-agents own bounded work packages, not the entire site. A **verification agent** reviews each package independently of its author. The **art director** enforces visual coherence across environments and audience views. Source-maintainer liaison happens through recorded evidence and explicit user-authorised communication; agents do not send unsolicited external messages.

With the present four-slot limit, run the integrator plus at most **three active specialists**. Roles are logical and can be reused across waves. Never start a downstream task before its input contract is accepted. Long-running benchmark/GPU jobs are serialised per device; that resource limit is independent of agent count.

Each package uses an isolated `codex/<package-id>` branch/worktree once authorised writable repository locations are established. If sharing a tree, enforce a file-ownership ledger and one writer per path. No concurrent edits to manifests, lockfiles, shared CSS, central routing or CI. Specialists propose contract changes; the integrator lands them and tells consumers the new version. Component changes go to the owning repo in a separate reviewable change, with compatibility evidence.

Package states: `queued → ready → active → review → accepted`; exceptions are `needs-input` and `failed-validation`, with a concrete reason. An agent stopping is not acceptance. A completion report includes changed paths, input/output versions, commands and results, evidence locations, unresolved risks and next dependencies. Integration reruns boundary checks; it does not merely trust a screenshot or a sub-agent's prose.

The accompanying [work-packages.json](work-packages.json) is the authoritative dispatch ledger. Every row specifies role, owned paths, dependency IDs, inputs, deliverables, acceptance, exclusions and review. Proposed paths are future interfaces, not claims that directories already exist. The specialist [Lab audit](lab-audit.md) and [architecture audit](architecture-audit.md) provide source detail and scenario designs; their LAB/ARCH package sketches are inputs to P01–P18, not a second competing dispatch schedule or ownership ledger.

The source audit identified eight benchmark routes, 63 anatomy preview routes and 37 historical/design routes within the 202-entry baseline. It also identified an existing shared test registry and measured comparative results dated 15 November. Preserve its zero-assertion skip semantics, actual engine/proxy distinctions and immutable results. P09 must correct hardware metadata that currently treats unprobed NPU presence as false. P01/P12 must reconcile QDNF/identifier fabric with the module manifest.

### Dispatch instructions for any package

1. Read this plan, the package row, applicable repository instructions and referenced contracts. Verify dependency acceptance and the recorded source revisions.
2. State the owned paths and immediate deliverable. Inspect existing source/function registries before proposing new platform functionality. Preserve unrelated work.
3. Implement only the package scope. Request a contract change from the integrator when necessary; do not patch another agent's files opportunistically.
4. Run meaningful package checks and save machine-readable evidence. Inspect visual assets at full-size and in the actual browser context. Record actual runtime/binary identity.
5. Submit a reviewable diff and completion report. Never change a failed result into a pass, broaden claims beyond evidence, or publish/merge another package unilaterally.

## 8. Delivery waves and critical path

| Wave | Packages | Exit gate |
|---|---|---|
| 0: protect and measure | P01 baseline; P02 contracts; P03 visual direction | Inventories and preservation ledger captured; contract v1 accepted; art direction reviewable |
| 1: common foundation | P04 publication graph; P05 locale/content shell; P06 production/render proof | One entity round-trips through graph/HTML/world; en-AU/RTL works; real native asset renders |
| 2: Lab vertical slice | P07 experiment adapters; P08 test registry; P09 benchmark evidence | Existing demo, test and benchmark each survive presentation replacement with equivalence evidence |
| 3: integrated review | P10 Lab world; P11 observer/support; P12 content/module coverage | One complete journey combines scene, readable content, real experiment, support and evidence |
| 4: site-wide migration | P13 world expansion; P14 all routes; P15 developer/audience routes | Every baseline entry assigned and migrated/archived with tested links; all module profiles present |
| 5: qualification | P16 accessibility/i18n; P17 performance/reliability; P18 release | Acceptance matrix passes; deployment preview, migration and rollback are reviewable |

This is a dependency graph, not a promise that each row occupies equal time. P03 can proceed alongside inventories; content drafting can proceed after P02 while graph work continues. P10 depends on P03/P05/P06/P07/P08/P09. P14 must not remove old runners before P07/P09 acceptance. Estimate durations after P01 establishes actual workload and hardware capacity; do not invent completion dates.

## 9. Acceptance and release gates

**G0 — preservation:** inventory coverage is 100% of the refreshed baseline; all important workloads, downloads and routes have owners and baseline evidence. Missing/runnable/archival states are explicit.

**G1 — creative direction:** user can review original natural-world art direction, camera/title treatments and a functioning representative scene. Compare against the strongest old site demonstrations. A generic low-poly world or decorative connected nodes alone does not meet the brief. Source recipes, licences, native previews and performance costs accompany the scene.

**G2 — platform proof:** a single journey runs native production, displays real outputs/receipts, and survives unsupported-device fallback. Confirm producer and renderer identity separately. No fake live statuses or generated benchmark scores.

**G3 — Lab equivalence:** old/new runner comparisons pass; at least one example from each demo/test/benchmark class works through both spatial and conventional UI. Test-suite manifests cover owning repositories; remaining execution coverage is visible.

**G4 — human support and languages:** en-AU default and persistence verified; representative RTL, CJK, Indic, long text and mixed-script tests pass; keyboard/no-GPU/reduced-motion routes preserve task completion. Purpose does not grant access. Correction/contestability/positive support scenarios execute.

**G5 — complete overhaul:** all module profiles and audience paths reviewed; no legacy page is silently lost; original scientific evidence preserved; errors, downloads, search, mobile layout and metadata work. New benchmarks are honestly measured and old runs retained. Every promoted runnable workload has parity or qualified-improvement evidence for each advertised profile. Other legacy workloads remain available until their replacements qualify, or receive an explicitly reviewed unavailable/archive disposition. G3 is only a vertical-slice gate, never a substitute for exhaustive accounting.

**G6 — release:** pinned reproducible build, link/asset/header checks, compatible WASM profiles, sanitised public evidence, preview on deployment-equivalent infrastructure, route redirects, rollback and monitoring. Confirm both existing publication paths and DNS/hosting settings before cut-over. User review of the concrete release precedes public deployment where required by the existing brief.

P03 identifies target devices; P06 establishes provisional measured transfer/frame/memory budgets before environment expansion; P17 verifies final budgets. Initial design targets are a usable HTML route before optional heavy assets, lazy scene loading, responsive interaction during jobs, a capped mobile quality tier and explicit long-run cancellation. Adopt numeric pass/fail budgets from measurements; do not use desktop-only FPS to claim universal support. Benchmark-history retention and semantic correctness are hard gates even if visual work is approved.

P14 may prepare the migration ledger early, but final acceptance waits for P10/P13/P15 route manifests. P10 consumes P04's actual publication graph. P16 owns translation authoring as well as qualification, with independent language reviewers; drafts can begin against versioned messages before its final dependency gate. P09 requires correctness assertions before timed samples. Baseline migration records and final route mappings have separate owned paths to avoid competing writers.

## 10. Risks and decisions to keep visible

- **Visual ambition versus platform proof:** prototype presentation and native assets currently have different rendering paths. P06 must resolve the production composition strategy with evidence, not branding.
- **Graph semantics versus personalised decoration:** authored sequences are currently not graph-derived observer decisions. P04/P11 close that gap before claiming it.
- **Registry scale:** thousands of tests need generated grouping/search and useful suite summaries, not thousands of spatial props.
- **Translation quality:** all-language architecture cannot replace human editorial review. Scientific meaning and rights terminology require particular care.
- **Benchmark contamination:** separate display cost, engine workload, transfer, compilation and GPU synchronisation; publish methodology changes as new series.
- **Multi-repository drift:** pin source bundles, record dependencies and fail inconsistent publication. Never update a module claim from a README alone.
- **Current demo host:** localhost Python/MCP is a review tool, not the public hosting architecture. Public experiments need bounded adapters and explicitly scoped execution.
- **Approval scope:** this plan prepares swarm implementation. It does not reinterpret “ok” as acceptance of every scene, completion of tests or permission to publish.

Implementation completion means the complete experience, preserved/improved technical functions and their evidence pass these gates together. Finishing only the scenery or only the data pipeline is insufficient.
