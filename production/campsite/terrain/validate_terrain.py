"""Verify existing native assets, source hashes, and every emitted triangle."""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parents[1]))
from build_art import parse_dxf

def digest(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()

if __name__=='__main__':
    record=json.loads((HERE/'terrain-production.json').read_text())
    receipt=record['production']['receipt']
    assert receipt['strict_replay_verified'] is True
    assert digest(HERE.parents[2]/'site/world/terrain.js')==record['terrain_source_sha256']
    # The production API hashes source text; Python's Windows file writer uses CRLF.
    assert hashlib.sha256((HERE/'recipes/clearing.vibe').read_text().encode('utf-8')).hexdigest()==receipt['source_sha256']
    for item in [receipt['hmc'],*receipt['inputs'],*receipt['resources']]:
        assert digest(item['path'])==item['sha256'],item['path']
    dxf=next(r for r in receipt['resources'] if r['kind']=='dxf')
    positions,indices=parse_dxf(Path(dxf['path']))
    authored=json.loads((HERE/'recipes/authored-terrain.json').read_text())
    browser=json.loads((HERE/'assets/catchment-clearing.mesh.json').read_text())['parts'][0]
    assert positions==browser['positions'] and indices==browser['indices']
    assert len(indices)==len(authored['indices'])==153600
    error=max(abs(positions[indices[i]*3+a]-authored['positions'][authored['indices'][i]*3+a]) for i in range(len(indices)) for a in range(3))
    assert error==record['max_dxf_coordinate_error_metres'] and error<.0001
    preview=Path(record['preview_dir'])
    review=json.loads((preview/'review-receipt.json').read_text())
    native=next(r for r in receipt['resources'] if r['kind']=='10d')
    assert review['source']['sha256']==native['sha256']
    assert review['projection']['triangle_count']==51200
    for view in review['views']:assert digest(preview/view['file'])==view['sha256']
    output={'strict_replay':True,'source_hashes_verified':True,'native_preview_hashes_verified':True,'triangles':len(indices)//3,'dxf_browser_exact':True,'max_coordinate_error_metres':error,'native_sha256':native['sha256']}
    (HERE/'assets/integrity-review.json').write_text(json.dumps(output,indent=2))
    print(json.dumps(output))
