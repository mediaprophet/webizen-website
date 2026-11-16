# Architecture, editorial and inclusion work packages

Planning contribution, 16 November 2026. Paths below are relative to `C:/github/webizen-website` unless an owning repository is named. This document plans work; it does not certify the current prototype as a completed ecosystem implementation.

## Inspected baseline and concrete gaps

- `plans/renewal/README.md`, `plans/OBSERVER-PATHS.md` and `plans/HUMAN-SUPPORT-DEMONSTRATIONS.md` already establish a dedicated website repository, natural-world framing, explicit user approval before full rollout, stable entities across representations and behaviour-based human support. Retain these requirements.
- `data/modules.json` contains 15 planning entries with `publication_status: not-yet-qualified`. The renewal plan additionally identifies QDNF/identifier fabric as a distinct public element. Reconcile this discrepancy in the coverage ledger; do not remove it merely because its implementation belongs inside QualiaDB. Webizen and Poet repository ownership remains unresolved in the manifest.
- The site has six authored places, a versioned JSON manifest, simple environment/exhibit registries, Three.js presentation, native Synth resource production and a separate QualiaPortal terrain inspection page. These are useful foundations, not approval of final cinematic quality or an Etalon-rendered world.
- `tooling/world_server.py` loads `data/world.ttl` into the native graph. Its path handler queries that graph, selects a Python-authored sequence and packages JSON through Vibe. Graph-derived route selection, durable `.q42` website publication and authenticated observer composition remain website integration work. Do not describe these platform foundations as absent.
- The same server executes the real QualiaDB deontic evaluator over two synthetic norms and decides whether to return a synthetic summary. This is a bounded demonstration. It does not yet demonstrate complete host authentication, delegation, revocation, correction/contestability or positive supported participation.
- The inspected deployed `site/world/i18n.js` has six draft packs, per-message English fallback, Unicode locale parsing, RTL direction and remembered locale. Region/script lookup and Australian-English default are being revised by the primary agent. Package closure must verify final deployed behaviour rather than rely on this snapshot.
- The existing registry validates very little beyond ID/position/duplicates; `app.js` dispatches experiment types with conditionals. It is not yet a complete extension contract with schema migration, capability requirements, lifecycle, fallback and isolated failure.
- The public technical inventory must remain independently usable: spatial presentation must not make navigation depend on GPU availability, a scenic journey or spoken English.

## Governing data contracts

The integration owner must freeze version 1 of these contracts before dependent agents edit consumers. They are website contracts and adapters to actual platform types, not invented kernel APIs.

1. **Publication bundle:** schema/version, source repositories and commit IDs, build runtime IDs, generated-at timestamp, content/evidence hashes, vocabulary revisions, `.q42` location and digest, semantic HTML/index projections, media and licence manifest, qualification state and superseded revision.
2. **Entity:** stable ID; type; human title; descriptions by locale; owning module(s); activities and purposes; typed information/knowledge/socioeconomic relationships; source/evidence references; claims and limits; available representations/actions; support alternatives; review owner/status. Do not encode all relations as generic `relatedTo`.
3. **Exhibit:** entity ID; environment/scene asset ID; camera entry; input schema and units; actual operation/runner ID; runtime/device requirements; budget/cancellation semantics; output/evidence schema; accessible counterpart; provenance; fallback and failure state. No arbitrary code in content manifests.
4. **Observer request:** declared purpose, requested depth, locale, voluntary support preferences, actual authenticated relationship/grants where relevant, circumstance and runtime constraints. Separate presentation preferences from authority. Anonymous public is explicit; never inherit the Rust enum's `Principal` default into browser authority.
5. **Selection receipt:** request/context version, graph hash, policy/rule revision, selected stable entity/action IDs, reasons, alternatives, qualified host and limits. A purpose change may change invitations and sequence; it must not alter measured facts or benchmark results.
6. **Translation record:** canonical locale, source key/revision, translated value, actual language/script, draft/reviewed/stale status, reviewer and glossary revision. A fallback message keeps its true language tag. Distinguish content translation from format locale.

## Bounded packages suitable for parallel agents

### ARCH-01 — Publication contracts and ownership ledger

**Owner:** schema/integration agent. **Exclusive writes:** `schemas/`, `data/module-owners.json`, `contracts/`, `tooling/validate_publication.*`. Component repositories are read-only. **Depends:** source/demo baseline inventory.

Deliver schema fixtures for the six contracts, source-owner qualification ledger covering every module and subsystem, ID rules, schema migration/deprecation policy and deterministic validation command. Resolve Webizen/Poet ownership as an evidence task; retain unresolved entries honestly until qualified. Record QDNF/identifier fabric explicitly.

**Acceptance:** a new module/exhibit can be represented without changing the schema; invalid IDs, missing evidence references, duplicate stable IDs, conflicting source revisions and unknown schema versions fail with actionable errors. Every existing module entry is mapped, retained or explicitly split; none disappears silently. Integrator owns all cross-package schema changes.

