"""Assemble reviewed native production outputs; no scene logic or asset generation."""
from pathlib import Path
import json,sys,copy
from pin_proof_textures import pin
ROOT=Path(__file__).resolve().parents[1]
def main():
    scene=json.loads((ROOT/'production/native-world/scene.json').read_text())
    changes=[];textures=[]
    terrain=ROOT/'production/aaa-terrain/integration.json'
    if terrain.exists():
        record=json.loads(terrain.read_text());scene['assets']=[a for a in scene['assets'] if not a['id'].startswith('terrain--')]
        textures.extend(record.pop('textures'));scene['assets'].append(record);changes.append('Native habitat material field with UV01 and three actual material maps')
    ecology=ROOT/'production/aaa-environment/assets/integration.json'
    if not ecology.exists():ecology=ROOT/'production/aaa-environment/assets/integration-eucalyptus.json'
    if ecology.exists():
        data=json.loads(ecology.read_text());scene['assets'].extend(data['assets']);textures.extend(data['textures']);changes.append('Native ecology asset batches')
    sys.path.insert(0,str(ROOT/'production/aaa-proof/materials'))
    from namespace import namespace_asset
    namespaced=[]
    for asset in scene['assets']:
        if asset['url'].startswith(('/production/aaa-terrain/','/production/aaa-environment/')):
            output=ROOT/'production/aaa-proof/assets'/(asset['id']+'-'+asset['sha256'][:12]+'.10d')
            asset=namespace_asset(asset,ROOT/asset['url'].lstrip('/'),output)
        namespaced.append(asset)
    scene['assets']=namespaced
    scene['textures']=pin(list({(t['sha256'],t['role'],t.get('cutoff',.5)):t for t in textures}.values()))
    for key in ['vertices','triangles','bytes']:
        scene['totals'][key]=sum(a[key] for a in scene['assets'])
    scene['totals']['assets']=len(scene['assets']);scene['vertex_count']=scene['totals']['vertex_count']=scene['totals']['vertices'];scene['triangle_count']=scene['totals']['triangle_count']=scene['totals']['triangles']
    scene['totals'].pop('material_definitions',None)
    scene['production_status']='Native proof in progress; visual acceptance pending'
    scene['proof_changes']=changes
    out=ROOT/'production/aaa-proof';out.mkdir(exist_ok=True)
    (out/'scene-full.json').write_text(json.dumps(scene,indent=2))
    if '--review' in sys.argv:
        selection=json.loads((ROOT/'production/aaa-environment/assets/review-selection.json').read_text())
        ids=selection.get('asset_ids',selection.get('selected_asset_ids'))
        if ids is None:raise ValueError('Review selection has no asset ID list')
        lookup={a['id']:a for a in scene['assets']}
        scene['assets']=[lookup[id] for id in ids]
        for key in ['vertices','triangles','bytes']:scene['totals'][key]=sum(a[key] for a in scene['assets'])
        scene['vertex_count']=scene['totals']['vertex_count']=scene['totals']['vertices'];scene['triangle_count']=scene['totals']['triangle_count']=scene['totals']['triangles'];scene['totals']['assets']=len(scene['assets'])
        scene['composition_profile']='Explicitly curated bounded forest composition; original full scene and every source face remain preserved separately. No runtime geometry reduction.'
    (out/'scene.json').write_text(json.dumps(scene,indent=2))
    html=(ROOT/'site/native.html').read_text(encoding='utf-8')
    html=html.replace('<canvas id="native-world"','<canvas data-scene="/production/aaa-proof/scene.json" id="native-world"')
    html=html.replace('<title>Webizen — The living world</title>','<title>Webizen — Native world proof</title>')
    html=html.replace('<a href="/lab/">Tests &amp; benchmarks</a>','<a href="#native-proof-lab">The proof laboratory</a>')
    (ROOT/'site/aaa-proof.html').write_text(html,encoding='utf-8')
    (ROOT/'site/aaa-full-proof.html').write_text(html.replace('/production/aaa-proof/scene.json','/production/aaa-proof/scene-full.json'),encoding='utf-8')
    diagnostic=html.replace('/production/aaa-proof/scene.json','/production/native-world/scene.json').replace('<title>Webizen — Native world proof</title>','<title>Webizen — Native laboratory proof</title>')
    (ROOT/'site/lab-proof.html').write_text(diagnostic,encoding='utf-8')
    print(json.dumps({'scene':str(out/'scene.json'),'assets':len(scene['assets']),'changes':changes,'textures':len(scene['textures'])}))
if __name__=='__main__':main()
