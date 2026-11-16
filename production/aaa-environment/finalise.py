from pathlib import Path
import json,hashlib,collections
P=Path(__file__).resolve().parent
S=P.parent.parent
E=S/'evidence/aaa-art';E.mkdir(parents=True,exist_ok=True)
def read(p):return json.loads(Path(p).read_text())
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def local(url):
 p=S/url.lstrip('/')
 assert p.resolve().is_relative_to(S.resolve())
 return p
def write(p,v):Path(p).write_text(json.dumps(v,indent=2),encoding='utf8')
records=[];assets=[];textures={};checks=[]
for prefix in ['eucalyptus','bank-rock','grass','shrub','leaf-litter']:
 m=read(P/'assets'/('integration-'+prefix+'.json'))
 for a in m['assets']:
  assert a['id'] not in {x['id'] for x in assets}
  p=local(a['url']);assert sha(p)==a['sha256'] and p.stat().st_size==a['bytes']
  g=read(P/'groups'/(a['id']+'.json'));r=read(local(a['provenance_receipt']))
  assert g['production']['receipt']['strict_replay_verified']
  assert r['passed'] and r['original_sections_unchanged'] and r['header_flags_preserved']
  assert r['attested_sha256']==a['sha256'] and r['source_sha256']==sha(g['input'])
  assert r['original_native_sha256']==sha(g['native']['path'])
  assert r['vertices']==a['vertices'] and r['triangles']==a['triangles']
  assert a['vertices']>=g.get('source_vertices',a['vertices']) and a['vertices']<=a['triangles']*3
  records.append(g);assets.append(a)
  checks.append({'id':a['id'],'native_sha256':a['sha256'],'strict_replay_verified':True,'provenance_sections_preserved':True,'source_hash_verified':True,'triangle_count_verified':True})
 for t in m['textures']:
  assert sha(local(t['url']))==t['sha256']
  textures[(t['sha256'],t['role'])]=t
placements=read(P/'assets/placement-evaluation.json')['placements']
counts=dict(collections.Counter(p['asset'] for p in placements))
manifest={'schema':'webizen.aaa-native-environment/v1','frame':'existing fictional catchment, Y-up metres','epistemic_class':'authored-artwork','assets':assets,'textures':list(textures.values()),'placements':'/production/aaa-environment/assets/placement-evaluation.json','review_status':'Native front/side/quarter and assembled understorey study inspected. Whole-world visual acceptance and performance qualification pending.','integration_requirement':'Namespace independent MAT1 material IDs through native codec before shared-scene merge. Do not mutate original outputs.','triangles':sum(a['triangles'] for a in assets),'vertices':sum(a['vertices'] for a in assets),'bytes':sum(a['bytes'] for a in assets),'instances':sum(a['instances'] for a in assets),'placement_counts':counts,'biological_individuals_note':'Shrub branch and leaf assets share14 placements and constitute14 shrubs, not28 plants. These are authored forms, not taxonomically verified specimens.','assembled_study':'/production/aaa-environment/previews/understorey-study-1794799841879897300/frame-0000.png'}
write(P/'assets/integration.json',manifest)
write(E/'native-environment-validation.json',{'passed':True,'scope':'Hash, size, native replay receipt, retained source geometry and provenance validation; not whole-world visual or performance acceptance.','manifest_sha256':sha(P/'assets/integration.json'),'assets':checks,'texture_bindings_verified':len(textures),'counts':{k:manifest[k] for k in ['triangles','vertices','bytes','instances','placement_counts']}})
print(json.dumps({k:manifest[k] for k in ['triangles','vertices','bytes','instances','placement_counts']},indent=2));print('ASSETS',len(assets),'TEXTURES',len(textures),'MANIFEST',P/'assets/integration.json')