### ARCH-02 — Genuine `.q42` publication and reproducible projections

**Owner:** graph/publication agent. **Exclusive writes:** `publication/`, `tooling/publish_graph/`, `adapters/qualiadb/`, `tests/publication/`. **Depends:** ARCH-01; pinned source facts supplied by content/lab owners.

Use the existing QualiaDB writer/reader and actual admitted Vibe/host operations. Read source/indexes first; do not invent a new `.q42` encoding or new `hypermedia.*` operation. Build a public bundle plus equivalent semantic HTML and compact indexes. Keep private data outside public artifacts. Provide a small graph revision that contains a real correction and retained provenance.

**Acceptance:** the produced `.q42` is opened by the real reader and queried; entity counts and selected answers agree with projected HTML/indexes; repeated builds are reproducible under a stated profile or explicitly document nondeterministic inputs; source/build hashes accompany artifacts. Invalid/truncated bundles fail clearly. Update and rollback preserve stable links. Every public asset and graph record is assessed as public before packaging.

### ARCH-03 — Observer-relative paths and host adapter

**Owner:** observer/interaction agent. **Exclusive writes:** `behaviour/paths/`, `adapters/observer/`, `tests/observer/`. **Depends:** ARCH-01/02 and shell extension interface. Coordinate with support agent through fixtures, not shared source edits.

Map existing entity-view observer, circumstance, representation-wing, attribution, rights-filter and projection concepts into the website host. Replace the current separately hardcoded route arrays with qualified graph queries and actual supported Vibe composition. Preserve direct navigation, orientation and a readable route explanation. Never infer sensitive personal status from navigation behaviour.

**Acceptance:** at least three explicit synthetic contexts select meaningfully different paths through the same graph; a declared purpose alone cannot elevate authority; graph revision and actual rule/host are visible in evidence. An immersive-to-reading switch keeps entity ID, current question and result. Unknown/anonymous status, expired grants where supported, cancellation, stale graph projections and changed purpose produce tested, comprehensible results. Unsupported host semantics remain explicit limits.

### ARCH-04 — Rights, support and contestability scenarios

**Owner:** human-support scenario agent. **Exclusive writes:** `scenarios/human-support/`, `content/support/`, `tests/human-support/`. **Depends:** ARCH-01; can author synthetic fixtures while ARCH-02/03 proceed; enforcement acceptance waits for host adapter.

Deliver three complete activities: (a) positive accessible participation in an environmental inquiry; (b) bounded community-purpose sharing/delegated assistance with permitted and refused outcomes plus a useful alternative; (c) correction/contestability that changes a dependent query without erasing original provenance. Extend later to cooperative work/resource needs using qualified existing operations. Each scenario connects a sourced principle, attributed interpretation, requirement, mechanism, observed behaviour, limits and recourse.

**Acceptance:** an actual input changes an actual supported outcome, not merely a badge; the protected output is absent from refused network responses, not hidden by CSS. Support routes complete equivalent tasks and retain source/result identity. A synthetic correction produces an inspectable before/after query result and preserves contested assertions. No claims of universal rights/legal compliance or institutional endorsement from a passing test. Authenticated-host claims require actual host evidence.

### ARCH-05 — Extension system and representation shell

**Owner:** application-platform agent. **Exclusive writes:** `site/runtime/`, `site/components/`, `site/extensions/`, `tests/extensions/`. **Depends:** ARCH-01. Receives scene modules, lab runner adapters and content through interfaces.

Turn registries into versioned extension contracts for environments, exhibit renderers, technical instruments and content representations. Give each mount/unmount, resize, pause/resume, locale/update, dispose, capability report and accessible fallback hooks. Keep trusted executable modules in the reviewed build; data bundles must not inject arbitrary JavaScript. Add resource budgets, cancellation and isolated failure. The current host's fixed allowlisted recipes are the starting safety boundary, not a public unrestricted MCP endpoint.

**Acceptance:** a seventh exhibit and a new environment are added through documented modules/manifests without editing central experiment conditionals; failure in one exhibit leaves map, reading route and other exhibits usable; unloading releases its graphics/audio workers/resources. Deep links and keyboard routes work before any cinematic transition. Source identifiers and technical receipts remain inspectable without leaking local paths or private details in the public build.

### ARCH-06 — Internationalisation and Australian-English editorial foundation

**Owner:** internationalisation agent. **Exclusive writes:** `site/i18n/`, `site/locales/`, `content/locales/`, `tooling/localisation/`, `tests/i18n/`. **Depends:** ARCH-01 and shared shell; content agent supplies source keys and meaning.

