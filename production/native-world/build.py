"""Native world package: unchanged source faces + authored GLB nodes -> Synth -> .10d.

Python assembles retained input records. QualiaDB applies all affine transforms,
imports material attributes and seals geometry through native Synth morphisms.
No JavaScript is generated or used by this build.
"""
from pathlib import Path
import argparse,collections,hashlib,json,math,struct,sys,shutil,subprocess,time
HERE=Path(__file__).resolve().parent
PRODUCTION=HERE.parent
sys.path.insert(0,str(PRODUCTION))
from native_bridge import SynthBridge
from build_art import lit,parse_dxf
from woodland_lod import build_lod
URL='/production/native-world/'

def digest(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def save(path,value):Path(path).parent.mkdir(parents=True,exist_ok=True);Path(path).write_text(json.dumps(value,indent=2),encoding='utf-8')
def linear(c):return c/12.92 if c<=.04045 else ((c+.055)/1.055)**2.4
def material(p):
 c=p.get('color','#ffffff').lstrip('#');rgba=[linear(int(c[i:i+2],16)/255) for i in (0,2,4)]+[p.get('opacity',1)]
 return {'pbrMetallicRoughness':{'baseColorFactor':rgba,'metallicFactor':p.get('metalness',0),'roughnessFactor':p.get('roughness',.8)},'doubleSided':True,'alphaMode':'BLEND' if rgba[3]<1 else 'OPAQUE'}
def evaluate_authoring(bridge):
 sourcepath=PRODUCTION.parent/'site/world/living-world.vibe'
 placementpath=HERE/'scene-authoring.vibe'
 source=sourcepath.read_text(encoding='utf-8-sig')+'\n'+placementpath.read_text(encoding='utf-8-sig')+'\nlet placements = asset_placements();\nlet river_rings = river_sections();\nlet ground_y = terrain_height(-22.0,28.0);'
 (HERE/'recipes/evaluated-world.vibe').write_text(source,encoding='utf-8')
 result=bridge.call('vibe_eval',{'source':source});bindings=result['bindings']
 record={'schema':'webizen.native-world-evaluation/v1','tool':'vibe_eval','source_sha256':hashlib.sha256(source.encode()).hexdigest(),'world_source_sha256':digest(sourcepath),'placements_source_sha256':digest(placementpath),'placements':bindings['placements'],'river_rings':bindings['river_rings'],'ground_y':bindings['ground_y']}
 save(HERE/'authoring-evaluation.json',record);return record

def identity(id):return {'id':id,'translation':[0,0,0],'rotation':[0,0,0,1],'scale':[1,1,1]}

def water(bridge,rings):
 sourcepath=HERE/'recipes/river.vibe';cache=HERE/'cache/river.json'
 source='requires [capability("capability.invoke")];\nlet river=capability.invoke("hypermedia.manifold.loft",'+lit({'sections':rings,'caps':True,'interpolation':'linear'})+');\nlet exchange=capability.invoke("hypermedia.manifold.project_dxf",{mesh:river,layer:"water"});\nlet native=capability.invoke("hypermedia.manifold.compile_10d_pbr",{mesh:river});\nlet mut asset={files:[{key:"river.dxf",resource:exchange},{key:"river.10d",resource:native}]};'
 sourcepath.parent.mkdir(parents=True,exist_ok=True);sourcepath.write_text(source)
 sha=hashlib.sha256(source.encode()).hexdigest()
 prior=json.loads(cache.read_text()) if cache.exists() else {}
 if prior.get('source_sha256')==sha:return prior
 result=bridge.produce(source,'native-river');resources=result['receipt']['resources'];dxf=next(r for r in resources if r['key']=='river.dxf');pos,ind=parse_dxf(Path(dxf['path']))
 record={'source_sha256':sha,'production':result,'palette':{'water':{'color':'#528b8c','metalness':.15,'roughness':.24}},'parts':[{'name':'river','material':'water','positions':pos,'indices':ind,'source_sha256':dxf['sha256']}]};save(cache,record);return record

def collect(bridge,evaluated):
 groups=[];sources=[];placements=[];table=[];tablelookup={}
 instance_table=collections.defaultdict(list)
 for item in evaluated['placements']:instance_table[item['asset']].append({k:v for k,v in item.items() if k!='asset'})
 def add(asset,source,instances,palette_override=None):
  data=json.loads(source.read_text()) if isinstance(source,Path) else source
  if isinstance(source,Path):sources.append({'id':asset,'path':str(source.relative_to(PRODUCTION)).replace('\\','/'),'sha256':digest(source)})
  bymat=collections.defaultdict(list)
  for p in data['parts']:bymat[p['material']].append(p)
  for label,parts in bymat.items():
   mat=material((palette_override or data['palette'])[label]);key=json.dumps(mat,sort_keys=True)
   if key not in tablelookup:tablelookup[key]=len(table);table.append(mat)
   # Bound native import resources, keeping each native face unchanged.
   nv=sum(len(p['positions'])//3 for p in parts);nt=sum(len(p['indices'])//3 for p in parts)
   per=max(1,min(180000//nv,350000//nt));assert nv<=180000 and nt<=350000
   for start in range(0,len(instances),per):
    batch=instances[start:start+per];id=asset+'--'+label+(f'-{start//per+1}' if len(instances)>per else '')
    groups.append({'id':id,'asset':asset,'material':tablelookup[key],'parts':parts,'instances':batch,'vertices':nv*len(batch),'triangles':nt*len(batch)})
  placements.append({'asset':asset,'instances':instances})
 # Preserve the existing native terrain triangles; colour bands only partition faces.
 terrain=json.loads((PRODUCTION/'campsite/terrain/assets/catchment-clearing.mesh.json').read_text());p=terrain['parts'][0];bands=collections.defaultdict(list)
 for i in range(0,len(p['indices']),3):
  tri=p['indices'][i:i+3];y=sum(p['positions'][v*3+1] for v in tri)/3
  band='river-bank' if y<1 else 'grassland' if y<8 else 'upland' if y<18 else 'high-ridge';bands[band]+=tri
 terrain['parts']=[{'name':name,'material':name,'positions':p['positions'],'indices':indices} for name,indices in bands.items()]
 terrain['palette']={name:{'color':colour,'roughness':1} for name,colour in [('river-bank','#918d6a'),('grassland','#647957'),('upland','#556652'),('high-ridge','#a2aba0')]}
 add('terrain',terrain,[identity('terrain')]);sources.append({'id':'terrain','path':'campsite/terrain/assets/catchment-clearing.mesh.json','sha256':digest(PRODUCTION/'campsite/terrain/assets/catchment-clearing.mesh.json')})
 water_record=water(bridge,evaluated['river_rings'])
 camp=PRODUCTION/'campsite'
 specs=[('car-passat-b7',camp/'car/assets/car.json'),('vintage-windup-camper',camp/'camper/assets/vintage-windup-camper.mesh.json'),('solar-array-three',camp/'equipment/assets/solar-array.mesh.json'),('solar-array-two',camp/'equipment/assets/solar-array-two.mesh.json'),('starlink-mini',camp/'equipment/assets/starlink-mini.mesh.json'),('eucalyptus',camp/'woodland/assets/eucalyptus.json')]
 for asset,path in specs:
  if asset=='eucalyptus':
   near=build_lod(bridge,path,40000);far=build_lod(bridge,path,8000)
   add(asset,near,instance_table[asset]);add('eucalyptus-far',far,instance_table['eucalyptus-far'])
   sources.append({'id':'eucalyptus-original','path':str(path.relative_to(PRODUCTION)).replace('\\','/'),'sha256':digest(path),'native_lod_targets':[40000,8000]})
  else:add(asset,path,instance_table[asset])
 for id in ['canopy','tidal-vessel','memory-frond']:add(id,PRODUCTION/'assets'/f'{id}.mesh.json',instance_table[id])
 props=PRODUCTION/'native-world-props/assets'
 for id,name in [('footbridge','footbridge'),('observatory','observatory-receiver'),('biology','biological-helix')]:
  path=props/(name+'.mesh.json')
  if path.exists():add(id,path,instance_table[id])
  else:print('PENDING_PROP',id,flush=True)

 return groups,table,sources,placements,water_record

def glb(group,materials):
 binary=bytearray();views=[];accessors=[];primitives=[]
 def stream(values,fmt,type,count,target):
  while len(binary)%4:binary.append(0)
  start=len(binary);binary.extend(struct.pack('<'+str(len(values))+fmt,*values));view=len(views);views.append({'buffer':0,'byteOffset':start,'byteLength':len(binary)-start,'target':target});accessor=len(accessors);accessors.append({'bufferView':view,'componentType':5126 if fmt=='f' else 5125,'count':count,'type':type});return accessor
 for p in group['parts']:
  pos=p['positions'];ind=p['indices'];pa=stream(pos,'f','VEC3',len(pos)//3,34962);ia=stream(ind,'I','SCALAR',len(ind),34963)
  accessors[pa]['min']=[min(pos[j::3]) for j in range(3)];accessors[pa]['max']=[max(pos[j::3]) for j in range(3)]
  primitives.append({'attributes':{'POSITION':pa},'indices':ia,'material':p.get('material_index',group['material']),'mode':4})
 nodes=[{'name':n['id'],'mesh':0,**{k:n[k] for k in ['translation','rotation','scale']}} for n in group['instances']]
 doc={'asset':{'version':'2.0','generator':'Native-world authored input; exact retained Synth faces'},'buffers':[{'byteLength':len(binary)}],'bufferViews':views,'accessors':accessors,'materials':materials,'meshes':[{'primitives':primitives}],'nodes':nodes,'scenes':[{'nodes':list(range(len(nodes)))}],'scene':0}
 encoded=json.dumps(doc,separators=(',',':')).encode();encoded+=b' '*((-len(encoded))%4);binary+=b'\0'*((-len(binary))%4)
 return struct.pack('<III',0x46546c67,2,28+len(encoded)+len(binary))+struct.pack('<II',len(encoded),0x4e4f534a)+encoded+struct.pack('<II',len(binary),0x004e4942)+binary

def compile_group(bridge,group,materials):
 id=group['id'];inputpath=HERE/'inputs'/f'{id}.glb';inputpath.parent.mkdir(exist_ok=True);inputpath.write_bytes(glb(group,materials))
 description={k:group[k] for k in ['id','asset','material','instances']};description['input']='world.glb'
 source='requires [capability("capability.invoke")];\nlet world='+lit(description)+';\nlet geometry=capability.invoke("hypermedia.manifold.import_mesh",{input:world.input,format:"glb"});\nlet native=capability.invoke("hypermedia.manifold.compile_10d_pbr",{mesh:geometry});\nlet mut asset={files:[{key:"'+id+'.10d",resource:native}]};'
 recipe=HERE/'recipes'/f'{id}.vibe';recipe.write_text(source,encoding='utf-8');cache=HERE/'cache'/f'{id}.json';key=hashlib.sha256(source.encode()+inputpath.read_bytes()).hexdigest()
 prior=json.loads(cache.read_text()) if cache.exists() else {}
 if prior.get('key')==key and Path(prior['native']['path']).exists():return prior
 result=bridge.produce(source,'world-'+id,inputs=[{'key':'world.glb','kind':'glb','path':str(inputpath)}]);assert result['receipt']['strict_replay_verified'] is True
 native=next(r for r in result['receipt']['resources'] if r['key'].endswith('.10d'))
 out=HERE/'assets'/native['key'];out.parent.mkdir(exist_ok=True);shutil.copyfile(native['path'],out);assert digest(out)==native['sha256']
 record={'key':key,'id':id,'native':native,'production':result,'vertices':group['vertices'],'triangles':group['triangles'],'instances':len(group['instances']),'material':group['material'],'url':URL+'assets/'+out.name,'colour':[1,1,1,1],'sha256':digest(out),'bytes':out.stat().st_size,'recipe':URL+'recipes/'+recipe.name,'input_sha256':digest(inputpath)};save(cache,record);print('SEALED',id,record['triangles'],flush=True);return record

def main():
 parser=argparse.ArgumentParser();parser.add_argument('--only');parser.add_argument('--preview',action='store_true');parser.add_argument('--review-lods',action='store_true');parser.add_argument('--review-lod-target',type=int);args=parser.parse_args();HERE.mkdir(exist_ok=True);(HERE/'recipes').mkdir(exist_ok=True);bridge=SynthBridge(HERE)
 try:
  evaluated=evaluate_authoring(bridge)
  groups,materials,sources,placements,water_record=collect(bridge,evaluated)
  save(HERE/'scene-authoring.json',{'schema':'webizen.native-world-authoring/v1','units':'metres','up':'+Y','material_binding':'Shared ordered glTF material table imported by Synth into native MAT1; no browser-side material geometry generation','materials':materials,'placements':placements,'sources':sources,'solar_configuration':[3,3,2]})
  records=[]
  if args.review_lods or args.review_lod_target:
   preview_records=[]
   for target in ([args.review_lod_target] if args.review_lod_target else [40000,2000]):
    data=build_lod(bridge,PRODUCTION/'campsite/woodland/assets/eucalyptus.json',target)
    parts=[{**p,'material_index':materials.index(material(data['palette'][p['material']]))} for p in data['parts']]
    id='eucalyptus-lod'+str(target)
    group={'id':id,'asset':'eucalyptus','material':0,'parts':parts,'instances':[identity(id)],'vertices':sum(len(p['positions'])//3 for p in parts),'triangles':data['triangles']}
    record=compile_group(bridge,group,materials);folder=HERE/'previews'/(id+'-'+str(int(time.time())));folder.parent.mkdir(exist_ok=True)
    subprocess.run(['C:/github/synth/target-prod/debug/synth.exe','preview','--asset',str(HERE/'assets'/(id+'.10d')),'--out',str(folder)],check=True);preview_records.append({'id':id,'directory':str(folder),'triangles':data['triangles']})
   save(HERE/'lod-preview-index.json',preview_records);return
  for group in groups:
   if args.only and not group['id'].startswith(args.only):continue
   records.append(compile_group(bridge,group,materials))
  if args.only:return
  manifest={'schema':'webizen.native-world/v1','units':'metres','up':'+Y','solar_configuration':[3,3,2],'woodland_lod':{'near_target':40000,'far_target':8000,'near_instances':6,'far_instances':12,'method':'Native Synth simplify, originals preserved'},'material_binding':'Native MAT1 baseColor/roughness/metallic/doubleSided imported from the shared glTF material table. Manifest colour is a neutral multiplier.','authored_frame':True,'ground_y':evaluated['ground_y'],'camera':{'position':[-10,evaluated['ground_y']+8,47],'target':[-21,evaluated['ground_y']+1.25,29]},'assets':[{k:r[k] for k in ['id','url','colour','sha256','bytes','vertices','triangles','instances','material','recipe','input_sha256']} for r in records],'totals':{'assets':len(records),'bytes':sum(r['bytes'] for r in records),'vertices':sum(r['vertices'] for r in records),'triangles':sum(r['triangles'] for r in records),'vertex_count':sum(r['vertices'] for r in records),'triangle_count':sum(r['triangles'] for r in records),'material_definitions':len(materials)},'sources':sources}
  manifest['vertex_count']=manifest['totals']['vertex_count'];manifest['triangle_count']=manifest['totals']['triangle_count']
  wp=water_record['parts'][0]
  wi=[v for i in range(0,len(wp['indices']),3) if all(abs(wp['positions'][v*3+1]-.08)<.00001 for v in wp['indices'][i:i+3]) for v in wp['indices'][i:i+3]]
  manifest['water']={'positions':wp['positions'],'indices':wi,'source':'Native Synth loft -> exact DXF top faces','native_recipe':URL+'recipes/river.vibe','native_source_sha256':water_record['source_sha256']}
  manifest['authoring']={'vibe':URL+'scene-authoring.vibe','evaluation':URL+'authoring-evaluation.json','source_sha256':evaluated['source_sha256']}
  save(HERE/'scene.json',manifest)
  from attest import update as attach_source_provenance
  attach_source_provenance(HERE/'scene.json')
  print(json.dumps(json.loads((HERE/'scene.json').read_text())['totals']),flush=True)
  if args.preview:
   preview_records=[]
   for id in ['solar-array-three--panel','car-passat-b7--gold','terrain--grassland','eucalyptus--leaf-olive']:
    record=next((r for r in records if r['id']==id),None)
    if not record:continue
    target=HERE/'previews'/(id+'-'+str(int(time.time())));target.parent.mkdir(exist_ok=True);subprocess.run(['C:/github/synth/target-prod/debug/synth.exe','preview','--asset',str(HERE/'assets'/(id+'.10d')),'--out',str(target)],check=True);preview_records.append({'id':id,'directory':str(target)})
   save(HERE/'preview-index.json',preview_records)
 finally:bridge.close()
if __name__=='__main__':main()

