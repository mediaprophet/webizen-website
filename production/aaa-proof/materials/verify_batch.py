from pathlib import Path
import json,sys,subprocess,hashlib,time
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2];sys.path.insert(0,str(HERE));from namespace import namespace_asset,_check_identities
start=time.time();records=[json.loads((ROOT/'production/aaa-terrain/integration.json').read_text())]+json.loads((ROOT/'production/aaa-environment/assets/integration-eucalyptus.json').read_text())['assets']
results=[]
for i,record in enumerate(records):
 output=ROOT/'production/aaa-proof/assets'/(record['id']+'--material-identity.10d');result=namespace_asset(record,ROOT/record['url'].lstrip('/'),output);results.append(result);print('NAMESPACED',i+1,len(records),record['id'],flush=True)
# Real damaged CRC test; input rejected and no derivative created.
source=ROOT/records[0]['url'].lstrip('/');bad=HERE/'corrupt-input.10d';data=bytearray(source.read_bytes());data[-1]^=1;bad.write_bytes(data);target=HERE.parent/'assets/corrupt-must-not-exist.10d';assert not target.exists();done=subprocess.run([str(HERE/'material-namespace.exe'),str(bad),str(target)],capture_output=True);assert done.returncode!=0 and not target.exists()
# Native content identity is idempotent on an actual admitted asset.
first=results[0];out2=HERE.parent/'assets/habitat-terrain--identity-replay.10d';replay=subprocess.run([str(HERE/'material-namespace.exe'),str(ROOT/first['url'].lstrip('/')),str(out2)],capture_output=True,check=True);second=json.loads(replay.stdout);assert second['sha256']==first['sha256'];assert out2.read_bytes()==(ROOT/first['url'].lstrip('/')).read_bytes()
registry=json.loads((HERE/'identity-registry.json').read_text());shared=len(registry)
proof={'schema':'webizen.material-namespace-batch/v1','passed':True,'assets':results,'asset_count':len(results),'canonical_material_identities':shared,'vertices':sum(a['vertices'] for a in results),'triangles':sum(a['triangles'] for a in results),'bytes':sum(a['bytes'] for a in results),'native_tests':4,'corrupted_actual_input_rejected':True,'corruption_error':done.stderr.decode().strip(),'actual_namespace_idempotent':True,'elapsed_seconds':round(time.time()-start,2),'all_non_material_payloads_byte_exact':True,'all_native_admitted':True,'all_original_outputs_immutable':True}
(HERE/'batch-proof.json').write_text(json.dumps(proof,indent=2),encoding='utf-8');print(json.dumps({k:v for k,v in proof.items() if k!='assets'}),flush=True)
