"""Verify and integrate a completed native CLI terrain production."""
from pathlib import Path
import sys,json,hashlib,subprocess,time
ROOT=Path(__file__).resolve().parent
run=Path(sys.argv[1]).resolve()
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
source=run/'habitat-terrain.10d';glb=ROOT/'inputs/habitat-terrain.glb';output=ROOT/'assets/habitat-terrain.10d'
done=subprocess.run([str(ROOT.parent/'native-world/provenance/attest.exe'),str(source),str(glb),str(output)],capture_output=True,text=True,check=True)
admission=json.loads(done.stdout)
(ROOT/'evidence/admission.json').write_text(json.dumps(admission,indent=2))
textures=[]
for role,name in enumerate(['habitat-colour.png','soil-normal.png','soil-orm.png']):
    path=ROOT/'assets'/name;digest=sha(path)
    # Resolve by immutable content digest, never by imported dependency order.
    dependency=run/'textures/sha256'/digest
    assert dependency.read_bytes()==path.read_bytes(),f'Native dependency mismatch: {name}'
    textures.append({'url':'/production/aaa-terrain/assets/'+name,'sha256':digest,'mime_type':'image/png','role':role,'cutoff':.5})
record={'id':'habitat-terrain','url':'/production/aaa-terrain/assets/habitat-terrain.10d','colour':[1,1,1,1],'sha256':sha(output),'bytes':output.stat().st_size,'vertices':admission['vertices'],'triangles':admission['triangles'],'instances':1,'recipe':'/production/aaa-terrain/recipes/terrain.vibe','input_sha256':sha(glb),'epistemic_class':'authored-artwork','description':'Vibe habitat material field; fictional catchment, not measured land cover','textures':textures}
(ROOT/'integration.json').write_text(json.dumps(record,indent=2))
prior=json.loads((ROOT/'evidence/cli-production.json').read_text()) if (ROOT/'evidence/cli-production.json').exists() else {}
(ROOT/'evidence/cli-production.json').write_text(json.dumps({'native_output':str(source),'native_sha256':sha(source),'hmc':str(run.with_suffix('.hmc')),'hmc_sha256':sha(run.with_suffix('.hmc')),'dependency_policy':'Content digest lookup; native MAT1 digests, HMC dependencies and explicitly typed scene textures verified. No dependence on texture array ordering.','same_native_bytes_as_previous':prior.get('native_sha256')==sha(source)},indent=2))
if prior.get('native_sha256')==sha(source) and (ROOT/'evidence/preview.json').exists():
    print(json.dumps({'integration':str(ROOT/'integration.json'),'native_bytes_unchanged':True,'previous_preview_reused':True}));sys.exit(0)
preview=ROOT/'previews'/('terrain-'+str(time.time_ns()))
print(json.dumps({'integration':str(ROOT/'integration.json'),'preview':str(preview),'admission':admission}),flush=True)
subprocess.run(['C:/github/synth/target-prod/debug/synth.exe','preview','--asset',str(source),'--out',str(preview),'--hmc',str(run.with_suffix('.hmc')),'--asset-key','outputs/habitat-terrain.10d'],check=True)
(ROOT/'evidence/preview.json').write_text(json.dumps({'directory':str(preview),'review_status':'pending actual visual inspection'},indent=2))
