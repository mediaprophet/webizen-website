from pathlib import Path
import json,shutil,math,hashlib,subprocess,time
from native_bridge import SynthBridge
from build_art import ROOT,parse_dxf
shutil.copy2('C:/github/synth/production/webizen-world-v2/recipes/living-catchment.obj',ROOT/'recipes/living-catchment.obj')
source='''requires [capability("capability.invoke")];
let landform=capability.invoke("hypermedia.manifold.import_mesh",{input:"terrain.obj",format:"obj"});
let meshfile=capability.invoke("hypermedia.manifold.compile_10d_pbr",{mesh:landform});
let faces=capability.invoke("hypermedia.manifold.project_dxf",{mesh:landform,layer:"catchment"});
let surfacemaps=capability.invoke("hypermedia.surface.synthesize_maps",{side:256,seed:2317,tint:[0.55,0.59,0.43],contrast:0.22,relief:0.003,bands:[7,3],band_strength:0.12,roughness:0.94,metallic:0.0});
let maps=capability.invoke("hypermedia.surface.pack_pbr_set",{colour:surfacemaps.colour,normal:surfacemaps.normal,orm:surfacemaps.orm});
let mut asset={files:[{key:"catchment.10d",resource:meshfile},{key:"catchment.dxf",resource:faces},{key:"ground-colour.png",resource:maps.colour},{key:"ground-normal.png",resource:maps.normal},{key:"ground-orm.png",resource:maps.orm}]};'''
(ROOT/'recipes/catchment.vibe').write_text(source)
bridge=SynthBridge(ROOT)
try:result=bridge.produce(source,'catchment',[{'key':'terrain.obj','kind':'obj','path':'recipes/living-catchment.obj'}])
finally:bridge.close()
resources={r['key']:r for r in result['receipt']['resources']}
def url(path):return '/production/'+Path(str(path).removeprefix('\\\\?\\')).relative_to(ROOT).as_posix()
positions,indices=parse_dxf(Path(resources['catchment.dxf']['path']))
colours=[];uvs=[]
for i in range(0,len(positions),3):
 x,y,z=positions[i:i+3];d=abs(x-(math.sin(z*.044)*10+math.sin(z*.095)*3));t=max(0,min(1,(y-8)/16));bank=max(0,min(1,(5-d)/3));base=[.23+.23*t,.30+.16*t,.19+.23*t];sand=[.56,.49,.34];colours.extend([v*(1-bank)+sand[j]*bank for j,v in enumerate(base)]);uvs.extend([x/12,z/12])
mesh={'schema':'webizen.native-mesh-adapter/v1','id':'catchment','palette':{'ground':{'color':'#ffffff','roughness':.94,'metalness':0}},'textures':{'ground':{k:url(resources['ground-'+k+'.png']['path']) for k in ['colour','normal','orm']}},'parts':[{'name':'catchment','material':'ground','positions':positions,'indices':indices,'colours':colours,'uvs':uvs,'source_sha256':resources['catchment.dxf']['sha256']}],'colour_scope':'Authored geological/bank palette for presentation; not measured land cover'}
path=ROOT/'assets/catchment.mesh.json';path.write_text(json.dumps(mesh,separators=(',',':')))
model=resources['catchment.10d'];record={'id':'catchment','title':'Living catchment','role':'environment','mesh_url':url(path),'model_url':url(model['path']),'recipe_url':'/production/recipes/catchment.vibe','receipt_url':'/production/assets/catchment.receipt.json','package_url':url(result['saved_hmc']),'triangles':len(indices)//3,'native_sha256':model['sha256'],'mesh_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'bounds':{'min':[min(positions[k::3]) for k in range(3)],'max':[max(positions[k::3]) for k in range(3)]},'strict_replay_verified':result['strict_replay_verified'],'epistemic_class':'authored-artwork','geometry_origin':'Existing authored catchment OBJ imported, sealed and exported by native Synth; every exported DXF face preserved in browser adapter','textures':mesh['textures']}
(ROOT/'assets/catchment.receipt.json').write_text(json.dumps(result['receipt'],indent=2));(ROOT/'assets/environment.json').write_text(json.dumps(record,indent=2));print(json.dumps(record),flush=True)
preview=ROOT/'previews'/('catchment-'+str(int(time.time())));subprocess.run(['C:/github/synth/target-prod/debug/synth.exe','preview','--asset',model['path'],'--out',str(preview)],check=True);record['preview_dir']=url(preview);(ROOT/'assets/environment.json').write_text(json.dumps(record,indent=2));print('PREVIEW',preview)