from pathlib import Path
import importlib.util,sys,json,hashlib,shutil,subprocess,math
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'data/publications/aaa/terrain-model-v2';OUT.mkdir(parents=True,exist_ok=True)
SOURCE=ROOT/'data/acquisition/aaa/canberra-reference-corrected';EVID=ROOT/'evidence/aaa-data'
sys.path.insert(0,str(ROOT/'production'));sys.path.insert(0,str(ROOT/'production/native-world'))
from native_bridge import SynthBridge
spec=importlib.util.spec_from_file_location('retained_native_glb',ROOT/'production/native-world/build.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
mesh=json.loads((SOURCE/'mesh-inspection-fixed.json').read_text());hf=json.loads((SOURCE/'heightfield.json').read_text())
pos=[v for row in mesh['positions'] for v in row];ind=mesh['indices'];assert len(pos)==16992*3 and len(ind)==33462*3
scale=1/30;ground=min(hf['values_m']);t={'id':'canberra-reference-model','translation':[0,-ground*scale,0],'rotation':[-math.sqrt(.5),0,0,math.sqrt(.5)],'scale':[scale]*3}
group={'parts':[{'positions':pos,'indices':ind}],'material':0,'instances':[t]}
material={'name':'Neutral relief material; not observed landcover','doubleSided':True,'pbrMetallicRoughness':{'baseColorFactor':[.38,.34,.27,1],'metallicFactor':0,'roughnessFactor':.94}}
inputpath=OUT/'reference-terrain-model.glb';inputpath.write_bytes(module.glb(group,[material]))
source="""requires [capability("capability.invoke")];
let terrain=capability.invoke("hypermedia.manifold.import_mesh",{input:"terrain.glb",format:"glb"});
let model=capability.invoke("hypermedia.manifold.compile_10d_pbr",{mesh:terrain,topology:false});
let mut asset={files:[{key:"reference-terrain-model.10d",resource:model}]};
"""
(OUT/'reference-terrain-model.vibe').write_text(source,encoding='utf-8')
bridge=SynthBridge(OUT)
try:
 result=bridge.produce(source,'terrain-model',inputs=[{'key':'terrain.glb','kind':'glb','path':str(inputpath)}])
 assert result['receipt']['strict_replay_verified']
 native=next(x for x in result['receipt']['resources'] if x['key'].endswith('.10d'))
 raw=OUT/'reference-terrain-model.unattested.10d';shutil.copyfile(native['path'],raw)
 dest=OUT/'reference-terrain-model.10d';assert not dest.exists()
 admission=json.loads(subprocess.run([str(ROOT/'native/data-pack-builder/terrain-admit.exe'),str(raw),str(SOURCE/'reference-terrain.10d'),str(dest)],check=True,capture_output=True).stdout)
 assert admission['vertices']==16992 and admission['triangles']==33462
 record={'schema':'webizen.native-terrain-model/v1','id':'canberra-public-reference-model','title':'Canberra reference landscape,1:30 scale model','not_personal_campsite':True,'source_sha256':sha(SOURCE/'reference-terrain.10d'),'source_bundle_sha256':sha(SOURCE/'source-tiles.qdem'),'orientation_correction':'Native source generate_terrain_mesh corrected clockwise -Z to counterclockwise +Z; all original sampled positions retained unchanged; indices only swap winding', 'native_import_input_sha256':sha(inputpath),'recipe_sha256':sha(OUT/'reference-terrain-model.vibe'),'native_transform':t,'display_frame':'Y-up; +X grid east; +Z grid north; source elevation541m mapped to displayY0. Native GLB importer applies node transform.','scale_denominator':30,'vertical_exaggeration':1,'height_range_source_m':[min(hf['values_m']),max(hf['values_m'])],'material_interpretation':'Uniform neutral relief shading, not satellite imagery or measured landcover','source_rights':'Original native DEM provenance sidecar preserved byte-for-byte, including source raster bundle and attribution. Display scaling/rotation declared in receipt; no artwork-only licence applied.','production':result,'admission':admission,'asset':{'id':'canberra-reference-terrain','url':'/data/publications/aaa/terrain-model-v2/'+dest.name,'sha256':sha(dest),'bytes':dest.stat().st_size,'colour':[1,1,1,1],'vertices':16992,'triangles':33462},'bounds_display':{'x':[-hf['width']*hf['cell_size_m']/60,(hf['width']/2-1)*hf['cell_size_m']/30],'y':[0,(max(hf['values_m'])-ground)/30],'z':[-(hf['height']/2-1)*hf['cell_size_m']/30,hf['height']*hf['cell_size_m']/60]}}
 (OUT/'model-manifest.json').write_text(json.dumps(record,indent=2),encoding='utf-8');(EVID/'terrain-model-v2-production.json').write_text(json.dumps(record,indent=2),encoding='utf-8')
 scene={'schema':'webizen.native-world/v1','authored_frame':True,'units':'display metres,1:30 source metres','up':'+Y','assets':[record['asset']],'vertex_count':16992,'triangle_count':33462,'totals':{'assets':1,'bytes':dest.stat().st_size,'vertices':16992,'triangles':33462},'camera':{'position':[60,55,74],'target':[0,2,0]},'data_source':record['title']}
 (OUT/'scene.json').write_text(json.dumps(scene,indent=2),encoding='utf-8')
 subprocess.run(['C:/github/synth/target-prod/debug/synth.exe','preview','--asset',str(dest),'--out',str(EVID/'terrain-model-v2-preview')],check=True)
 print(json.dumps({'scene':str(OUT/'scene.json'),'asset':record['asset'],'bounds':record['bounds_display']}))
finally:bridge.close()
