"""Produce immutable native prototype containers from the exact retained ecology inputs."""
from pathlib import Path
import sys,json,hashlib,importlib.util
ROOT=Path(__file__).resolve().parent
STAGE=ROOT.parents[1]
ECO=ROOT.parent/'aaa-environment'
sys.path.insert(0,str(ECO));sys.path.insert(0,str(ROOT.parent))
import assemble
from native_bridge import SynthBridge
spec=importlib.util.spec_from_file_location('namespace',ROOT.parent/'aaa-proof/materials/namespace.py')
namespace=importlib.util.module_from_spec(spec);spec.loader.exec_module(namespace)
for folder in ['inputs','recipes','assets','groups','provenance','textures','artifacts']:(ROOT/folder).mkdir(exist_ok=True)
assemble.ROOT=ROOT;assemble.URL='/production/aaa-instancing/'
def read(p):return json.loads(Path(p).read_text(encoding='utf8'))
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
inventory=read(ECO/'assets/prototype-inventory.json')
placements=read(ECO/'assets/placement-evaluation.json')['placements']
records=[];textures={}
bridge=SynthBridge(ROOT)
try:
 for item in inventory['prototypes']:
  key=item['id']
  source=ROOT.parent/'native-world/lod'/('eucalyptus-'+('40000' if key.endswith('near') else '8000')+'.json') if key.startswith('eucalyptus') else ECO/'assets'/(key+'.mesh.json')
  data=read(source);maps=None if key.startswith('eucalyptus') else {next(iter(data['palette'])):data['textures']}
  record=assemble.compile_group(bridge,'prototype-'+key,data['parts'],data['palette'],[{'id':key,'translation':[0,0,0],'rotation':[0,0,0,1],'scale':[1,1,1]}],maps)
  output=STAGE/record['url'].lstrip('/')
  record=namespace.namespace_asset(record,output,ROOT.parent/'aaa-proof/assets'/(record['id']+'-'+sha(output)[:12]+'.10d'))
  assert record['triangles']==item['triangles']
  selected=[p for p in placements if p['asset']==key]
  record={k:record[k] for k in ['id','url','sha256','bytes','vertices','triangles','colour','material_namespace_receipt','provenance_receipt','recipe']}
  record['placements']=[{'id':p['id'],'translation':p['translation'],'rotation':p['rotation'],'scale':p['scale']} for p in selected]
  record['retained_prototype_input']={'path':str(source),'sha256':sha(source)}
  records.append(record)
  group=read(ROOT/'groups'/('prototype-'+key+'.json'))
  for texture in group['textures']:textures[(texture['sha256'],texture['role'])]=texture
finally:bridge.close()
result={'schema':'webizen.native-ecology-prototypes/v1','assets':records,'textures':list(textures.values()),'placements_sha256':sha(ECO/'assets/placement-evaluation.json'),'source_triangles':sum(a['triangles'] for a in records),'expanded_triangles':sum(a['triangles']*len(a['placements']) for a in records),'instances':sum(len(a['placements']) for a in records),'bytes':sum(a['bytes'] for a in records),'vertices':sum(a['vertices'] for a in records)}
assert result['instances']==367
(ROOT/'integration.json').write_text(json.dumps(result,indent=2),encoding='utf8')
print(json.dumps({k:result[k] for k in ['source_triangles','expanded_triangles','instances','bytes','vertices']}))
