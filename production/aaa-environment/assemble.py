"""Native batch integration: exact Synth faces + retained UV01 + embedded PBR maps.
All transformations/import/tangent generation/sealing are performed by Synth.
"""
from build import *
import collections,shutil
URL='/production/aaa-environment/'
for d in ['groups','provenance','review','textures']:(ROOT/d).mkdir(exist_ok=True)
ATTEST=ROOT.parent/'native-world/provenance/attest.exe'

def url(p):return URL+Path(str(p).removeprefix('\\\\?\\')).relative_to(ROOT).as_posix()
def placements(bridge):
 rng=random.Random(16112026);c=[]
 def add(asset,x,z,r,s=1,yaw=0,offset=0):c.append({'asset':asset,'id':asset+'-'+str(len(c)),'x':round(x,4),'z':round(z,4),'radius':r,'scale':round(s,4),'yaw':round(yaw,4),'offset':offset})
 # Art-directed candidates. Native Vibe applies exclusions and current surface heights.
 for x in range(-68,62,9):
  for z in range(-58,68,10):
   if rng.random()<.68:add('eucalyptus-mid',x+rng.uniform(-3,3),z+rng.uniform(-3,3),5,.68+rng.random()*.42,rng.random()*math.tau)
 for x,z in [(-39,15),(-38,41),(-30,2),(-10,2)]:add('eucalyptus-near',x,z,5,.85,1.2)
 for k in range(90):
  z=rng.uniform(-45,62);x=math.sin(z*.044)*10+math.sin(z*.095)*3+(1 if k%2 else -1)*rng.uniform(4.1,6.2)
  add('bank-rock-mid' if k%4 else 'bank-rock-near',x,z,.5,rng.uniform(.25,.75),rng.random()*math.tau,-.10)
 for k in range(230):
  x=rng.uniform(-47,31);z=rng.uniform(-30,55)
  add('grass-mid' if k%6 else 'grass-near',x,z,.3,rng.uniform(.45,.95),rng.random()*math.tau,-.015)
 for k in range(25):
  x=rng.uniform(-45,28);z=rng.uniform(-29,50)
  for a in ['shrub-branches','shrub-leaves']:add(a,x,z,1,rng.uniform(.75,1.05) if a=='none' else 1,k*.7,-.02)
 for k in range(36):add('leaf-litter',rng.uniform(-39,-11),rng.uniform(5,45),.3,rng.uniform(.8,1.3),rng.random()*math.tau,-.002)
 # Native access paths and camera sight-lines are kept open, not filled indiscriminately.
 source='''pure fn ecology_clear(x:f64,z:f64,r:f64) -> bool {
 let camp=math.sqrt((x+22.0)*(x+22.0)+(z-28.0)*(z-28.0));
 if camp < 10.5+r { return false; }
 if x > -21.0-r && x < -5.0+r && z > 32.0-r && z < 55.0+r { return false; }
 if math.abs(z - 4.0) < 3.0 + r && math.abs(x - river_x(4.0)) < 10.0 + r { return false; }
 for stop in [[-17.0,6.0],[-20.0,-16.0],[20.0,-7.0],[-15.0,-32.0],[17.0,-37.0]] {
  if math.sqrt((x-stop[0])*(x-stop[0])+(z-stop[1])*(z-stop[1])) < 5.0+r { return false; }
 }
 return true;
}
pure fn ecology_placements() budget(steps:2000000) -> List {
 let candidates='''+lit(c)+''';
 let mut result=[];
 for p in candidates {
  let d=math.abs(p.x-river_x(p.z));
  if ecology_clear(p.x,p.z,p.radius) && d > 3.8+p.radius {
   result=result.push({asset:p.asset,id:p.id,translation:[p.x,terrain_height(p.x,p.z)+p.offset,p.z],rotation:[0.0,math.sin(p.yaw/2.0),0.0,math.cos(p.yaw/2.0)],scale:[p.scale,p.scale,p.scale]});
  }
 }
 return result;
}
'''
 source=source.replace('ecology_placements()', 'ecology_placements(candidates: List)').replace(' let candidates='+lit(c)+';'+chr(10),'');save(ROOT/'inputs/ecology-candidates.json',c);recipe=ROOT/'recipes/ecological-placement.vibe';recipe.write_text(source);world=(ROOT.parent.parent/'site/world/living-world.vibe').read_text(encoding='utf-8-sig').split('pure fn view(')[0];(ROOT/'inputs/fictional-world-frame.vibe').write_text(world)
 items=[];evaluations=[]
 for offset in range(0,len(c),40):
  evaluated=world+'\n'+source+'\nlet ecology=ecology_placements('+lit(c[offset:offset+40])+');';result=bridge.call('vibe_eval',{'source':evaluated});items+=result['bindings']['ecology'];evaluations.append({'source_sha256':hashlib.sha256(evaluated.encode()).hexdigest(),'input_count':len(c[offset:offset+40]),'output_count':len(result['bindings']['ecology'])})
 save(ROOT/'assets/placement-chunks.json',evaluations)
 save(ROOT/'assets/placement-evaluation.json',{'schema':'webizen.native-placement-evaluation/v1','epistemic_class':'authored-artwork','frame':'Existing fictional catchment, Y-up metres; not public Canberra DEM or private location','recipe_sha256':digest(recipe),'world_sha256':digest(ROOT/'inputs/fictional-world-frame.vibe'),'evaluator':'Native Poet/Vibe via synth vibe_eval','placements':items});return items

