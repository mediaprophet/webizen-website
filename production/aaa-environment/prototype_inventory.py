from pathlib import Path
import json,hashlib,collections,shutil
P=Path(__file__).resolve().parent
S=P.parent.parent
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
read=lambda p:json.loads(Path(p).read_text())
shutil.copyfile(S/'evidence/aaa-art/NATIVE-INSTANCING-HANDOVER.md',P/'NATIVE-INSTANCING-HANDOFF.md')
placements=read(P/'assets/placement-evaluation.json')['placements']
by=collections.defaultdict(list)
for p in placements:by[p['asset']].append(p['id'])
prototypes=[]
for id,ids in by.items():
 if id.startswith('eucalyptus'):
  budget=40000 if id.endswith('near') else 8000
  source=P.parent/'native-world/lod'/('eucalyptus-'+str(budget)+'.json');data=read(source)
  native=None;recipe=source.with_suffix('.vibe');receipt=data['production']['receipt'];textures=[]
  stage='Native simplified exact face source available; standalone attributed prototype must be sealed and attested for new batch admission.'
 else:
  manifest=P/'assets'/(id+'.manifest.json');m=read(manifest);source=Path(m['mesh_json']);data=read(source)
  native={k:m['native'][k] for k in ['path','sha256','bytes']};recipe=Path(m['surface_recipe']);receipt=m['surface_production']['receipt'];textures=[{'role':role,'path':v['path'],'sha256':v['sha256']} for role,v in m['textures'].items()]
  stage='Native UV/PBR prototype sealed; existing standalone prototype requires provenance admission attestation, as already applied to grouped derivatives.'
 assert receipt['strict_replay_verified']
 parts=data['parts'];nv=sum(len(p['positions'])//3 for p in parts);nt=sum(len(p['indices'])//3 for p in parts)
 bounds={'min':[min(p['positions'][a::3]) for a in range(3) for p in []]} if False else {'min':[min(min(p['positions'][a::3]) for p in parts) for a in range(3)],'max':[max(max(p['positions'][a::3]) for p in parts) for a in range(3)]}
 item={'id':id,'source_mesh':{'path':str(source),'sha256':sha(source)},'native_prototype':native,'source_recipe':{'path':str(recipe),'sha256':sha(recipe)},'strict_replay_verified':True,'triangles':nt,'source_vertices':nv,'material_names':list(data['palette']),'textures':textures,'local_bounds':bounds,'placement_asset_key':id,'placement_ids':ids,'instance_count':len(ids),'epistemic_class':'authored-artwork','admission_status':stage}
 prototypes.append(item)
out={'schema':'webizen.native-prototype-inventory/v1','purpose':'Concrete native per-prototype instancing integration input; not completed GPU batch support or performance acceptance.','frame':'local prototype Y-up metres; placement transforms in existing fictional catchment metres','placements':{'path':str(P/'assets/placement-evaluation.json'),'sha256':sha(P/'assets/placement-evaluation.json'),'quaternion_order':'xyzw','matrix_convention':'compose translation * rotation * scale, then common scene orbit frame exactly once'},'prototype_count':len(prototypes),'instance_count':len(placements),'abi_version_observed':2,'instance_record_bytes_observed':128,'instance_bytes_arithmetic':128*len(placements),'prototypes':prototypes,'blocker':'A21 production-readiness: current merged static payload duplicates native geometry. Existing GPU instancing providers need native per-prototype draw lifecycle wiring and measured acceptance.'}
(P/'assets/prototype-inventory.json').write_text(json.dumps(out,indent=2),encoding='utf8')
print('PROTOTYPES',len(prototypes),'SOURCE_TRIANGLES',sum(p['triangles'] for p in prototypes),'INSTANCES',len(placements))

