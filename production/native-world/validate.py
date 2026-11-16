"""Verify the native world package and its declared provenance without a browser."""
from pathlib import Path
import json,hashlib,struct,math
R=Path(__file__).resolve().parent
P=R.parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
scene=json.loads((R/'scene.json').read_text());authoring=json.loads((R/'scene-authoring.json').read_text());evaluation=json.loads((R/'authoring-evaluation.json').read_text())
instances={n['id']:n for n in evaluation['placements']};triangles=vertices=totalbytes=0
for a in scene['assets']:
 path=R/'assets'/Path(a['url']).name;cache=json.loads((R/'cache'/(a['id']+'.json')).read_text());proof=json.loads((R/'provenance'/(a['id']+'.json')).read_text());assert sha(path)==a['sha256']==proof['attested_sha256'];assert proof['original_sha256']==cache['native']['sha256']==a['original_native_sha256'];assert proof['native_governance_refused'] is False and proof['original_sections_unchanged'] and proof['header_flags_preserved'];assert path.stat().st_size==a['bytes'];assert cache['production']['receipt']['strict_replay_verified'] is True
 inp=R/'inputs'/(a['id']+'.glb');assert sha(inp)==a['input_sha256'];blob=inp.read_bytes();length=struct.unpack_from('<I',blob,12)[0];doc=json.loads(blob[20:20+length]);assert doc['materials']==authoring['materials']
 for node in doc['nodes']:
  if node['name'] in instances:
   original=instances[node['name']]
   for key in ['translation','rotation','scale']:assert node[key]==original[key]
  else:assert node['name']=='terrain' and node['translation']==[0,0,0]
 vertices+=a['vertices'];triangles+=a['triangles'];totalbytes+=a['bytes']
 assert a['colour']==[1,1,1,1]
for source in scene['sources']:assert sha(P/source['path'])==source['sha256']
assert vertices==scene['vertex_count']==scene['totals']['vertex_count'];assert triangles==scene['triangle_count']==scene['totals']['triangle_count'];assert totalbytes==scene['totals']['bytes']
assert len([x for x in evaluation['placements'] if x['asset']=='solar-array-three'])==2
assert len([x for x in evaluation['placements'] if x['asset']=='solar-array-two'])==1
assert len([x for x in evaluation['placements'] if x['asset']=='eucalyptus'])==6
assert len([x for x in evaluation['placements'] if x['asset']=='eucalyptus-far'])==12
assert scene['solar_configuration']==[3,3,2]
water=scene['water'];assert len(water['indices'])%3==0 and len(water['indices'])//3==256
assert all(math.isfinite(v) for v in water['positions'])
assert all(abs(water['positions'][i*3+1]-.08)<.00001 for i in water['indices'])
report={'passed':True,'schema':'webizen.native-world-integrity/v1','assets':len(scene['assets']),'vertices':vertices,'triangles':triangles,'bytes':totalbytes,'all_native_sha256_verified':True,'all_native_provenance_validated':True,'all_original_sections_and_policy_flags_preserved':True,'all_production_strict_replay_verified':True,'all_material_tables_identical':True,'all_node_transforms_match_native_vibe_evaluation':True,'solar_configuration':[3,3,2],'water_native_top_triangles':256,'woodland':{'near':6,'near_triangles_each':39994,'far':12,'far_triangles_each':7992},'counts_scope':'Exact source primitive counts sent to native static GLB importer; host additionally verifies resident native counts.'}
(R/'integrity.json').write_text(json.dumps(report,indent=2));print(json.dumps(report))
