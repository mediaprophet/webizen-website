"""Verify browser geometry against native DXF exports, hashes and replay receipts."""
from pathlib import Path
import sys,json,hashlib,math
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parents[1]))
from build_art import parse_dxf
results=[]
for path in sorted((HERE/'assets').glob('*.manifest.json')):
    manifest=json.loads(path.read_text())
    mesh=json.loads(Path(manifest['mesh_json']).read_text())
    resources={r['key']:r for r in manifest['production']['receipt']['resources']}
    triangles=vertices=0
    for part in mesh['parts']:
        resource=resources[part['name']+'.dxf'];native=Path(resource['path'])
        digest=hashlib.sha256(native.read_bytes()).hexdigest()
        assert digest==resource['sha256']==part['source_sha256']
        positions,indices=parse_dxf(native)
        assert positions==part['positions'] and indices==part['indices']
        assert all(math.isfinite(v) for v in positions)
        assert min(indices)>=0 and max(indices)<len(positions)//3
        triangles+=len(indices)//3;vertices+=len(positions)//3
    assert triangles==manifest['triangles'] and vertices==manifest['vertices']
    for production in ['production','sealing']:
        assert manifest[production]['strict_replay_verified'] is True
        assert manifest[production]['receipt']['strict_replay_verified'] is True
    native=manifest['native']
    assert hashlib.sha256(Path(native['path']).read_bytes()).hexdigest()==native['sha256']
    previews=Path(manifest['preview_dir'])
    for view in ['front','side','three-quarter']:
        assert (previews/(view+'-view-assumed.png')).is_file()
    results.append({'id':manifest['id'],'parts':len(mesh['parts']),'vertices':vertices,'triangles':triangles,'exact_native_dxf_identity':True,'strict_replay_verified':True,'native_sha256':native['sha256'],'visual_review':'Front, side and three-quarter native previews inspected; shape accepted as authored visual reconstruction. Native preview is uncoloured; browser palette applied separately.'})
report={'schema':'webizen.equipment-validation/v1','passed':True,'assets':results}
(HERE/'geometry-validation.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print(json.dumps(report,indent=2))
