"""Record evidence without treating partial packages as accepted."""
from pathlib import Path
import json
root=Path(__file__).resolve().parents[1];repo=Path('C:/github/webizen-website')
out=root/'plans/swarm';out.mkdir(parents=True,exist_ok=True)
p=json.loads((repo/'plans/swarm/work-packages.json').read_text(encoding='utf-8'))
notes={
 'P01':('implemented; acceptance review ongoing','204 source routes inventoried, 44 registered suites and five immutable benchmark datasets retained. 114 candidate source-reference issues require runtime triage.'),
 'P02':('accepted','Versioned publication schemas and fail-closed validation; 23 contract tests pass.'),
 'P03':('in-progress','Living catchment and field studio artwork, broad regional journey palettes and native asset review. Final cinematic direction is still subject to user review.'),
 'P04':('implemented; qualification ongoing','Actual native Q42 writer/reader, embedded lexicon and verified public graph; four purpose-dependent routes. Semantic roundtrip verified; native header timestamps prevent byte-identical publication replay.'),
 'P05':('in-progress','Extensible world/exhibit registry, en-AU default and locale fallbacks. Lab and journey translation coverage remains incomplete.'),
 'P06':('in-progress','Current Synth catalog150 checked through private native MCP. Native scene assets and deterministic production receipts available; main review scene uses Three.js, not an Etalon-only renderer.'),
 'P07':('in-progress','Original logic/WASM runners adapted with real worker execution and cancellation. Native profiles remain linked to source rather than all recreated.'),
 'P08':('in-progress','588-test catalogue; actual logic317pass/0fail and WASM505pass/0fail/11skip. Current browser binary reports0.0.40; native source version is distinguished.'),
 'P09':('in-progress','Five hash-verified historical records preserved. Real1K/10K benchmark runs with correctness, warm-up and30samples; no fabricated competitors. Full native/comparison workload migration remains.'),
 'P10':('in-progress','Four functional Lab instruments, native artwork and conventional readable UI. Mobile and no-WebGL checks passed; further spatial design remains.'),
 'P11':('in-progress','Q42-derived declared-purpose paths, reduced motion, large text and reading mode. Purpose changes presentation and never grants authority.'),
 'P12':('in-progress','16 human-facing module profiles and source links authored; independent capability/editorial qualification remains.'),
 'P13':('in-progress','Native campsite assets, terrain clearing and seven broad journey chapters implemented. Additional ecosystem environments remain.'),
 'P14':('in-progress','All204 inventoried source routes remain reachable; full replacement UI migration is incomplete.'),
 'P15':('in-progress','Audience invitations and module atlas available; complete task-oriented audience journeys remain.'),
 'P16':('in-progress','Keyboard, mobile, reduced-motion and fallback slices checked; full accessibility and translation qualification remains.'),
 'P17':('in-progress','Selected real correctness/performance evidence collected; device matrix and sustained reliability gates remain.'),
 'P18':('in-progress','Local preview on127.0.0.1:8800. No public deployment or remote repository creation has occurred.')}
p['status']='active implementation; package-specific evidence and remaining work recorded'
for item in p['packages']:
 item['status'],item['implementation_note']=notes[item['id']]
(out/'work-packages.json').write_text(json.dumps(p,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
lines=['# Implementation evidence — 16 November 2026','','Local review: http://127.0.0.1:8800/','',
 'The overhaul is in progress. Completed components below are reviewable; this ledger does not mark the whole website or every work package complete.','',
 '## Review entry points','','- Field studio: `/?chapter=southern-tablelands#field-studio`','- Journey: `/journey/`','- Real tests, production and benchmarks: `/lab/`','- Ecosystem module profiles: `/atlas/`','- Accessible graph publication: `/publication/reading.html`','',
 '## Work packages','','| Package | Status | Evidence and remaining work |','|---|---|---|']
for item in p['packages']:lines.append('| '+item['id']+' | '+item['status']+' | '+item['implementation_note']+' |')
lines+=['','## Journey handling','','Only seven broad regional chapters and month bands enter the website. The original timeline remains at its user-provided location. Longer visit segments are not asserted to be individual campsites. The scenery and colour palettes are artistic interpretations, not geospatial reconstructions.','',
 '## Evidence locations','','`evidence/baseline/`, `evidence/integration/`, `evidence/campsite/`, `tests/lab/runner-results.json`, `production/campsite/`, `publication/generated/q42-receipt.json`, `tests/journey/`.','',
 '## Known technical findings','','The legacy browser loader used full-width hashes for synthetic subject/predicate terms while the native query path uses canonical60-bit hashes. The benchmark adapter corrects the synthetic terms and records before/after hashes; source demos are preserved. This is not a claim that all legacy demonstrations have been repaired.','',
 'Native Q42 graph publication verifies decoded content, digest and blocks. Its current writer includes timestamps, so semantic reproducibility and byte reproducibility are reported separately. Native Synth production receipts for the authored assets verify strict replay.']
(out/'IMPLEMENTATION-STATUS.md').write_text('\n'.join(lines)+'\n',encoding='utf-8')
print('Updated18work-package statuses with evidence and explicit remaining work.')
