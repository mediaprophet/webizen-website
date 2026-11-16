"""Check/evaluate world authoring with the real current Synth Vibe host."""
from pathlib import Path
import sys,json,hashlib
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'production'))
from native_bridge import SynthBridge
source=(ROOT/'site/world/living-world.vibe').read_text(encoding='utf-8')
evidence=ROOT/'evidence/native-world';evidence.mkdir(parents=True,exist_ok=True)
bridge=SynthBridge(evidence)
try:
 checked=bridge.call('vibe_check',{'source':source})
 (evidence/'vibe-check.json').write_text(json.dumps(checked,indent=2),encoding='utf-8')
 print('CHECK',json.dumps(checked)[:3000])
 evaluated=bridge.call('vibe_eval',{'source':source+'\nlet settings = world_settings();\nlet positions = scene_positions();\n'})
 report={'source_sha256':hashlib.sha256((ROOT/'site/world/living-world.vibe').read_bytes()).hexdigest(),'evaluated_source_sha256':hashlib.sha256(source.encode()).hexdigest(),'runtime':'current native Synth Vibe host','result':evaluated}
 (evidence/'vibe-evaluation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
 layout={'schema':'webizen.vibe-world-layout/v1','source_uri':'/world/living-world.vibe','source_sha256':report['source_sha256'],'runtime':report['runtime'],'places':evaluated['bindings']['positions']}
 (ROOT/'data/native-world-layout.json').write_text(json.dumps(layout,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
 print('EVAL',json.dumps(evaluated,ensure_ascii=False)[:9000])
finally:bridge.close()
