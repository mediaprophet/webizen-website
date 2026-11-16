from pathlib import Path
import sys,json
P=Path(__file__).resolve().parent;sys.path.insert(0,str(P.parent))
from native_bridge import SynthBridge
from build_art import lit
b=SynthBridge(P)
try:
 points=[[-90+(x+.5)*180/64,-90+(z+.5)*180/64] for z in range(2) for x in range(64)]
 source=(P/'inputs/fictional-world-frame.vibe').read_text()+'\n'+(P.parent/'aaa-terrain/material-field.vibe').read_text()+'\nlet cells=habitat_cells('+lit(points)+'); let palette=habitat_palette();'
 r=b.call('vibe_eval',{'source':source});assert len(r['bindings']['cells'])==128
 (P/'review/habitat-field/build-batch-compatibility.json').write_text(json.dumps({'passed':True,'native_batch_size':128,'result':r},indent=2));print('NATIVE_BUILD_BATCH_128_PASS')
finally:b.close()
