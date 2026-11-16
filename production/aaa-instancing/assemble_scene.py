"""Replace expanded ecology with exact immutable native prototypes and placement records."""
from pathlib import Path
import json,hashlib,sys
ROOT=Path(__file__).resolve().parent
STAGE=ROOT.parents[1]
sys.path.insert(0,str(STAGE/'tooling'))
from pin_proof_textures import pin
base=json.loads((ROOT.parent/'aaa-proof/scene-full.json').read_text())
prototypes=json.loads((ROOT/'integration.json').read_text())
expanded=[a for a in base['assets'] if a['id'].startswith('ecology-')]
assert len(expanded)==36
assert sum(a['triangles'] for a in expanded)==prototypes['expanded_triangles']
base['assets']=[a for a in base['assets'] if not a['id'].startswith('ecology-')]+prototypes['assets']
for asset in base['assets']:
    file=STAGE/asset['url'].lstrip('/')
    assert hashlib.sha256(file.read_bytes()).hexdigest()==asset['sha256'],asset['id']
    assert file.stat().st_size==asset['bytes'],asset['id']
base['textures']=pin(list({(t['sha256'],t['role']):t for t in base['textures']+prototypes['textures']}.values()))
base['expanded_triangle_count']=base['triangle_count']
base['triangle_count']=sum(a['triangles'] for a in base['assets'])
base['vertex_count']=sum(a['vertices'] for a in base['assets'])
base['totals']={'assets':len(base['assets']),'bytes':sum(a['bytes'] for a in base['assets']),'vertices':base['vertex_count'],'triangles':base['triangle_count'],'vertex_count':base['vertex_count'],'triangle_count':base['triangle_count'],'expanded_triangles':base['expanded_triangle_count'],'ecology_placements':prototypes['instances'],'native_instance_records':sum(max(1,len(a.get('placements',[]))) for a in base['assets'])}
base['instancing']={'schema':'webizen.native-prototype-groups/v1','provider':'QualiaDB PortalGpu / Etalon native instance stream ABI v2','placement_evaluation_sha256':prototypes['placements_sha256'],'prototype_count':len(prototypes['assets']),'geometry':'Original retained local prototypes; all 367 authored placement transforms; source and expanded triangle counts separated','runtime_lod':'Existing authored near/mid prototype choices retained; no automatic LOD substitution'}
assert sum(a['triangles']*max(1,len(a.get('placements',[]))) for a in base['assets'])==base['expanded_triangle_count']
(ROOT/'scene.json').write_text(json.dumps(base,indent=2))
page=(STAGE/'site/aaa-proof.html').read_text(encoding='utf8').replace('/production/aaa-proof/scene.json','/production/aaa-instancing/scene.json').replace('Native world proof','Native instanced world proof')
(STAGE/'site/aaa-instanced-proof.html').write_text(page,encoding='utf8')
print(json.dumps(base['totals'],indent=2))
