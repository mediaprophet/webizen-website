"""Author controls; let Synth generate every mesh; adapt exported DXF without remodelling."""
from pathlib import Path
import math,json,sys,hashlib,subprocess,time
ROOT=Path(__file__).resolve().parent

from native_bridge import SynthBridge
TAU=math.tau
def lit(v):
 if isinstance(v,dict):return '{'+','.join(k+':'+lit(x) for k,x in v.items())+'}'
 if isinstance(v,list):return '['+','.join(lit(x) for x in v)+']'
 if isinstance(v,float):return str(round(v,6))
 return json.dumps(v)
def norm(a):
 n=math.sqrt(sum(v*v for v in a));return [v/n for v in a]
def cross(a,b):return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
def tube(path,radii,sides=12):
 rings=[]
 for i,p in enumerate(path):
  prev=path[max(0,i-1)];nxt=path[min(len(path)-1,i+1)];t=norm([nxt[j]-prev[j] for j in range(3)])
  u=norm(cross(t,[0,1,0] if abs(t[1])<.95 else [0,0,1]));v=cross(t,u);r=radii[i] if isinstance(radii,list) else radii;rx,ry=r if isinstance(r,tuple) else (r,r)
  rings.append([[p[j]+rx*math.cos(k*TAU/sides)*u[j]+ry*math.sin(k*TAU/sides)*v[j] for j in range(3)] for k in range(sides)])
 return rings
def torus(radius,y,thick=.1):return tube([[radius*math.cos(t*TAU/64),y,radius*math.sin(t*TAU/64)] for t in range(65)],thick)
def loft(name,rings,material,caps=True):return dict(name=name,op='hypermedia.manifold.loft',args=dict(sections=rings,caps=caps,interpolation='cubic',samples=2),material=material)
def turned(name,points,material):return dict(name=name,op='hypermedia.manifold.revolve',args=dict(points=[[max(.001,r),y] for r,y in points],segments=128),material=material)
def pavilion():
 parts=[turned('limestone-terraces',[[0,-.65],[9.4,-.65],[9.4,-.4],[9,-.4],[9,-.15],[8.6,-.15],[8.6,.1],[0,.1]],'limestone')]
 for k in range(20):
  path=[]
  for i in range(13):
   t=i/12;r=8.1-5.5*t+1.5*math.sin(math.pi*t);a=k*TAU/20+.37*math.sin(math.pi*t)
   path.append([r*math.cos(a),.1+7.3*math.sin(t*math.pi/2),r*math.sin(a)])
  parts.append(loft('timber-rib-'+str(k),tube(path,[(.14-.04*i/12,.23-.08*i/12) for i in range(13)]),'timber'))
 for h,r in [(7.4,2.6),(5.5,6.2)]:parts.append(loft('bronze-ring-'+str(h),torus(r,h,.09),'bronze',False))
 # Organic narrow benches around the open central court.
 for k in range(4):
  path=[[5.3*math.cos(k*TAU/4+.1+t*.85/16),.55,5.3*math.sin(k*TAU/4+.1+t*.85/16)] for t in range(17)]
  parts.append(loft('curved-seat-'+str(k),tube(path,[(.30,.13)]*17,12),'timber'))
 return parts
def vessel():
 parts=[turned('plinth',[[0,-.2],[1.45,-.2],[1.45,0],[1.25,.18],[.8,.18],[.8,.4],[0,.4]],'limestone')]
 rings=[]
 # A single flowing rim returns down the inside: a hollow fluted vessel.
 for inside in [False,True]:
  stations=range(25) if not inside else reversed(range(25))
  for i in stations:
   t=i/24;y=.4+3.8*t;r=.7+1.65*t**.75+.28*math.sin(math.pi*t)
   if inside:r-=.10
   rings.append([[(r+.085*math.sin(18*a+2*t))*math.cos(a),y+.10*t**4*math.sin(6*a),(r+.085*math.sin(18*a+2*t))*math.sin(a)] for a in [j*TAU/96 for j in range(96)]])
 parts.append(loft('fluted-tidal-shell',rings,'verdigris',False))
 for radius,y in [(1.05,1.1),(1.38,1.65),(1.72,2.2)]:parts.append(loft('resonance-ring-'+str(y),torus(radius,y,.025),'bronze',False))
 return parts
def fern():
 parts=[turned('specimen-base',[[0,-.2],[1.1,-.2],[1.1,.05],[.8,.15],[0,.15]],'limestone')]
 stem=[[.3*math.sin(i/20*math.pi),i/20*6.6,.15*math.sin(i/20*TAU)] for i in range(21)]
 parts.append(loft('living-spine',tube(stem,[.075*(1-i/22) for i in range(21)]),'bronze'))
 for i in range(10):
  t=(i+1)/12;y=.5+t*5.6;length=2.8*math.sin(math.pi*t)**.7*(1-.25*t)
  for side in [-1,1]:
   rings=[]
   for j in range(13):
    q=j/12;x=.3*math.sin(y/6.6*math.pi)+side*length*q;yy=y+.7*math.sin(q*math.pi*.8);z=.15*math.sin(y/6.6*TAU)+.25*math.sin(q*math.pi)-.35*q
    width=.32*math.sin(math.pi*q)**.8+.008
    rings.append([[x,yy+width*math.cos(k*TAU/12),z+.025*math.sin(k*TAU/12)] for k in range(12)])
   parts.append(loft('pinna-'+str(i)+'-'+str(side),rings,'leaf',True))
 return parts
