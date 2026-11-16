# Website publication contracts v1

P02 implementation, 16 November 2026. Default language: Australian English (`en-AU`). These are website publication envelopes and validation rules, not replacements for engine APIs, native codecs or receipts.

## Run

From the website/staging root:

```powershell
python contracts/build_schema.py
python tests/contracts/test_contracts.py
python contracts/validate.py contracts/fixtures/valid-draft.json
```

The validator returns JSON and exit code 0 for a valid publication; exit code 1 with a path-specific error otherwise. `validate(bundle)` is also importable. The committed schema is JSON Schema 2020-12. The dependency-free validator supports precisely the schema keywords used in that file, rejects unknown keywords, and adds semantic cross-reference/qualification checks. It is not advertised as a universal JSON Schema implementation.

## Records and integration

A `webizen.publication/v1` bundle contains `sources`, `runtimes`, `modules`, `exhibits`, `experiments`, `test_runs`, `benchmarks`, `locales`, `assets`, `observers`, `migrations` and an optional `graph`. Each record has a stable ID and schema discriminator. The generated `$defs` document is the exact field contract; fixtures show the full shape.

The P01 baseline ledger remains unchanged. Its 204 routes, 44 registered suites, 16 modules, runner source identities and retained benchmark JSON are inputs to a publication adapter, not automatically verified production records. Unresolved owners remain unqualified. QDNF remains a named module even if implemented inside QualiaDB. New module/exhibit/locale records fit the same schema; IDs and cross-references, not copied navigation code, connect them.

Source references contain actual repository, revision, relative path and SHA-256. Root source identities pin a publication. Evidence references contain kind, public URI, SHA-256 and observed time. Native receipts remain immutable external resources; the schema neither fabricates them nor rejects future native receipt fields. It validates the reference envelope. Runtime IDs distinguish browser, native, CI and recorded contexts.

The validator checks reference integrity and declared evidence constraints, **not whether a referenced file truly exists, a claimed hash matches its bytes, an attestation is authentic, or a scene is visually good**. Publication adapters and release verification must perform those checks against native readers/hosts and preserved artefacts. Structural validation is one gate, not proof of implementation or compliance.

## Gates enforced

- Unknown versions, properties, record IDs, duplicate IDs and dangling references fail.
- Numeric inputs require finite ordered bounds; strings require length bounds and experiments require an overall byte budget.
- Test outcomes preserve passed, failed, skipped, not-run and unsupported. A zero-assertion run cannot pass. Executed results need dated evidence; non-execution needs a reason.
- A newly measured benchmark requires correctness success, dated evidence and raw samples with matching count. Historical aggregates can retain their reported count without invented samples. Not-run records cannot carry timings. Proxy results stay explicitly classified.
- Exactly one default locale is `en-AU`. Each message retains actual language, review state and reviewer where claimed reviewed. Locale tags receive syntax validation here; canonical BCP 47 handling and script/region lookup belong to the browser/publisher locale implementation.
- Anonymous observers cannot hold grants; verified authority requires grant and host-runtime evidence references. Purpose-derived authority is explicitly false.
- Native-produced assets require recipe and runtime receipt references; visual acceptance requires visual-review evidence.
- Workstation paths, credentials and parent-traversal public references are refused.
- Release publications require a genuine q42 reader round-trip evidence envelope, qualified migrations and no test-fixture evidence. The release verifier still verifies actual bytes and execution.

## Extension/change process

Add new records under existing schemas without modifying schema IDs. Changes to meaning, required fields or allowed runtime behaviours need an explicit versioned contract review by the integrator and affected consumers. Preserve old bundles and adapters. Unknown fields fail rather than silently modifying behaviour. Put evolving native details in their original referenced receipt rather than inventing website fields.

Executable extensions remain trusted, reviewed build modules selected by runner/environment IDs. Content cannot supply arbitrary JavaScript or process commands. The shell adapter implements lifecycle, cancellation, resource limits and readable fallback; the schema describes their declared contract.

## Fixtures and test evidence

`fixtures/valid-draft.json` and `invalid-zero-assertion-pass.json` are clearly named **contract test fixtures**, not website content, scientific measurements or real execution receipts. Their zero hashes are deliberate test data. Do not publish them as results. `tests/contracts/test_contracts.py` covers 23 validation cases including false pass/timing/authority claims and preservation of historical aggregate-only results.