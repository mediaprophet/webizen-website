"""Exercise the actual Vibe evaluator against terms read by the native Q42 codec."""
from pathlib import Path
import json,subprocess,sys,hashlib
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'production'))
from native_bridge import SynthBridge

def literal(value):
 if isinstance(value,dict):return '{'+','.join(k+':'+literal(v) for k,v in value.items())+'}'
 if isinstance(value,list):return '['+','.join(map(literal,value))+']'
 return json.dumps(value,ensure_ascii=False)

pointer=json.loads((ROOT/'publication/generated/current.json').read_text())
volume=ROOT/'publication/generated'/pointer['q42']
decoded=json.loads(subprocess.check_output([str(ROOT/'adapters/qualiadb/q42_web.exe'),'inspect',str(volume)],text=True,encoding='utf-8'))
prefixes=['https://webizen.au/'+p+'/' for p in ['world','relation','purpose']]
def local(value):
 for prefix in prefixes:
  if value.startswith(prefix):return value[len(prefix):]
 return value
edges=[dict(zip(['subject','predicate','object'],map(local,row))) for row in decoded['rows']]
places=json.loads((ROOT/'publication/generated/entities.json').read_text(encoding='utf-8'))['entities']
source=(ROOT/'site/world/living-world.vibe').read_text(encoding='utf-8')
out=ROOT/'evidence/native-world';out.mkdir(exist_ok=True,parents=True)
bridge=SynthBridge(out)
try:
 prelude='\nlet places = '+literal(places)+';\nlet edges = '+literal(edges)+';\n'
 bindings={}
 for purpose in ['learn','research','create','participate']:
  result=bridge.call('vibe_eval',{'source':source+prelude+'let route = observer_path(places,edges,"'+purpose+'");\n'})
  if result.get('status')!='ok':raise RuntimeError(result)
  bindings[purpose]=result['bindings']['route']
 changed=[{**e,'object':'canopy'} if e['subject']=='research' and e['predicate']=='startsAt' else e for e in edges]
 statements='\nlet changed = observer_path(places,'+literal(changed)+',"research");\n'
 statements+='let zoomed = input_camera(world_settings().camera,{kind:"zoom",dx:0.0,dy:100000.0});\n'
 result=bridge.call('vibe_eval',{'source':source+prelude+statements})
 if result.get('status')!='ok':raise RuntimeError(result)
 bindings.update(result['bindings']);ids={p['id'] for p in places}
 for purpose,start in [('learn','river'),('research','observatory'),('create','atelier'),('participate','commons')]:
  route=bindings[purpose];assert len(route)==len(ids) and set(route)==ids and route[0]==start,(purpose,route)
 assert bindings['changed'][0]=='canopy' and bindings['changed']!=bindings['research']
 assert bindings['zoomed']['zoom']==150.0
 receipt={'passed':True,'q42_sha256':hashlib.sha256(volume.read_bytes()).hexdigest(),'vibe_sha256':hashlib.sha256((ROOT/'site/world/living-world.vibe').read_bytes()).hexdigest(),'reader':decoded['reader'],'verified_blocks':decoded['verified_blocks'],'evaluator':'actual Synth vibe_eval','routes':{p:bindings[p] for p in ['learn','research','create','participate']},'changed_policy_route':bindings['changed'],'camera_upper_bound':bindings['zoomed']['zoom']}
 (out/'native-policy.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
 print(json.dumps(receipt))
finally:bridge.close()
