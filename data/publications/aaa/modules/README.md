# The ecosystem, explained in Australian English

This is a substantive content and graph-data migration for all16 module groups in the source inventory. The explanations cover what each does, its particular design, why it matters to people, rights and agency considerations, an example and the actual scope of its native website demonstrations. They do not claim that an unmigrated module is complete.

`narratives.en-AU.json` is the structured readable source. Source references include owning repository, inspected path, revision and exact working-file SHA256. A repository-baseline link is not substituted for local source identity. Poet's shared execution host is mapped; ownership of the separate authoring application remains explicitly unresolved.

`triples.json` is the exact input to the existing native `data-pack.exe build` command. The native UnifiedVolumeBuilder writes the Q42 file; native block verification, lexicon reading and query VM validate it. Python only authors the editorial records and orchestrates those existing native functions. The generated native-validation.json records the complete decoded-row roundtrip and nine meaningful query checks, including the missing-module empty-result case. Expected rows are independently selected from the authored source triples and compared with the real decoded native result.

Run `python data/publications/aaa/modules/build_modules.py` from the staging root to refresh source identity, build a new create-only content-named Q42 if needed, validate it and consciously extend the catalogue and handoff. It asserts preservation of every existing qualified handoff file except the catalogue being extended. Repository source changes create a distinct input digest; published old Q42 bytes are not overwritten. Native library and compiler build identity remain in evidence/aaa-data/native-build.json.

The current native Data and queries instrument discovers this third dataset from catalogue.json; no new host implementation or browser reference algorithm is required. Select “The ecosystem —16 module groups explained”, then choose a query about names, human significance, rights, demo status, relationships or the Poet/QDNF boundaries. Data inspection and export remain native. A purpose-built editorial reading experience is still separate UI work.

The other two Q42 datasets, acquired terrain/astronomy evidence and all five historical benchmark files are retained. This slice does not rerun benchmarks or qualify new clinical, legal, networking or sensing outcomes.