PALETTE={'limestone':{'color':'#c5b795','roughness':.9,'metalness':0},'timber':{'color':'#9b6c42','roughness':.7,'metalness':0},'bronze':{'color':'#b59152','roughness':.3,'metalness':.75},'verdigris':{'color':'#3b7970','roughness':.4,'metalness':.65},'leaf':{'color':'#55755b','roughness':.6,'metalness':.08}}
def parse_dxf(path):
 lines=[line for line in path.read_text().splitlines() if line.strip()];pairs=list(zip(lines[::2],lines[1::2]));faces=[];face=None
 for code,value in pairs:
  code=code.strip()
  if code=='0':
   if face is not None:faces.append(face)
   face={} if value.strip()=='3DFACE' else None
  elif face is not None:face[code]=value
 vertices=[];indices=[];lookup={}
 for f in faces:
  for i in range(3):
   p=tuple(float(f[str(c+i)]) for c in [10,20,30])
   if p not in lookup:lookup[p]=len(vertices)//3;vertices.extend(p)
   indices.append(lookup[p])
 assert indices and len(indices)%3==0
 assert len(indices)//3==sum(1 for code,value in pairs if code.strip()=='0' and value.strip()=='3DFACE'), 'DXF face loss'
 return vertices,indices
def produce(bridge,id,title,parts):
 recipes=ROOT/'recipes';recipes.mkdir(exist_ok=True)
 lines=['requires [capability("capability.invoke")];'];files=[]
 for i,p in enumerate(parts):
  lines += [f'let m{i}=capability.invoke("{p["op"]}",{lit(p["args"])});',f'let d{i}=capability.invoke("hypermedia.manifold.project_dxf",{{mesh:m{i},layer:"{p["material"]}"}});']
  files.append(f'{{key:"{p["name"]}.dxf",resource:d{i}}}')
 lines.append('let mut asset={files:['+','.join(files)+']};');source='\n'.join(lines)
 (recipes/(id+'.vibe')).write_text(source,encoding='utf-8');start=time.time();native=bridge.produce(source,id);print('PRODUCED',id,round(time.time()-start,2),flush=True)
 resource={r['key']:r for r in native['receipt']['resources']};meshes=[];verts=[];tris=[]
 for p in parts:
  record=resource[p['name']+'.dxf'];positions,indices=parse_dxf(Path(record['path']));base=len(verts)//3;verts+=positions;tris += [v+base for v in indices]
  meshes.append(dict(name=p['name'],material=p['material'],positions=positions,indices=indices,source_sha256=record['sha256']))
 obj=['# Assembled from actual Synth DXF exports without remodelling']+['v '+' '.join(str(n) for n in verts[i:i+3]) for i in range(0,len(verts),3)]+['f '+' '.join(str(n+1) for n in tris[i:i+3]) for i in range(0,len(tris),3)]
 inputpath=recipes/(id+'.obj');inputpath.write_text('\n'.join(obj),encoding='utf-8')
 seal='requires [capability("capability.invoke")];\nlet geometry=capability.invoke("hypermedia.manifold.import_mesh",{input:"assembly.obj",format:"obj"});\nlet native=capability.invoke("hypermedia.manifold.compile_10d_pbr",{mesh:geometry});\nlet mut asset={files:[{key:"'+id+'.10d",resource:native}]};'
 (recipes/(id+'-seal.vibe')).write_text(seal,encoding='utf-8')
 sealed=bridge.produce(seal,id+'-sealed',inputs=[{'key':'assembly.obj','path':str(inputpath),'kind':'obj'}]);print('SEALED',id,flush=True)
 out=ROOT/'assets';out.mkdir(exist_ok=True)
 meshfile=out/(id+'.mesh.json');meshfile.write_text(json.dumps(dict(schema='webizen.native-mesh-adapter/v1',id=id,palette=PALETTE,parts=meshes),separators=(',',':')),encoding='utf-8')
 model=next(r for r in sealed['receipt']['resources'] if r['key'].endswith('.10d'))
 record=dict(id=id,title=title,epistemic_class='authored-artwork',geometry_origin='Synth native loft/revolve -> DXF -> lossless indexed adapter; same faces imported and sealed as .10d',parts=len(parts),vertices=len(verts)//3,triangles=len(tris)//3,mesh_json=str(meshfile),native=model,production=native,sealing=sealed,recipe=str(recipes/(id+'.vibe')),palette=PALETTE)
 (out/(id+'.manifest.json')).write_text(json.dumps(record,indent=2),encoding='utf-8')
 preview=ROOT/'previews'/(id+'-'+str(int(time.time())));preview.parent.mkdir(exist_ok=True)
 subprocess.run(['C:/github/synth/target-prod/debug/synth.exe','preview','--asset',model['path'],'--out',str(preview)],check=True)
 record['preview_dir']=str(preview);(out/(id+'.manifest.json')).write_text(json.dumps(record,indent=2),encoding='utf-8');return record
if __name__=='__main__':
 ROOT.mkdir(exist_ok=True);bridge=SynthBridge(ROOT)
 try:
  manifest=[]
  for id,title,fn in [('canopy','Canopy of inquiry',pavilion),('tidal-vessel','Tidal resonance vessel',vessel),('memory-frond','Botanical memory frond',fern)]:
   if len(sys.argv)>1 and sys.argv[1]!=id:continue
   manifest.append(produce(bridge,id,title,fn()))
  (ROOT/'assets/collection.json').write_text(json.dumps({'schema':'webizen.art-collection/v1','catalog_count':bridge.catalog.get('count'),'assets':manifest},indent=2),encoding='utf-8')
 finally:bridge.close()
