"""Native Synth terrain production from the exact shared browser heightfield."""
from pathlib import Path
import sys,json,subprocess,hashlib,time,math
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parents[1]));from native_bridge import SynthBridge;from build_art import parse_dxf
source='''requires [capability("capability.invoke")];
let landscape=capability.invoke("hypermedia.manifold.import_mesh",{input:"landscape.obj",format:"obj"});
let native=capability.invoke("hypermedia.manifold.compile_10d_pbr",{mesh:landscape});
let exchange=capability.invoke("hypermedia.manifold.project_dxf",{mesh:landscape,layer:"catchment"});
let mut asset={files:[{key:"catchment-clearing.10d",resource:native},{key:"catchment-clearing.dxf",resource:exchange}]};'''
if __name__=='__main__':
 (HERE/'recipes/clearing.vibe').write_text(source,encoding='utf-8');bridge=SynthBridge(HERE)
 try:
  result=bridge.produce(source,'catchment-clearing',inputs=[{'key':'landscape.obj','kind':'obj','path':str(HERE/'recipes/catchment-clearing.obj')}]);print('PRODUCED',result['receipt']['strict_replay_verified'],flush=True)
  for r in result['receipt']['resources']:r['url']='/production/campsite/terrain/artifacts/'+Path(r['path']).parent.name+'/'+r['key']
  result['package_url']='/production/campsite/terrain/artifacts/'+Path(result['saved_hmc']).name
  result['receipt_url']='/production/campsite/terrain/artifacts/'+Path(result['saved_hmc']).with_suffix('.json').name
  resources=result['receipt']['resources'];model=next(r for r in resources if r['key'].endswith('.10d'));dxf=next(r for r in resources if r['key'].endswith('.dxf'));positions,indices=parse_dxf(Path(dxf['path']));authored=json.loads((HERE/'recipes/authored-terrain.json').read_text());assert len(indices)==len(authored['indices'])
  # Compare every emitted triangle-coordinate in native face order to the authored import.
  max_error=max(abs(positions[indices[i]*3+axis]-authored['positions'][authored['indices'][i]*3+axis]) for i in range(len(indices)) for axis in range(3))
  assert max_error<.0001, max_error
  assets=HERE/'assets';assets.mkdir(exist_ok=True);mesh={'schema':'webizen.native-mesh-adapter/v1','id':'catchment-clearing','palette':{'terrain':{'color':'#718267','roughness':1,'metalness':0}},'parts':[{'name':'catchment-clearing','material':'terrain','positions':positions,'indices':indices,'source_sha256':dxf['sha256']}]};(assets/'catchment-clearing.mesh.json').write_text(json.dumps(mesh,separators=(',',':')),encoding='utf-8')
  record={'model_url':model['url'],'source':'/production/campsite/terrain/recipes/clearing.vibe','geometry_source':'/production/campsite/terrain/recipes/catchment-clearing.obj','mesh_json':'/production/campsite/terrain/assets/catchment-clearing.mesh.json','production':result,'heightfield':json.loads((HERE/'heightfield-check.json').read_text()),'same_authored_geometry':True,'max_dxf_coordinate_error_metres':max_error,'terrain_source_sha256':hashlib.sha256((HERE.parents[2]/'site/world/terrain.js').read_bytes()).hexdigest(),'rendering_note':'Browser heightAt/terrainMesh and native imported mesh share the same160-grid authored terrain, with a level campsite clearing and smooth transition. Native DXF correspondence checked for every triangle coordinate.'}
  (HERE/'terrain-production.json').write_text(json.dumps(record,indent=2),encoding='utf-8');(assets/'terrain.manifest.json').write_text(json.dumps(record,indent=2),encoding='utf-8')
  preview=HERE/'previews'/('catchment-clearing-'+str(int(time.time())));preview.parent.mkdir(exist_ok=True);subprocess.run(['C:/github/synth/target-prod/debug/synth.exe','preview','--asset',model['path'],'--out',str(preview)],check=True);record['preview_dir']=str(preview);(HERE/'terrain-production.json').write_text(json.dumps(record,indent=2),encoding='utf-8');(assets/'terrain.manifest.json').write_text(json.dumps(record,indent=2),encoding='utf-8');print(json.dumps({'triangles':len(indices)//3,'max_error':max_error,'preview':str(preview),'strict_replay':result['receipt']['strict_replay_verified'],'native_sha256':model['sha256']}),flush=True)
 finally:bridge.close()
