"""Author native ecology studies. Python specifies control rings, Synth owns mesh generation."""
from pathlib import Path
import sys,math,json,hashlib,time,subprocess,random,struct
ROOT=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT.parent))
from native_bridge import SynthBridge
from surface_adapter import retained_uvs
from build_art import lit,loft,tube,parse_dxf
EXE='C:/github/synth/target-prod/debug/synth.exe'
for folder in ['recipes','assets','artifacts','previews','inputs']:(ROOT/folder).mkdir(exist_ok=True)
def save(p,v):Path(p).write_text(json.dumps(v,indent=2),encoding='utf8')
def digest(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def rock(seed=19):
 rng=random.Random(seed);sides=32;rings=[]
 # Unequal bedding shelves and a displaced crown; no sphere/ellipsoid primitive.
 stations=[(-.12,.66),(.0,.93),(.12,1.03),(.25,1.02),(.32,.91),(.40,.94),(.57,.82),(.69,.77),(.73,.59),(.82,.48),(.88,.20),(.90,.025)]
 for j,(y,r) in enumerate(stations):
  ring=[]
  for k in range(sides):
   a=k*math.tau/sides;shape=1+.10*math.sin(a*3+.6)+.065*math.sin(a*7+1.2)+.025*math.sin(a*13+j*.6)
   ring.append([.11*y+r*shape*math.cos(a)*1.2,y+.035*math.sin(a*4+j*.45)*min(r,1),-.10*y+r*shape*math.sin(a)*.79])
  rings.append(ring)
 return [dict(name='weathered-bedding',op='hypermedia.manifold.loft',args={'sections':rings,'caps':True,'interpolation':'linear','samples':1},material='stone')]
def grass(seed=71,count=84):
 rng=random.Random(seed);parts=[]
 for k in range(count):
  a=k*2.399963+.12*rng.random();radius=.18*math.sqrt(rng.random());height=.32+.55*rng.random();lean=.25+.55*rng.random();base=[radius*math.cos(a),0,radius*math.sin(a)];rings=[]
  for j in range(7):
   t=j/6;spread=lean*t*t;w=(.006+.012*rng.random())*(1-t)**.7+.0005;cy=height*math.sin(t*1.7);cx=base[0]+math.cos(a)*spread;cz=base[2]+math.sin(a)*spread;dx=-math.sin(a)*w;dz=math.cos(a)*w
   rings.append([[cx-dx,cy,cz-dz],[cx,cy+.0025*(1-t),cz],[cx+dx,cy,cz+dz],[cx,cy-.001,cz]])
  parts.append(dict(name=f'arched-blade-{k:03}',op='hypermedia.manifold.loft',args={'sections':rings,'caps':True,'interpolation':'linear','samples':1},material='grass'))
 # Delicate seed culms give the tuft a non-uniform outline.
 for k in range(5):
  a=k*2.4;path=[[.03*math.cos(a)+.11*t*t,.95*t,.03*math.sin(a)+.07*t*t] for t in [0,.25,.5,.75,1]]
  parts.append(loft('seed-culm-'+str(k),tube(path,[.003,.003,.002,.0015,.0006],5),'grass'))
  for j in range(5):
   y=.69+j*.045;parts.append(loft('seed-'+str(k)+'-'+str(j),tube([[.10,y,.06],[.12,y+.025,.065],[.115,y+.04,.062]],[.001,.004,.0004],5),'grass'))
 return parts
PALETTES={'stone':{'color':'#9a937e','roughness':.94,'metalness':0,'tint':[.59,.56,.47],'seed':318,'relief':.012,'bands':[15,2],'band_strength':.22},'grass':{'color':'#778556','roughness':.86,'metalness':0,'tint':[.42,.51,.27],'seed':185,'relief':.001,'bands':[2,16],'band_strength':.16}}
def geometry(bridge,id,parts):
 if len(parts)>120:
  chunks=[geometry(bridge,id+'-source-'+str(i//120),parts[i:i+120]) for i in range(0,len(parts),120)];pos=[];inds=[];sourceparts=[]
  for g in chunks:
   off=len(pos)//3;start=len(inds);pos+=g['positions'];inds += [i+off for i in g['indices']];sourceparts += [{**p,'index_start':p['index_start']+start} for p in g['source_parts']]
  obj=ROOT/'inputs'/(id+'.obj');obj.write_text('\n'.join(['# Exact native Synth chunk faces, Y-up metres']+['v '+' '.join(map(str,pos[i:i+3])) for i in range(0,len(pos),3)]+['f '+' '.join(str(j+1) for j in inds[i:i+3]) for i in range(0,len(inds),3)]))
  recipe=ROOT/'recipes'/(id+'-sources.json');save(recipe,{'chunk_recipes':[g['recipe'] for g in chunks],'bounded_max_parts':120})
  return {'id':id,'positions':pos,'indices':inds,'source_parts':sourceparts,'production':{'chunks':[g['production'] for g in chunks],'strict_replay_verified':True},'input':str(obj),'recipe':str(recipe),'material':parts[0]['material']}
 src=['requires [capability("capability.invoke")];'];files=[]
 for i,p in enumerate(parts):
  src += [f'let m{i}=capability.invoke("{p["op"]}",{lit(p["args"])});',f'let d{i}=capability.invoke("hypermedia.manifold.project_dxf",{{mesh:m{i},layer:"{p["material"]}"}});'];files.append(f'{{key:"{p["name"]}.dxf",resource:d{i}}}')
 src.append('let mut asset={files:['+','.join(files)+']};');source='\n'.join(src);rp=ROOT/'recipes'/(id+'.vibe');rp.write_text(source);result=bridge.produce(source,id);assert result['receipt']['strict_replay_verified']
 res={r['key']:r for r in result['receipt']['resources']};pos=[];inds=[];sourceparts=[]
 for p in parts:
  r=res[p['name']+'.dxf'];pp,ii=parse_dxf(Path(r['path']));assert digest(r['path'])==r['sha256'];offset=len(pos)//3;start=len(inds);pos+=pp;inds += [i+offset for i in ii];sourceparts.append({'name':p['name'],'dxf':r['path'],'sha256':r['sha256'],'index_start':start,'index_count':len(ii)})
 obj=ROOT/'inputs'/(id+'.obj');obj.write_text('\n'.join(['# Exact native Synth faces, authored units metres Y up']+['v '+' '.join(map(str,pos[i:i+3])) for i in range(0,len(pos),3)]+['f '+' '.join(str(j+1) for j in inds[i:i+3]) for i in range(0,len(inds),3)]))
 return {'id':id,'positions':pos,'indices':inds,'source_parts':sourceparts,'production':result,'input':str(obj),'recipe':str(rp),'material':parts[0]['material']}
def seal(bridge,g):
 id=g['id'];pal=PALETTES[g['material']];maps={k:pal[k] for k in ['seed','tint','relief','bands','band_strength']};maps.update(side=256,contrast=.25,roughness=pal['roughness'],metallic=0)
 source='''requires [capability("capability.invoke")];
let geometry=capability.invoke("hypermedia.manifold.import_mesh",{input:"geometry.obj",format:"obj"});
let uv=capability.invoke("hypermedia.surface.unwrap_slim",{mesh:geometry,mode:"slim",iterations:8});
let tangent=capability.invoke("hypermedia.surface.compute_mikktspace",{mesh:geometry,uvs:uv.uvs});
let texfield=capability.invoke("hypermedia.surface.synthesize_maps",'''+lit(maps)+''');
let maps=capability.invoke("hypermedia.surface.pack_pbr_set",{colour:texfield.colour,normal:texfield.normal,orm:texfield.orm});
let bsdf=capability.invoke("hypermedia.surface.compile_bsdf_graph",{closure:"principled_v2",base_color:[1.0,1.0,1.0],roughness:1.0,metallic:0.0,coat_weight:0.0,ior:1.5});
let model=capability.invoke("hypermedia.manifold.compile_10d_pbr",{mesh:geometry,source_10d:uv.container_10d,tangents:tangent.tangents,bsdf_graph:bsdf.bsdf_json,colour_map:maps.colour,normal_map:maps.normal,orm_map:maps.orm,topology:false,spatial_index:true});
let mut asset={files:[{key:"'''+id+'''.10d",resource:model},{key:"colour.png",resource:maps.colour},{key:"normal.png",resource:maps.normal},{key:"orm.png",resource:maps.orm},{key:"uv.svg",resource:uv.uv_chart_svg},{key:"bsdf.json",resource:bsdf.bsdf_json}]};'''
 recipe=ROOT/'recipes'/(id+'-surface.vibe');recipe.write_text(source);result=bridge.produce(source,id+'-surface',[{'key':'geometry.obj','path':g['input'],'kind':'obj'}]);assert result['receipt']['strict_replay_verified'];res={r['key']:r for r in result['receipt']['resources']};native=res[id+'.10d'];path=ROOT/'assets'/(id+'.mesh.json')
 save(path,{'schema':'webizen.native-mesh-adapter/v1','id':id,'units':'metres','up':'+Y','palette':{g['material']:pal},'parts':[{'name':id,'material':g['material'],'positions':g['positions'],'indices':g['indices'],'uvs':retained_uvs(native['path'],len(g['positions'])//3),'source_parts':g['source_parts']}],'surface_native':native,'textures':{k:res[k+'.png'] for k in ['colour','normal','orm']},'warning':'Adapter preserves exact faces; consume native UV01/tangents or the attributed native input for material integration, do not discard native surface attributes.'})
 prev=ROOT/'previews'/(id+'-'+str(time.time_ns()));subprocess.run([EXE,'preview','--asset',native['path'],'--out',str(prev),'--hmc',result['receipt']['hmc']['path'],'--asset-key','outputs/'+id+'.10d'],check=True)
 rec={'id':id,'epistemic_class':'authored-artwork','description':'Native authored shape study, not species or measured geology claim','native':native,'mesh_json':str(path),'mesh_sha256':digest(path),'textures':{k:res[k+'.png'] for k in ['colour','normal','orm']},'geometry_recipe':g['recipe'],'surface_recipe':str(recipe),'production':g['production'],'surface_production':result,'preview_dir':str(prev),'triangles':len(g['indices'])//3,'vertices':len(g['positions'])//3,'bounds':{'min':[min(g['positions'][j::3]) for j in range(3)],'max':[max(g['positions'][j::3]) for j in range(3)]},'strict_replay_verified':True,'review_status':'pending actual image inspection'};save(ROOT/'assets'/(id+'.manifest.json'),rec);print('COMPLETE',id,prev,flush=True)
 return rec
if __name__=='__main__':
 bridge=SynthBridge(ROOT)
 try:
  save(ROOT/'catalog.json',bridge.catalog)
  id=sys.argv[1] if len(sys.argv)>1 else 'bank-rock';parts=rock() if id=='bank-rock' else grass()
  seal(bridge,geometry(bridge,id,parts))
 finally:bridge.close()





