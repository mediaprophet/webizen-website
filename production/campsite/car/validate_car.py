from pathlib import Path
import sys,json,hashlib,math,datetime
ROOT=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT.parents[1]))
from build_art import parse_dxf
m=json.loads((ROOT/'assets/car.manifest.json').read_text());data=json.loads((ROOT/'assets/car.json').read_text());resources={r['key']:r for r in m['production']['receipt']['resources']};checks=[]
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
for part in data['parts']:
 r=resources[part['name']+'.dxf'];positions,indices=parse_dxf(Path(r['path']))
 assert positions==part['positions'] and indices==part['indices'],part['name']
 assert sha(r['path'])==r['sha256']==part['source_sha256']
 assert all(math.isfinite(x) for x in positions)
 assert min(indices)>=0 and max(indices)<len(positions)//3
 checks.append({'part':part['name'],'triangles':len(indices)//3,'exact_dxf_faces':True,'sha256':r['sha256']})
assert sum(p['triangles'] for p in checks)==m['triangles']
for phase in ['production','sealing']:
 r=m[phase]['receipt'];assert r['strict_replay_verified'] is True
 assert sha(r['hmc']['path'])==r['hmc']['sha256']
 assert r['catalog']['count']==150
assert sha(m['native']['path'])==m['native']['sha256']
assert sha(m['recipe'])==m['production']['receipt']['source_sha256']
assert sha(ROOT/'recipes/passat-b7.obj')==m['sealing']['receipt']['inputs'][0]['sha256']
preview=Path(m['preview_dir'])
for f in ['front-view-assumed.png','side-view-assumed.png','three-quarter-view-assumed.png']:assert (preview/f).exists()
validation={'schema':'webizen.native-art-validation/v1','validated_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'parts':len(checks),'triangles':m['triangles'],'exact_dxf_faces':True,'model_sha256':m['native']['sha256'],'strict_replay_verified':True,'catalog_count':150,'checks':checks}
(ROOT/'review/geometry-validation.json').write_text(json.dumps(validation,indent=2))
def url(p):return '/production/campsite/car/'+Path(str(p).removeprefix('\\\\?\\')).relative_to(ROOT).as_posix()
def receipt_url(phase):return url(Path(str(m[phase]['receipt']['hmc']['path']).removeprefix('\\\\?\\')).with_suffix('.json'))
integration={'schema':'webizen.campsite-native-asset/v1','id':m['id'],'title':m['title'],'description':m['description'],'mesh_url':'/production/campsite/car/assets/car.json','model_url':url(m['native']['path']),'receipt_url':receipt_url('sealing'),'production_receipt_url':receipt_url('production'),'recipe_url':url(m['recipe']),'package_url':url(m['sealing']['receipt']['hmc']['path']),'source_package_url':url(m['production']['receipt']['hmc']['path']),'validation_url':'/production/campsite/car/review/geometry-validation.json','colour':'#bba570','role':'campsite-vehicle','units':'metres','up':'+Y','forward':'+Z','ground_y':0,'bounds':m['bounds'],'parts':m['parts'],'triangles':m['triangles'],'mesh_sha256':sha(ROOT/'assets/car.json'),'model_sha256':m['native']['sha256'],'reference_dimensions':m['reference_dimensions'],'references':m['references'],'epistemic_class':m['epistemic_class']}
(ROOT/'assets/car.integration.json').write_text(json.dumps(integration,indent=2));print(json.dumps(integration,indent=2))
