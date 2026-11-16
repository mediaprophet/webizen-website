from pathlib import Path
import sys,json
P=Path(__file__).resolve().parent;sys.path.insert(0,str(P.parent))
from native_bridge import SynthBridge
b=SynthBridge(P)
try:
 src=(P/'inputs/fictional-world-frame.vibe').read_text()+'\nlet camera={ground:terrain_height(-32.0,44.0),position:[-32.0,8.9,44.0],target:[-22.0,5.840921689,28.0]};'
 r=b.call('vibe_eval',{'source':src});(P/'assets/review-camera-close-evaluation.json').write_text(json.dumps(r,indent=2));print(r['bindings']['camera'])
finally:b.close()
