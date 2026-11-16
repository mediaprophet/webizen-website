"""Verify the emitted native assets and exact authored placement mapping."""
from pathlib import Path
import json,hashlib
ROOT=Path(__file__).resolve().parent; STAGE=ROOT.parents[1]
def read(p):return json.loads(p.read_text(encoding='utf8'))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
scene=read(ROOT/'scene.json'); original=read(ROOT.parent/'aaa-proof/scene-full.json')
evaluation=read(ROOT.parent/'aaa-environment/assets/placement-evaluation.json')['placements']
expected={p['id']:{k:p[k] for k in ['id','translation','rotation','scale']} for p in evaluation}
actual={};checks=[]
for asset in scene['assets']:
    file=STAGE/asset['url'].lstrip('/')
    assert sha(file)==asset['sha256']
    for placement in asset.get('placements',[]):
        assert placement['id'] not in actual
        actual[placement['id']]=placement
        authored=next(p for p in evaluation if p['id']==placement['id'])
        assert asset['id']=='prototype-'+authored['asset']
    if not asset.get('placements'):continue
    group=read(ROOT/'groups'/(asset['id']+'.json'))
    assert group['production']['strict_replay_verified'] is True
    provenance=read(STAGE/asset['provenance_receipt'].lstrip('/'))
    material=read(STAGE/asset['material_namespace_receipt'].lstrip('/'))
    assert provenance['passed'] and provenance['original_sections_unchanged']
    assert material['passed'] and material['non_material_sections_byte_exact'] and material['native_decoded_geometry_equal']
    source=asset['retained_prototype_input']
    assert sha(Path(source['path']))==source['sha256']
    checks.append({'id':asset['id'],'instances':len(asset['placements']),'source_triangles':asset['triangles'],'expanded_triangles':asset['triangles']*len(asset['placements']),'strict_replay_verified':True,'native_provenance_preserved':True,'native_geometry_and_nonmaterial_sections_preserved':True})
assert actual==expected
assert len(actual)==367
assert sum(a['triangles']*max(1,len(a.get('placements',[]))) for a in scene['assets'])==original['triangle_count']
for texture in scene['textures']:assert sha(STAGE/texture['url'].lstrip('/'))==texture['sha256']
result={'schema':'webizen.native-instancing-asset-verification/v1','passed':True,'placements_exact':len(actual),'prototype_checks':checks,'source_geometry':scene['totals'],'comparison':{'baked_bytes':original['totals']['bytes'],'prototype_scene_bytes':scene['totals']['bytes'],'saved_bytes':original['totals']['bytes']-scene['totals']['bytes'],'saved_fraction':1-scene['totals']['bytes']/original['totals']['bytes']},'scene_sha256':sha(ROOT/'scene.json'),'runtime_visual_test':'Pending combined WASM and browser verification; asset tests do not establish visual quality or frame rate'}
(ROOT/'asset-verification.json').write_text(json.dumps(result,indent=2))
print(json.dumps({k:result[k] for k in ['passed','placements_exact','comparison']}))
