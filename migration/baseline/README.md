# P01 preservation baseline

Generate from the actual local source tree:

```powershell
python C:/github/synth/production/webizen-swarm/tooling/baseline/build_baseline.py
python C:/github/synth/production/webizen-swarm/tooling/baseline/build_baseline.py --check
```

`--check` fails when source coverage, captured source hashes, metadata or preserved benchmark bytes differ. Inspect intentional source updates before regenerating. Benchmark snapshots are stored under their SHA-256, so a changed source artefact creates a new snapshot without modifying the previous one.

## Files

- `routes.json`: every current HTML source, original anchors and controls, dependency-resolution candidates, source ownership, module hints, preserved route and proposed replacement/archive disposition. Proposed destinations are not implemented routes.
- `modules.json`: source-owner ledger, including explicit QDNF coverage. Webizen application implementation is mapped to QualiaDB; Poet application ownership remains unresolved instead of being falsely attributed to its MCP adapter.
- `tests.json`: real shared registry's suite imports and mode groups. These are suite registrations, not test totals or passes.
- `runners.json`: source hashes for existing benchmark/test runners. No runner was replaced in this package.
- `benchmarks.json`: original JSON identity and relative snapshot paths under `evidence/baseline/`. No benchmark was rerun or rewritten.
- `repositories.json`: repository commit identities where readable; working trees are not assumed clean. Per-file hashes identify the observed source bytes.
- `triage.json`: source-reference candidates and known review tasks. Jekyll-generated routes and runtime anchors require built-site validation; a source-resolution candidate is not automatically a public broken link.
- `../../evidence/baseline/summary.json`: coverage totals, source digest and explicit qualification scope.

The refreshed inventory contains 204 HTML sources: all prior 202 plus two retained planning/mock-up documents. The latter are historical/design material, not additional working demonstrations. Source coverage is complete; runtime qualification is assigned to later packages.

## Ownership and integration

This package owns only `migration/baseline/`, `evidence/baseline/` and `tooling/baseline/`. Final replacement routes belong to the migration package. The integrator can consume these ledgers through the P02 publication contracts without rewriting original test or benchmark evidence.
