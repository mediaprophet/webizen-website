from pathlib import Path
import sys,math,random,json,hashlib,time,subprocess
ROOT=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT.parents[1]))
from woodland_bridge import SynthBridge
from build_art import lit,tube,loft,parse_dxf
rng=random.Random(201311);TAU=math.tau
PAL={'pale-bark':{'color':'#bfc4b2','roughness':.91,'metalness':0},'warm-bark':{'color':'#d0c0a6','roughness':.92,'metalness':0},'old-bark':{'color':'#716c57','roughness':.98,'metalness':0},'twig':{'color':'#80795c','roughness':.95,'metalness':0},'leaf-olive':{'color':'#4a684d','roughness':.82,'metalness':0},'leaf-blue':{'color':'#54786d','roughness':.79,'metalness':0},'leaf-young':{'color':'#7b9063','roughness':.83,'metalness':0}}
parts=[];leafsets=[[] for _ in range(16)];leafscales=[[] for _ in range(16)]
def centre(y):return [.055*y+.22*math.sin(y*.62),y,.08*y+.12*math.sin(y*.8)]
def branch(name,path,radii,mat,sides=9):
 p=loft(name,tube(path,radii,sides),mat);p['args']['samples']=3;parts.append(p)
def lerp(a,b,t):return [a[i]*(1-t)+b[i]*t for i in range(3)]
def onpath(path,t):
 q=t*(len(path)-1);i=min(int(q),len(path)-2);return lerp(path[i],path[i+1],q-i)
# A modestly leaning open-grown trunk, with a long clean lower bole.
trunk=[centre(y) for y in [0,.3,1,2,3,4,5,6,7,8,9,10,11,12.1,13.1]]
branch('sinuous-main-bole',trunk,[.44,.41,.34,.30,.27,.24,.21,.19,.17,.145,.12,.09,.06,.032,.006],'pale-bark',16)
for k in range(6):
 a=k*TAU/6+.1;branch('root-flare-'+str(k),[[.26*math.cos(a),.24,.26*math.sin(a)],[.6*math.cos(a),.11,.6*math.sin(a)],[.96*math.cos(a),.015,.96*math.sin(a)]],[.12,.07,.01],'old-bark',9)
# Narrow strips of retained bark, broken irregularly around the lower bole.
for k in range(9):
 a=k*TAU/9;top=1.2+rng.random()*2.7;path=[]
 for j in range(7):
  y=.08+top*j/6;c=centre(y);r=.42-(.42-.27)*min(y/3,1);aa=a+.09*math.sin(j*.7+k);path.append([c[0]+r*math.cos(aa),y,c[2]+r*math.sin(aa)])
 branch('peeling-bark-'+str(k),path,[(.015,.045+(.025*math.sin(j))) for j in range(7)],'old-bark',6)
# Irregular skeletal crown. Every foliage spray grows along an actual articulated twig.
for k in range(8):
 a=k*2.39996+.18;startheight=4.1+(k%4)*.86;start=centre(startheight);extent=3.0+1.35*rng.random();tip=[start[0]+math.cos(a)*extent,10.3+3*rng.random(),start[2]+math.sin(a)*extent];path=[]
 for j in range(9):
  t=j/8;p=lerp(start,tip,t);p[0]+=.35*math.sin(math.pi*t)*math.sin(a+1);p[2]+=.42*math.sin(math.pi*t)*math.cos(a+.5);p[1]+=.55*math.sin(math.pi*t);path.append(p)
 branch('scaffold-limb-'+str(k),path,[.18*(1-j/8)**1.25+.008 for j in range(9)],'warm-bark' if k%3==0 else 'pale-bark',11)
 for j in range(5):
  t=.40+j*.145;origin=onpath(path,t);az=a+(-1 if j%2 else 1)*(.6+rng.random()*.8);span=1.1+rng.random()*1.45;end=[origin[0]+span*math.cos(az),origin[1]+.8+rng.random()*.9,origin[2]+span*math.sin(az)];secondary=[]
  for q in range(5):
   u=q/4;p=lerp(origin,end,u);p[1]+=.35*math.sin(math.pi*u);secondary.append(p)
  branch('secondary-'+str(k)+'-'+str(j),secondary,[.055,.045,.032,.019,.004],'pale-bark',8)
  for tw in range(3):
   point=onpath(secondary,.5+tw*.23);az2=az+(tw-1)*1.0;span2=.7+rng.random()*.6;twig=[]
   for q in range(5):
    u=q/4;twig.append([point[0]+span2*math.cos(az2)*u,point[1]+.27*math.sin(math.pi*u)-.3*u,point[2]+span2*math.sin(az2)*u])
   branch('terminal-twig-'+str(k)+'-'+str(j)+'-'+str(tw),twig,[.018,.014,.010,.006,.0015],'twig',6)
   for node in range(8):
    t2=.10+node*.12;pos=onpath(twig,t2)
    for side in [-1,1]:
     angle=az2+side*(.7+.2*rng.random());binid=int((angle%TAU)/TAU*16)%16
     offset=.025+rng.random()*.045;leafsets[binid].append([pos[0]+offset*math.cos(angle),pos[1]+.02*rng.random(),pos[2]+offset*math.sin(angle)])
     scale=.68+rng.random()*.72;leafscales[binid].append([scale,scale,scale])
