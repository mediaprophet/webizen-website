"""Build a web manifest from the actual retained native production outputs."""
from pathlib import Path
import json,hashlib
ROOT=Path(__file__).resolve().parent
def url(path):
 p=Path(str(path).removeprefix('\\\\?\\'))
 return '/production/'+p.relative_to(ROOT).as_posix()
assets=[]
descriptions={
 'canopy':'An open timber canopy shaped by twenty flowing ribs, gathered around an oculus. A shared place for inquiry, with limestone terraces and curved seats.',
 'tidal-vessel':'A hollow fluted vessel whose folded surface catches light like ripples. Authored sculptural geometry; no acoustic measurement is implied.',
 'memory-frond':'An authored botanical frond: a tapering spine and twenty curved pinnae. A study of repeated structure, not a calibrated species model.'}
for p in sorted((ROOT/'assets').glob('*.manifest.json')):
 d=json.loads(p.read_text(encoding='utf-8'));m=json.loads(Path(d['mesh_json']).read_text(encoding='utf-8'))
 coords=[part['positions'] for part in m['parts']];lo=[min(a[i] for a in coords for i in range(k,len(a),3)) for k in range(3)];hi=[max(a[i] for a in coords for i in range(k,len(a),3)) for k in range(3)]
 receipt=ROOT/'assets'/(d['id']+'.receipt.json');receipt.write_text(json.dumps({'geometry':d['production']['receipt'],'sealing':d['sealing']['receipt']},indent=2),encoding='utf-8')
 assets.append(dict(id=d['id'],title=d['title'],description=descriptions[d['id']],mesh_url=url(d['mesh_json']),model_url=url(d['native']['path']),receipt_url=url(receipt),recipe_url=url(d['recipe']),package_url=url(d['sealing']['saved_hmc']),colour='#b59152',role='architecture' if d['id']=='canopy' else 'specimen',bounds={'min':lo,'max':hi},parts=d['parts'],triangles=d['triangles'],native_sha256=d['native']['sha256'],mesh_sha256=hashlib.sha256(Path(d['mesh_json']).read_bytes()).hexdigest(),strict_replay_verified=d['production']['strict_replay_verified'] and d['sealing']['strict_replay_verified'],epistemic_class='authored-artwork',geometry_origin=d['geometry_origin'],preview_dir=url(d['preview_dir']) if d.get('preview_dir') else None))
out={'schema':'webizen.art-gallery/v1','default_locale':'en-AU','producer':'Synth native asset host','catalog_count':150,'assets':assets}
(ROOT/'assets/gallery.json').write_text(json.dumps(out,indent=2),encoding='utf-8')
print(json.dumps({'assets':[a['id'] for a in assets],'all_replay_verified':all(a['strict_replay_verified'] for a in assets)}))