The canonical default is **`en-AU`** for first visit, authored content, spelling and default formatting. A visitor's remembered explicit choice overrides it; browser language must not silently replace this default. Regional/script packs use a documented chain such as `zh-Hant-TW → zh-Hant → zh → en-AU`; optional generic `en` is an alias/fallback, not an ambiguous editorial authority. Preserve actual message language and format locale separately. Handle plurals/select messages through a tested message system, Unicode/bidirectional isolation, language-sensitive search, captions, transcripts, locale URLs, metadata and alternate language links.

**Acceptance:** first visit in a browser advertising `en-US` or another language still begins `en-AU`; explicit `ar`, `zh-Hant`, `sr-Latn` and other valid choices persist. Script/region fallback preserves meaning and script where packs exist. Unsupported language has clearly identified fallback, not a claim of full translation. RTL and mixed IDs/numbers/units render correctly; long text, CJK, combining marks and non-Latin search work. Translation coverage/staleness is machine-readable. Native review is required before a pack is labelled reviewed; do not claim all languages translated because the architecture accepts BCP47.

### ARCH-07 — Accessibility and equivalent task routes

**Owner:** accessibility agent. **Exclusive writes:** `tests/accessibility/`, `content/accessibility/`, `site/accessibility/`; shell fixes submitted to shell owner or allocated small non-overlapping patches. **Depends:** scenario fixtures and shell, but begins requirements in first wave.

Specify and verify keyboard/touch/screen-reader, reduced-motion, low-bandwidth, no-WebGL, caption/transcript and readable non-spatial paths. Require visible focus, status announcements, dialog focus management, adequate contrast and meaningful alt descriptions; no audio autoplay. Need/support choices are optional and adjustable. Do not collect diagnosis or personal need information to enter the experience.

**Acceptance:** reviewers complete the same inquiry, result inspection and correction task using keyboard and reading routes; focus survives scene changes and errors; essential content remains available when GPU or JS fails as appropriate to the content. Automated checks supplement manual task evidence. A conformance claim needs its scoped audit, not only a score. Agree a current standards target during implementation using authoritative specifications.

### ARCH-08 — Human editorial system and core-module atlas

**Owner:** editorial lead with bounded module researchers. **Exclusive writes:** `content/en-AU/`, `content/modules/`, `content/audiences/`, `content/glossary/`; source bundle records go through ARCH-01/02 interfaces. **Depends:** ownership ledger; writes can proceed alongside runtime work.

Each module explains a real human activity, what it does, what distinguishes its approach, why it matters, its ecosystem relationships, what can be tried today, and limits. Include QualiaDB, Webizen, Poet, VibeScript, Synth, Hypermedia, Etalon, Anvil, Aether, Chora, Webizen Host, QDNF/identifier fabric, semantic instruments, cooperation/Wellfare/work graph, Anatomy and Ecopunk. Resolve names with sources; do not infer product readiness from repository existence.

Deliver connected audience routes for everyday visitors; government/public institutions; UN/civil society; developers; researchers/technical AI users and agents; investors/partners. They share canonical claims/evidence and differ in depth/examples. Machine-readable technical projections must not dictate human homepage prose. Investor copy uses actual qualification, dependencies and opportunities; no invented traction, returns or adoption.

**Acceptance:** every inventory module and principal subsystem has a qualified page and a path into a working or honestly labelled demonstration; no page consists of generated feature lists. Representative readers can identify purpose, difference, practical consequence and a next action. Human editorial review rejects bot-to-bot progress language and unsupported comparisons. The same claim/evidence IDs appear across audience variants and locales.

## Swarm dependencies and integration rules

- Wave 0: baseline audit plus ARCH-01; content research and inclusion requirements may proceed read-only/in isolated notes.
- Wave 1: ARCH-02, ARCH-05, ARCH-06 and ARCH-08 after contract freeze. With four live slots, retain one integrator and run at most three producers.
- Wave 2: ARCH-03 and ARCH-04 integrate real graph/host behaviour; ARCH-07 tests the resulting slice. Visual/lab agents consume the same interfaces.
- Every package receives a base commit, allowed write paths, input fixture versions, exact deliverables, acceptance command/manual scenario and known limits. No two agents own the same file. A shared contract change is proposed to the integrator and consumers before merging.
- Completion packet: commit/diff, artifact hashes, test/output evidence, visual/manual review where needed, claim changes, unresolved limits, migration effects and next dependency. Reviewer must inspect the evidence, not accept the agent's summary alone.
- Integrator merges in dependency order, reruns the combined human task and lab regression gates, and records a rollback point before replacing public routes. Full-site deployment remains a separate concrete approval step if not otherwise authorised.

## Highest-priority acceptance slice

One river inquiry spans the natural world, an attributed observation, the Commons and Infosphere Lab. The visitor selects a declared purpose; the same graph entities yield a suitable route. They change a support preference, run the same investigation through reading or spatial interfaces, inspect actual computation and evidence, attempt permitted/refused synthetic access, correct a synthetic assertion and see the dependent result change. `en-AU` is default, with a reviewed RTL route and explicit fallback. This proves the composition before scene count or marketing breadth is expanded.