# Long narrow pendulous lanceolate leaves: slightly folded faces, varying azimuths, open air between sprays.
for k,points in enumerate(leafsets):
 angle=k*TAU/16;u=[math.cos(angle),0,math.sin(angle)];w=[-math.sin(angle),0,math.cos(angle)];rings=[]
 for t,width in [(0,.002),(.18,.032),(.43,.042),(.72,.023),(1,.0008)]:
  p=[u[0]*(.09*math.sin(math.pi*t)+.13*t),-.38*t,u[2]*(.09*math.sin(math.pi*t)+.13*t)];fold=.007*math.sin(math.pi*t)
  rings.append([[p[0]-w[0]*width,p[1],p[2]-w[2]*width],[p[0],p[1]+fold,p[2]],[p[0]+w[0]*width,p[1],p[2]+w[2]*width],[p[0],p[1]-fold-.001,p[2]]])
 p=loft('pendulous-leaf-spray-'+str(k),rings,['leaf-olive','leaf-blue','leaf-young'][k%3]);p['args']['samples']=1;p['args']['interpolation']='linear';p['points']=points;p['scales']=leafscales[k];parts.append(p)

def build():
 recipes=ROOT/'recipes';recipes.mkdir(exist_ok=True);out=ROOT/'assets';out.mkdir(exist_ok=True)
 lines=['requires [capability("capability.invoke")];'];files=[]
 for i,p in enumerate(parts):
  lines.append(f'let m{i}=capability.invoke("{p["op"]}",{lit(p["args"])});');name=f'm{i}'
  if 'points' in p:
   lines.append(f'let ins{i}=capability.invoke("hypermedia.manifold.instance_on_points",{{prototype:m{i},points:{lit(p["points"])},scales:{lit(p["scales"])}}});')
   lines.append(f'let real{i}=capability.invoke("hypermedia.manifold.realize_instances",{{instances:ins{i}}});');name=f'real{i}'
  lines.append(f'let d{i}=capability.invoke("hypermedia.manifold.project_dxf",{{mesh:{name},layer:"{p["material"]}"}});');files.append(f'{{key:"{p["name"]}.dxf",resource:d{i}}}')
 lines.append('let mut asset={files:['+','.join(files)+']};');source='\n'.join(lines);(recipes/'eucalyptus.vibe').write_bytes(source.encode())
 bridge=SynthBridge(ROOT)
 try:
  native={'receipt':json.loads((ROOT/'artifacts/eucalyptus-130269c645fa.json').read_text())} if '--resume' in sys.argv else bridge.produce(source,'eucalyptus');print('PRODUCED',flush=True);resources={r['key']:r for r in native['receipt']['resources']};batches={};verts=[];tris=[];checks=[]
  for p in parts:
   r=resources[p['name']+'.dxf'];positions,indices=parse_dxf(Path(r['path']));assert hashlib.sha256(Path(r['path']).read_bytes()).hexdigest()==r['sha256'];base=len(verts)//3;verts+=positions;tris += [v+base for v in indices]
   batch=batches.setdefault(p['material'],{'name':p['material'],'material':p['material'],'positions':[],'indices':[],'source_parts':[]});offset=len(batch['positions'])//3;start=len(batch['indices']);batch['positions']+=positions;batch['indices'] += [v+offset for v in indices];batch['source_parts'].append({'name':p['name'],'source_sha256':r['sha256'],'index_start':start,'index_count':len(indices),'vertex_start':offset,'vertex_count':len(positions)//3});checks.append({'name':p['name'],'triangles':len(indices)//3,'sha256':r['sha256']})
  obj=['# Exact Synth exported DXF faces, Y up, metres']+['v '+' '.join(str(x) for x in verts[i:i+3]) for i in range(0,len(verts),3)]+['f '+' '.join(str(x+1) for x in tris[i:i+3]) for i in range(0,len(tris),3)];inputpath=recipes/'eucalyptus.obj';inputpath.write_text('\n'.join(obj))
  seal='requires [capability("capability.invoke")];\nlet geometry=capability.invoke("hypermedia.manifold.import_mesh",{input:"assembly.obj",format:"obj"});\nlet native=capability.invoke("hypermedia.manifold.compile_10d_pbr",{mesh:geometry,topology:false,spatial_index:false});\nlet mut asset={files:[{key:"eucalyptus.10d",resource:native}]};';(recipes/'eucalyptus-seal.vibe').write_bytes(seal.encode());sealed=bridge.produce(seal,'eucalyptus-sealed',[{'key':'assembly.obj','path':str(inputpath),'kind':'obj'}]);print('SEALED',flush=True)
  bounds={'min':[min(verts[j::3]) for j in range(3)],'max':[max(verts[j::3]) for j in range(3)]};mesh={'schema':'webizen.native-mesh-adapter/v1','id':'eucalyptus','units':'metres','up':'+Y','palette':PAL,'parts':list(batches.values()),'bounds':bounds};meshpath=out/'eucalyptus.json';meshpath.write_text(json.dumps(mesh,separators=(',',':')))
  model=next(r for r in sealed['receipt']['resources'] if r['key'].endswith('.10d'))
  def url(p):return '/production/campsite/woodland/'+Path(str(p).removeprefix('\\\\?\\')).relative_to(ROOT).as_posix()
  preview=ROOT/'previews'/('eucalyptus-'+str(int(time.time())));preview.parent.mkdir(exist_ok=True)
  record={'schema':'webizen.campsite-native-asset/v1','id':'eucalyptus','title':'Open-grown eucalyptus','description':'Authored Australian woodland eucalyptus study: articulated pale limbs and individually realised pendulous lanceolate leaves. No species-exact botanical claim.','role':'woodland','units':'metres','up':'+Y','ground_y':0,'mesh_url':url(meshpath),'model_url':url(model['path']),'recipe_url':url(recipes/'eucalyptus.vibe'),'receipt_url':url(Path(str(sealed['receipt']['hmc']['path']).removeprefix('\\\\?\\')).with_suffix('.json')),'production_receipt_url':url(Path(str(native['receipt']['hmc']['path']).removeprefix('\\\\?\\')).with_suffix('.json')),'package_url':url(sealed['receipt']['hmc']['path']),'model_sha256':model['sha256'],'mesh_sha256':hashlib.sha256(meshpath.read_bytes()).hexdigest(),'parts':len(batches),'source_parts':len(parts),'triangles':len(tris)//3,'leaf_count':sum(len(x) for x in leafsets),'bounds':bounds,'preview_dir':str(preview),'strict_replay_verified':native['receipt']['strict_replay_verified'] and sealed['receipt']['strict_replay_verified'],'epistemic_class':'authored-artwork'}
  (out/'eucalyptus.integration.json').write_text(json.dumps(record,indent=2));(out/'eucalyptus.manifest.json').write_text(json.dumps({'integration':record,'production':native,'sealing':sealed,'face_checks':checks},indent=2));subprocess.run(['C:/github/synth/target-prod/debug/synth.exe','preview','--asset',model['path'],'--out',str(preview)],check=True);print(json.dumps(record,indent=2),flush=True)
 finally:bridge.close()
if __name__=='__main__':build()
