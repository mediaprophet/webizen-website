"""Check the exact native-DXF -> browser mesh correspondence, not a substitute mesh."""
from pathlib import Path
import json,sys,math,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parents[1]));from build_art import parse_dxf
manifest=json.loads((HERE/'assets/vintage-windup-camper.manifest.json').read_text());mesh=json.loads((HERE/'assets/vintage-windup-camper.mesh.json').read_text());resources={r['key']:r for r in manifest['production']['receipt']['resources']}
assert len(mesh['parts'])==manifest['parts']>=160
triangles=0;vertices=[]
for part in mesh['parts']:
 p=part['positions'];ind=part['indices'];assert all(math.isfinite(v) for v in p);assert len(p)%3==0 and len(ind)%3==0;assert all(isinstance(i,int) and 0<=i<len(p)//3 for i in ind)
 resource=resources[part['name']+'.dxf'];path=Path(resource['path']);assert hashlib.sha256(path.read_bytes()).hexdigest()==part['source_sha256']==resource['sha256'];actual_positions,actual_indices=parse_dxf(path);assert p==actual_positions and ind==actual_indices
 triangles+=len(ind)//3;vertices.extend([p[i:i+3] for i in range(0,len(p),3)])
assert triangles==manifest['triangles']
assert manifest['production']['receipt']['strict_replay_verified'];assert manifest['sealing']['receipt']['strict_replay_verified']
report={'schema':'webizen.camper-integrity-review/v1','parts':len(mesh['parts']),'triangles':triangles,'exact_native_dxf_correspondence':True,'production_strict_replay':True,'sealing_strict_replay':True,'bounds':{'min':[min(v[i] for v in vertices) for i in range(3)],'max':[max(v[i] for v in vertices) for i in range(3)]},'attachment_points':manifest['attachment_points'],'native_asset_sha256':manifest['native']['sha256'],'qualification':manifest['qualification']}
(HERE/'assets/integrity-review.json').write_text(json.dumps(report,indent=2));print(json.dumps(report))