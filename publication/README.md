# Actual Q42 website publication and observer paths

P04 implementation, 16 November 2026. This package publishes authored public website entities using the real QualiaDB writer and reader. It does not create a Python approximation of the file format.

## Commands

```powershell
python adapters/qualiadb/build_reader.py
python publication/publish.py
python tests/publication/test_publication.py
python adapters/qualiadb/graph_paths.py research
```

`build_reader.py` links the small Rust adapter against the current already-built QualiaDB rlib in Synth's `target-prod/debug/deps`. It does not rebuild or replace a live MCP/kernel binary. Compiler, source, library and adapter hashes are recorded in `adapters/qualiadb/reader-build.json`. A machine without that compatible library must build/provide the QualiaDB dependency first; it must not silently use a fake encoder. The checked-in Rust source and build script are the portable source of the Windows review executable.

## Native execution

The adapter uses `mini_parser::hash_token`, `UnifiedVolumeBuilder::with_lex_map`, `with_permissive_commons`, `Q42Volume::open`, `verify_all_blocks`, `read_all_quins` and `Q42LexMmap::lookup_owned`. The v4 lexicon can contain namespace-factored strings, so `lookup_owned` reassembles them. Each NQuin has actual parity, and the file carries native indexes, compression and DAG data. Only authored public website records enter this Commons-labelled volume; no personal data or authority grants are included.

Input JSON triples are an interchange input to the native writer. Generated `.q42` bytes come exclusively from that writer. Read-back terms are compared with every source triple. The current graph contains 123 quins, six places, sixteen module records, place/module links, typed relationships, reading/source links, and four purpose policies. Counts are generated facts, not fixed promises.

The native reader's bounded query adapter matches subject/predicate/object after verifying and decoding the real file. It is a straightforward bounded scan for this small publication, **not a claim of indexed SPARQL execution or a graph-query performance benchmark**.

## Reproducibility boundary

Canonical triples reproduce identically. Native byte-for-byte replay does not: `UnifiedVolumeBuilder` intentionally records wall-clock timestamps in its DAG commits and assertion header (`q42/q42_volume.rs`, current source around 931 and 990). Both actual outputs and hashes are retained. The receipt reports `canonical_replay_equal: true` and `strict_replay_verified: false` with the reason. No header, timestamp or DAG byte is patched to manufacture an identical hash. A future deterministic native publication profile must be a separately reviewed component integration.

## Integration API

```python
from graph_paths import GraphPaths
paths = GraphPaths(
    publication=ROOT / 'publication/generated',
    helper=ROOT / 'adapters/qualiadb/q42_web.exe')
result = paths.select('research', locale='en-AU', support_preferences=[])
```

Use importlib or add `adapters/qualiadb` to the application's module search path. `result` contains `places`, graph counts and a `selection_receipt` with actual query rows, reader identity, graph hash, policy, reasons and anonymous-public authority. Return useful errors if the reader or publication is unavailable; never fall back to an unlabelled authored full-route array.

Purpose anchors and relation preferences are facts inside `.q42`. The adapter queries them, queries typed edges, then traverses the returned graph breadth-first with the chosen relation prioritised. Remaining public places stay reachable. A changed purpose anchor in a newly written real Q42 changes the result in the test suite. The full route is not hardcoded. Support/locale choices affect representation separately and cannot grant authority.

The existing Vibe producer may package this resulting sequence and its evidence; the present traversal is the website adapter over native Q42 query results, not a claim that Vibe itself ran a SPARQL query. Authentication, private graph views, delegation and contestability remain separate host integration work.

## Public serving

Map `/publication/` to **`publication/generated/`**, not the source-code directory. It contains `current.json`, the immutable hash-named native volume, `entities.json`, schema-validated `publication.json`, `world.ttl`, `reading.html`, `q42-receipt.json` and retained replay outputs. The current pointer is digest-checked before any path query. Do not serve the native executable or workstation paths as public content.

The publisher emits canonical JSON, Turtle and reading HTML from the same entity data. The strict P02 bundle uses en-AU and carries draft editorial qualification honestly. Source `revision` fields identify an actual combined content snapshot (`content-sha256:...`), not an invented Git commit. Module profiles remain editorial work and are not falsely promoted to runtime-verified by graph inclusion.

## Verification

Ten tests use the actual native executable: exact decoded-term round trip, four distinct purpose routes, unknown purpose refusal, unsupported support/authority refusal, truncated-volume refusal, digest mismatch refusal, and an actual graph-policy edit producing a changed route. Test-generated graph variants stay under `tests/publication/runs/`; they are not public website output.
### Decoded relationship API

`GraphPaths(...).relationships(kind='all')` accepts only `all`, `knowledge`, `information` or `socioeconomic`. It verifies the current publication digest, executes native predicate queries, and returns `{kind, rows: [{subject, predicate, object}], graph_sha256, reader, verified_blocks, graph_uri}`. Terms are decoded from the real embedded Q42 lexicon, so no opaque numeric-ID-to-label guess is needed. It accepts no arbitrary predicate, source path or executable input. Additional tests compare every allowed kind against canonical source triples and reject unknown kinds or digest mismatches.