from pathlib import Path
import sys,json
P=Path(__file__).resolve().parent
sys.path.insert(0,str(P.parent))
from native_bridge import SynthBridge
b=SynthBridge(P)
try:
 source=(P/'inputs/fictional-world-frame.vibe').read_text()+'\nlet cameras=[{x:-29.0,z:56.0,ground:terrain_height(-29.0,56.0)},{x:-26.0,z:54.0,ground:terrain_height(-26.0,54.0)},{x:-22.0,z:28.0,ground:terrain_height(-22.0,28.0)},{x:-30.0,z:48.0,ground:terrain_height(-30.0,48.0)}];'
 result=b.call('vibe_eval',{'source':source})
 (P/'assets/review-camera-evaluation.json').write_text(json.dumps(result,indent=2))
 print(result['bindings']['cameras'])
finally:b.close()
