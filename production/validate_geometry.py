from pathlib import Path
import json,sys,math
sys.path.insert(0,str(Path(__file__).parent))
from build_art import ROOT,parse_dxf
results=[]
for path in (ROOT/'assets').glob('*.manifest.json'):
 d=json.loads(path.read_text());mesh=json.loads(Path(d['mesh_json']).read_text());resources={r['key']:r for r in d['production']['receipt']['resources']};total=0
 for part in mesh['parts']:
  r=resources[part['name']+'.dxf'];positions,indices=parse_dxf(Path(r['path']))
  assert positions==part['positions'] and indices==part['indices'],part['name']
  assert all(math.isfinite(x) for x in positions)
  assert max(indices)<len(positions)//3
  total+=len(indices)//3
 assert total==d['triangles']
 results.append({'id':d['id'],'triangles':total,'parts':len(mesh['parts']),'complete_dxf_face_adapter':True,'native_replay':d['production']['strict_replay_verified'] and d['sealing']['strict_replay_verified']})
out=ROOT.parent/'evidence/art';out.mkdir(parents=True,exist_ok=True);(out/'geometry-validation.json').write_text(json.dumps({'schema':'webizen.art-validation/v1','results':results},indent=2));print(json.dumps(results))