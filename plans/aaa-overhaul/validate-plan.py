"""Verify planning coverage, source identities, package ownership and local links."""
from pathlib import Path
import json,hashlib,re
ROOT=Path(__file__).resolve().parent
DOCS=Path('C:/github/qualiaDB/docs')
read=lambda name:json.loads((ROOT/name).read_text(encoding='utf-8'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
packages=read('work-packages.json');coverage=read('CAPABILITY-AND-DEMO-COVERAGE.json');ops=read('synth-operation-coverage.json')
ids={p['id'] for p in packages['packages']}
assert len(ids)==28 and all(p['default_locale']=='en-AU' for p in [packages,coverage])
route_names={r['source_route'] for r in coverage['routes']}
actual={p.relative_to(DOCS).as_posix() for p in DOCS.rglob('*.html')}
assert route_names==actual,(len(route_names),len(actual),sorted(route_names^actual))
assert len(route_names)==204
for route in coverage['routes']:
 assert sha(DOCS/route['source_route'])==route['source_sha256'],route['source_route']
 assert route['source_links_count_as_completion'] is False
 assert route['accessible_equivalent_required'] and route['input_inspect_export_local_query_required']
 assert route['dependency_closure_complete'] is False
 assert route['master_packages'] and set(route['master_packages'])<=ids
assert len(coverage['families'])==32
family_routes=[]
for family in coverage['families']:
 assert set(family['master_packages'])<=ids
 for field in ['actual_input_contract','native_controls_contract','output_and_data_contract','execution_and_acceptance_contract']:
  assert family[field],(family['id'],field)
 family_routes.extend(family['source_routes'])
assert set(family_routes)==route_names and len(family_routes)==204
for key,count in [('test_suites',44),('runner_sources',15),('historical_benchmark_artifacts',5),('dataset_descriptors',3),('module_ownership',16)]:assert len(coverage[key])==count,key
for item in coverage['historical_benchmark_artifacts']:
 assert sha(DOCS/item['source_route'])==item['source_sha256']
assert ops['count']==150 and len({o['id'] for o in ops['operations']})==150
assert sha(Path(ops['source']))==ops['source_sha256']
for op in ops['operations']:
 assert op['domain_package'] in ids and op['operation_instrument_owner']=='A15'
 assert op['example_dataset_contract'] and op['domain_reviewer']
milestones={m['id']:m for m in packages['milestones']};visited=set();active=set()
def visit(id):
 assert id in milestones,id
 assert id not in active,'Cycle: '+id
 if id in visited:return
 active.add(id)
 for dep in milestones[id]['dependencies']:visit(dep)
 active.remove(id);visited.add(id)
for id in milestones:visit(id)
assert len(milestones)==43
for n in range(7,21):assert 'A21.complete' in milestones[f'A{n:02}.complete']['dependencies']
assert 'A17.proof_ready' in milestones['A21.proofs_ready']['dependencies']
broken=[]
for doc in ROOT.glob('*.md'):
 for target in re.findall(r'\]\(([^)]+)\)',doc.read_text(encoding='utf-8')):
  if '://' in target or target.startswith(('#','C:','c:')):continue
  path=target.split('#')[0].strip('<>')
  if path and not (doc.parent/path).exists():broken.append((doc.name,target))
assert not broken,broken
report={'status':'planning pack checks passed; implementation and AAA acceptance remain pending','routes':204,'route_source_hashes_verified':204,'demo_subcontracts':32,'test_suite_registrations':44,'runner_identities':15,'historical_result_hashes_verified':5,'dataset_descriptors':3,'initial_module_groups':16,'canonical_synth_operations':150,'programme_packages':28,'acyclic_milestones':43,'full_expansion_blocked_until_reviewed_native_proofs':True,'proof_requires_actual_benchmark':True,'relative_document_links_resolve':True,'dependency_closure_honestly_marked_incomplete':True}
(ROOT/'plan-validation.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
print(json.dumps(report))
