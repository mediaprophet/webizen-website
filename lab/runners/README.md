# Actual Lab runner adapters

Browser entry point: `/lab/runners/index.js`. Each run owns one dedicated module Worker; cancellation terminates it. No synthetic result values are substituted for engine output.

```js
const tests = await runTests({ mode: 'wasm', onEvent, signal });
const benchmark = await runBenchmark({ size: 10000, onProgress, signal });
const catalogue = await catalogTests({ mode: 'both' });
```

The test adapter imports the actual `/source/tests/test-runner.js`, `suite-registry.js` and `wasm-loader.js`. Native modes explicitly probe the existing local daemon only when selected; a caller may supply `nativeToken`, which is not included in output. Suite skip semantics and zero-assertion handling remain unchanged. Catalogue entries describe registered tests, not passing results. Pure JavaScript logic suites are labelled reference suites, not native execution.

The benchmark imports the original synthetic dataset loader and executes the real WASM `execute_ntriples_query`. Textual point/filter/two linked point-query workloads receive full output equality checks before timing and after each timed sample. Each operation has ten warm-ups and thirty retained samples. Dataset construction, hashing and correctness checking are outside measured intervals; JSON decoding is inside. Two linked point queries are not labelled a single SPARQL join. Suspend scene rendering during measurement, not merely camera animation.

## Source-backed loader correction

An actual correctness test found `docs/benchmark-dataset-loader.js::qHash` retained 64-bit subject/predicate hashes, while canonical Rust `qualia-core-db/src/lib.rs::q_hash` reserves the top four bits and returns a 60-bit identifier. Wildcard queries returned records but bound textual queries returned zero matches. The original failure is retained in `tests/lab/legacy-loader-correctness-failure.json`.

`canonicalSyntheticIris` repairs only the known original synthetic IRI-only input, masks all three identifier fields and recomputes parity. It refuses other lexical domains. It never rewrites typed literals or modality-tagged data. Both pre/post input hashes and the transformation are emitted. The source engine and original docs loader remain unchanged; the source owner should separately fix/qualify that loader. This corrected series must not be merged with legacy incorrectly encoded timings.

`tests/lab/hash-regression.cjs` proves the old input misses the known point result and the corrected input returns it through the same actual textual query and WASM engine. `verify-runners.cjs` exercises catalogue, original logic/WASM tests, actual benchmark correctness/samples and worker termination in Chrome. These scripts depend on the local preview at port 8800 and the locally installed Playwright runtime.

Benchmark values are browser-WASM measurements only. Unknown CPU architecture and NPU are reported as unknown/unprobed. Original historical native/competitor measurements remain in the preservation baseline; no cross-profile ranking is made. Correctness failure stops timing publication rather than producing an attractive but unsupported score.