def glb(parts,palette,nodes,texture_map=None):
 binary=bytearray();views=[];access=[];prims=[];images=[];textures=[];materials=[];texroles=[]
 def blob(data,target=None):
  while len(binary)%4:binary.append(0)
  offset=len(binary);binary.extend(data);v={'buffer':0,'byteOffset':offset,'byteLength':len(data)}
  if target:v['target']=target
  views.append(v);return len(views)-1
 def stream(vals,fmt,typ,count,target):
  i=blob(struct.pack('<'+str(len(vals))+fmt,*vals),target);access.append({'bufferView':i,'componentType':5126 if fmt=='f' else 5125,'count':count,'type':typ});return len(access)-1
 for label,pal in palette.items():
  maps=(texture_map or {}).get(label);mat={'name':label,'doubleSided':True,'pbrMetallicRoughness':{'baseColorFactor':[1,1,1,1],'roughnessFactor':1,'metallicFactor':1 if maps else 0}}
  if maps:
   inds={}
   for role,k in enumerate(['colour','normal','orm']):
    record=maps[k];b=Path(record['path']).read_bytes();assert hashlib.sha256(b).hexdigest()==record['sha256'];vi=blob(b);images.append({'bufferView':vi,'mimeType':'image/png'});textures.append({'source':len(images)-1,'sampler':0});inds[k]=len(textures)-1
    out=ROOT/'textures'/(record['sha256']+'.png')
    if not out.exists():out.write_bytes(b)
    texroles.append({'url':url(out),'sha256':record['sha256'],'mime_type':'image/png','role':role,'cutoff':.5})
   mat['pbrMetallicRoughness'].update(baseColorTexture={'index':inds['colour']},metallicRoughnessTexture={'index':inds['orm']});mat.update(normalTexture={'index':inds['normal']},occlusionTexture={'index':inds['orm']})
  else:
   color=pal['color'].lstrip('#');srgb=[int(color[i:i+2],16)/255 for i in [0,2,4]];linear=[x/12.92 if x<=.04045 else ((x+.055)/1.055)**2.4 for x in srgb];mat['pbrMetallicRoughness'].update(baseColorFactor=linear+[1],roughnessFactor=pal['roughness'],metallicFactor=pal.get('metalness',0))
  materials.append(mat)
 labels=list(palette)
 for p in parts:
  pos=p['positions'];inds=p['indices'];pa=stream(pos,'f','VEC3',len(pos)//3,34962);ia=stream(inds,'I','SCALAR',len(inds),34963);access[pa].update(min=[min(pos[j::3]) for j in range(3)],max=[max(pos[j::3]) for j in range(3)]);attrs={'POSITION':pa}
  if p.get('uvs'):
   assert len(p['uvs'])==len(pos)//3*2;attrs['TEXCOORD_0']=stream(p['uvs'],'f','VEC2',len(pos)//3,34962)
  prims.append({'attributes':attrs,'indices':ia,'mode':4,'material':labels.index(p['material'])})
 ns=[{'name':n['id'],'mesh':n.get('mesh',0),**{k:n[k] for k in ['translation','rotation','scale']}} for n in nodes]
 meshgroups=collections.defaultdict(list)
 for part,prim in zip(parts,prims):meshgroups[part.get('_mesh',0)].append(prim)
 meshes=[{'primitives':meshgroups[i]} for i in range(max(meshgroups)+1)]
 doc={'asset':{'version':'2.0','generator':'Synth exact native faces, native retained UV01, native authored node transforms'},'buffers':[{'byteLength':len(binary)}],'bufferViews':views,'accessors':access,'materials':materials,'meshes':meshes,'nodes':ns,'scenes':[{'nodes':list(range(len(ns)))}],'scene':0}
 if images:doc.update(images=images,textures=textures,samplers=[{'wrapS':10497,'wrapT':10497,'magFilter':9729,'minFilter':9729}])
 encoded=json.dumps(doc,separators=(',',':')).encode();encoded+=b' '*((-len(encoded))%4);binary+=b'\0'*((-len(binary))%4)
 return struct.pack('<III',0x46546c67,2,28+len(encoded)+len(binary))+struct.pack('<II',len(encoded),0x4e4f534a)+encoded+struct.pack('<II',len(binary),0x004e4942)+binary,texroles

def compile_group(bridge,id,parts,pal,nodes,maps=None):
 inp=ROOT/'inputs'/(id+'.glb');b,textures=glb(parts,pal,nodes,maps);inp.write_bytes(b)
 tangent='' # Native GLB already supplies MikkTSpace corner splits.
 tanarg=''
 source='requires [capability("capability.invoke")];\nlet geometry=capability.invoke("hypermedia.manifold.import_mesh",{input:"batch.glb",format:"glb"});\n'+tangent+'let native=capability.invoke("hypermedia.manifold.compile_10d_pbr",{mesh:geometry,topology:false,spatial_index:true'+tanarg+'});\nlet mut asset={files:[{key:"'+id+'.10d",resource:native}]};'
 recipe=ROOT/'recipes'/(id+'.vibe');recipe.write_text(source);key=hashlib.sha256(source.encode()+b).hexdigest();recordpath=ROOT/'groups'/(id+'.json')
 if recordpath.exists():
  prior=json.loads(recordpath.read_text())
  if prior.get('cache_key')==key:return prior
 result=bridge.produce(source,id,[{'key':'batch.glb','kind':'glb','path':str(inp)}]);assert result['receipt']['strict_replay_verified'];native=next(r for r in result['receipt']['resources'] if r['key']==id+'.10d');out=ROOT/'assets'/(id+'.10d')
 report=json.loads(subprocess.run([str(ATTEST),native['path'],str(inp),str(out)],check=True,capture_output=True,text=True).stdout)
 nv=sum(sum(len(p['positions'])//3 for p in parts if p.get('_mesh',0)==n.get('mesh',0)) for n in nodes);nt=sum(sum(len(p['indices'])//3 for p in parts if p.get('_mesh',0)==n.get('mesh',0)) for n in nodes);assert nv <= report['vertices'] <= nt*3 and report['triangles']==nt,(report,nv,nt);source_nv=nv;nv=report['vertices']
 report.update({'source_sha256':digest(inp),'original_native_sha256':native['sha256'],'attested_sha256':digest(out),'helper_sha256':digest(ATTEST),'rights':'User-commissioned Webizen artwork; local preview/project inclusion authorised. No general reuse licence or endorsement asserted.','credential_asserted':False});save(ROOT/'provenance'/(id+'.json'),report)
 record={'id':id,'url':url(out),'sha256':digest(out),'bytes':out.stat().st_size,'vertices':nv,'source_vertices':source_nv,'native_attribute_policy':'Native GLB MikkTSpace corner splits retained; exact triangle count preserved','triangles':nt,'colour':[1,1,1,1],'instances':len(nodes),'textures':textures,'provenance_receipt':url(ROOT/'provenance'/(id+'.json')),'native':native,'production':result,'input':str(inp),'cache_key':key,'recipe':url(recipe)};save(recordpath,record);print('SEALED_GROUP',id,len(nodes),nt,flush=True);return record

def main():
 bridge=SynthBridge(ROOT)
 try:
  positions=json.loads((ROOT/'assets/placement-evaluation.json').read_text())['placements'] if (ROOT/'assets/placement-evaluation.json').exists() else placements(bridge);byasset=collections.defaultdict(list)
  for p in positions:byasset[p['asset']].append(p)
  records=[]
  for asset,nodes in byasset.items():
   if len(sys.argv)>1 and not asset.startswith(sys.argv[1]):continue
   if asset.startswith('eucalyptus'):
    target=40000 if asset.endswith('near') else 8000;data=json.loads((ROOT.parent/'native-world/lod'/('eucalyptus-'+str(target)+'.json')).read_text());parts=data['parts'];pal=data['palette'];maps=None
   else:
    path=ROOT/'assets'/(asset+'.mesh.json')
    if not path.exists():print('PENDING',asset,flush=True);continue
    data=json.loads(path.read_text());parts=data['parts'];pal=data['palette'];maps={next(iter(pal)):data['textures']}
   nv=sum(len(p['positions'])//3 for p in parts);nt=sum(len(p['indices'])//3 for p in parts);per=max(1,min(45000//nv,90000//nt,24))
   for offset in range(0,len(nodes),per):records.append(compile_group(bridge,'ecology-'+asset+'-'+str(offset//per),parts,pal,nodes[offset:offset+per],maps))
  assets=[{k:r[k] for k in ['id','url','sha256','bytes','vertices','triangles','colour','instances','provenance_receipt','recipe']} for r in records];textures={ (t['sha256'],t['role']):t for r in records for t in r['textures']}
  save(ROOT/'assets'/('integration'+('-'+sys.argv[1] if len(sys.argv)>1 else '')+'.json'),{'schema':'webizen.aaa-native-environment/v1','frame':'existing fictional catchment, Y-up metres','epistemic_class':'authored-artwork','assets':assets,'textures':list(textures.values()),'placements':url(ROOT/'assets/placement-evaluation.json'),'review_status':'native prototype views inspected; assembled review pending','triangles':sum(r['triangles'] for r in records),'vertices':sum(r['vertices'] for r in records),'instances':sum(r['instances'] for r in records)})
 finally:bridge.close()
if __name__=='__main__':main()





