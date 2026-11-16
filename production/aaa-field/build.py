from pathlib import Path
import sys,json,shutil,hashlib
ROOT=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT.parent))
from native_bridge import SynthBridge
b=SynthBridge(ROOT)
try:
 r=b.produce((ROOT/'light-field.vibe').read_text(encoding='utf-8-sig'),'light-field')
 (ROOT/'production-receipt.json').write_text(json.dumps(r,indent=2),encoding='utf8')
 for x in r['receipt']['resources']:
  shutil.copyfile(x['path'],ROOT/'assets'/x['key'])
 print(json.dumps({'replay':r['receipt'].get('strict_replay_verified'),'resources':[{k:x.get(k) for k in ['key','path','sha256']} for x in r['receipt']['resources']]}))
finally:b.close()
