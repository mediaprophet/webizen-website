from pathlib import Path
import json
from synth_bridge import SynthBridge
ROOT=Path(__file__).resolve().parents[1]
bridge=SynthBridge(ROOT)
try:
 source=(ROOT/'recipes/living-catchment.vibe').read_text(encoding='utf-8')
 data=bridge.produce(source,'living-catchment',[{'key':'landscape.obj','kind':'obj','path':'recipes/living-catchment.obj'}])
 resource=data['receipt']['resources'][0]
 record={'model_url':resource['url'],'source':'/recipes/living-catchment.vibe','geometry_source':'/recipes/living-catchment.obj','production':data,'rendering_note':'The review landscape uses the same authored terrain coordinates in Three.js. This native asset is independently produced and inspectable in QualiaPortal.'}
 (ROOT/'data/terrain-production.json').write_text(json.dumps(record,indent=2),encoding='utf-8')
 print(json.dumps({'model_url':resource['url'],'strict_replay':data['receipt']['strict_replay_verified'],'catalog':data['receipt']['catalog']['count'],'sha256':resource['sha256']}))
finally:bridge.close()
