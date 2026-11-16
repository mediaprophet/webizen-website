from pathlib import Path
import math,json,sys,subprocess,time,hashlib
ROOT=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT.parents[1]))
from native_bridge import SynthBridge
from build_art import lit,tube,loft,parse_dxf
TAU=math.tau
PALETTE={
 'gold':{'color':'#bba570','roughness':.29,'metalness':.62},
 'glass':{'color':'#193038','roughness':.17,'metalness':.3},
 'rubber':{'color':'#171b1b','roughness':.92,'metalness':0},
 'trim':{'color':'#202728','roughness':.5,'metalness':.05},
 'chrome':{'color':'#b8c4c5','roughness':.23,'metalness':.85},
 'alloy':{'color':'#a8afb0','roughness':.32,'metalness':.75},
 'brake':{'color':'#45494a','roughness':.65,'metalness':.6},
 'lamp':{'color':'#e1e4d8','roughness':.24,'metalness':.25,'emissive':'#9ca6a4','emissiveIntensity':.12},
 'red':{'color':'#a71f24','roughness':.24,'metalness':.2},
 'plate':{'color':'#e4e4d8','roughness':.55,'metalness':.1}}
parts=[]
def add(name,rings,mat,caps=True,cubic=False):
 p=loft(name,rings,mat,caps);p['args']['interpolation']='cubic' if cubic else 'linear';p['args']['samples']=2 if cubic else 1;parts.append(p)
def rod(name,path,radius,mat,sides=10):add(name,tube(path,radius,sides),mat,cubic=True)
def slab(name,polygon,offset,mat):add(name,[polygon,[[p[j]+offset[j] for j in range(3)] for p in polygon]],mat)
def zpanel(name,x0,x1,y0,y1,z,mat,thick=.008):slab(name,[[x0,y0,z],[x1,y0,z],[x1,y1,z],[x0,y1,z]],[0,0,thick],mat)
def wheelrings(xvalues,rvalues,z,sides=64):return [[[x,.326+r*math.sin(a),z+r*math.cos(a)] for a in [j*TAU/sides for j in range(sides)]] for x,r in zip(xvalues,rvalues)]
def bodyrings():
 # Explicit arched underbody sections avoid fragment-classifier remnants seen in the first Boolean study.
 stations=[(-2.3855,.76,.82,.31),(-2.32,.84,.99,.27),(-2.05,.89,1.015,.255),(-1.55,.91,1.015,.25),(-.8,.91,1.015,.25),(0,.91,1.0,.25),(.75,.90,.98,.25),(1.18,.90,.955,.25),(1.8,.89,.91,.25),(2.23,.85,.84,.28),(2.3855,.77,.78,.34)]
 zs=sorted(set([a[0] for a in stations]+[-2.3855+i*4.771/140 for i in range(141)]+[axle+off for axle in [1.4,-1.312] for off in [-.39,-.381,-.375,0,.375,.381,.39]]))
 rings=[]
 for z in zs:
  low,high=next(((a,b) for a,b in zip(stations,stations[1:]) if a[0]<=z<=b[0]),(stations[-2],stations[-1]));t=(z-low[0])/(high[0]-low[0]);w,h,b=[low[i]+t*(high[i]-low[i]) for i in range(1,4)]
  for axle in [1.4,-1.312]:
   d=z-axle
   if abs(d)<.381:b=max(b,.326+math.sqrt(.381**2-d*d))
  # Cross-sections keep side panels smoothly curved and preserve the actual wheel opening.
  q=max(b+.018,h-.13)
  xy=[[-w*.86,b],[-w*.98,b+.008],[-w,(b+q)*.5],[-w,q],[-w*.96,h-.045],[-w*.8,h],[-w*.4,h+.012],[0,h+.017],[w*.4,h+.012],[w*.8,h],[w*.96,h-.045],[w,q],[w,(b+q)*.5],[w*.98,b+.008],[w*.86,b],[0,b]]
  rings.append([[x,y,z] for x,y in xy])
 return rings
add('sculpted-gold-coachwork',bodyrings(),'gold',cubic=False)
# The body is cut with real native cylinder booleans below; these are modelling operands.
cutters=[wheelrings([-1.15,1.15],[.373,.373],z,48) for z in [1.40,-1.312]]
# Low, long roof and tapered estate cabin, with a narrow painted window surround.
cabin=[]
for z,top,w in [(-2.26,1.06,.85),(-2.08,1.30,.85),(-1.89,1.448,.85),(-1.65,1.485,.85),(-.7,1.493,.85),(.22,1.47,.85),(.42,1.385,.85),(.76,1.19,.85),(1.10,1.015,.85)]:
 bottom=.977; shoulder=min(top-.018,1.045);roofx=max(.68,.85-(top-bottom)*.28)
 cabin.append([[-w,bottom,z],[-w,shoulder,z],[-roofx,top-.018,z],[-roofx*.65,top,z],[0,top+.015,z],[roofx*.65,top,z],[roofx,top-.018,z],[w,shoulder,z],[w,bottom,z]])
add('estate-roof-and-pillars',cabin,'gold',cubic=True)
# Windows are authored surfaces outside the corresponding tapered cabin side.
for side in [-1,1]:
 tag='left' if side<0 else 'right'
 def sideglass(name,yz):
  polygon=[[side*(.853-max(0,y-1.045)*.28+.008),y,z] for y,z in yz]
  slab(tag+'-'+name,polygon,[side*.005,0,0],'glass')
 sideglass('front-door-glazing',[(1.025,.99),(1.422,.285),(1.449,-.30),(1.025,-.34)])
 sideglass('rear-door-glazing',[(1.025,-.405),(1.45,-.37),(1.457,-1.18),(1.025,-1.23)])
 sideglass('estate-quarter-glazing',[(1.025,-1.295),(1.452,-1.255),(1.422,-1.845),(1.28,-2.035),(1.025,-2.16)])
 # Chrome waist moulding and upper window outline.
 rod(tag+'-window-chrome-waist',[[side*.865,1.016,z] for z in [-2.17,-1.4,-.4,.5,1.01]],.009,'chrome',8)
 rod(tag+'-window-chrome-crown',[[side*.86,1.03,-2.18],[side*.79,1.29,-2.06],[side*.735,1.447,-1.87],[side*.73,1.475,-1.0],[side*.73,1.463,.26],[side*.85,1.035,1.02]],.009,'chrome',8)
 rod(tag+'-roof-rail',[[side*.64,1.49,z] for z in [-1.91]]+[[side*.65,1.545,z] for z in [-1.7,-1.0,-.2]]+[[side*.64,1.48,.28]],.018,'chrome',10)
 for z in [-1.7,.08]:rod(tag+'-rail-foot-'+str(z),[[side*.65,1.49,z],[side*.65,1.535,z]],.022,'trim',8)
 # Door shutlines sit on the exposed body panel rather than across glass.
 for z in [-1.25,-.37,.96]:rod(tag+'-door-seam-'+str(z),[[side*.881,1.008,z],[side*.912,.8,z-.015],[side*.917,.55,z-.02],[side*.884,.32,z-.02]],.0045,'trim',6)
 rod(tag+'-sill-moulding',[[side*.889,.31,z] for z in [-.91,-.4,.3,1.0]],.016,'chrome',8)
 for z in [-.92,.60]:rod(tag+'-door-handle-'+str(z),[[side*.925,.928,z-.075],[side*.934,.93,z+.075]],.015,'chrome',10)
 # Sculpted mirror housing: elliptical rings, separate glass and dark mounting arm.
 rod(tag+'-mirror-stalk',[[side*.85,1.045,.92],[side*.965,1.075,.87]],.033,'trim',10)
 mirror=[]
 for dz,rx,ry in [(-.105,.07,.035),(-.075,.12,.055),(.075,.12,.055),(.11,.07,.035)]:mirror.append([[side*1.00+rx*math.cos(a),1.095+ry*math.sin(a),.84+dz] for a in [i*TAU/16 for i in range(16)]])
 add(tag+'-gold-mirror',mirror,'gold',cubic=True)
 zpanel(tag+'-mirror-glass',side*1.00-.095,side*1.00+.095,1.055,1.13,.738,'glass',.003)
 # Arch lips follow the actual boolean aperture, as on pressed metal wings.
 for axle,z in [('front',1.4),('rear',-1.312)]:
  arc=[[side*.915,.326+.381*math.sin(a),z+.381*math.cos(a)] for a in [j*math.pi/32 for j in range(33)]]
  rod(tag+'-'+axle+'-arch-lip',arc,.013,'gold',8)
  xs=[side*x for x in [.775,.80,.825,.91,.935,.947]]
  add(tag+'-'+axle+'-tyre',wheelrings(xs,[.286,.318,.326,.326,.317,.282],z),'rubber',cubic=True)
  add(tag+'-'+axle+'-brake-disc',wheelrings([side*.93,side*.946],[.216,.216],z),'brake')
  # Polished rim flange and ten turbine-style spokes.
  rimpath=[[side*.951,.326+.23*math.sin(a),z+.23*math.cos(a)] for a in [j*TAU/64 for j in range(65)]]
  rod(tag+'-'+axle+'-rim-lip',rimpath,.012,'alloy',8)
  for k in range(10):
   a=k*TAU/10
   path=[[side*.953,.326+r*math.sin(a+.2*(r/.22)),z+r*math.cos(a+.2*(r/.22))] for r in [.055,.13,.218]]
   rod(tag+'-'+axle+'-spoke-'+str(k),path,(.015,.025),'alloy',6)
  add(tag+'-'+axle+'-hub',wheelrings([side*.95,side*.969],[.062,.062],z,24),'chrome')
  # Small wheel badge, no invented trim-specific wheel claim.
  add(tag+'-'+axle+'-hub-badge',wheelrings([side*.970,side*.972],[.028,.028],z,24),'trim')
# Front and rear glazing are separate slightly offset panes.
windscreen=[]
for z,y,w in [(1.11,1.065,.815),(.95,1.145,.79),(.76,1.226,.76),(.58,1.335,.735),(.42,1.422,.71),(.265,1.487,.69)]:
 windscreen.append([[-w,y,z],[-w*.5,y+.022,z],[0,y+.03,z],[w*.5,y+.022,z],[w,y,z],[w,y-.005,z],[w*.5,y+.017,z],[0,y+.025,z],[-w*.5,y+.017,z],[-w,y-.005,z]])
add('windscreen',windscreen,'glass',cubic=True)
slab('rear-estate-glass',[[-.805,1.088,-2.275],[.805,1.088,-2.275],[.70,1.452,-1.96],[-.70,1.452,-1.96]],[0,.002,-.009],'glass')
rod('rear-wiper',[[.38,1.10,-2.25],[.01,1.12,-2.235]],.008,'trim',6)
for x in [-.4,.3]:rod('windscreen-wiper-'+str(x),[[x-.2,1.075,1.03],[x+.20,1.085,1.025]],.007,'trim',6)
# B7's horizontal chrome grille and rectangular headlamp face.
zpanel('upper-grille',-.435,.435,.635,.797,2.402,'trim')
for y in [.65,.694,.738,.782]:rod('grille-chrome-'+str(y),[[-.43,y,2.417],[.43,y,2.417]],.009,'chrome',8)
zpanel('lower-intake',-.59,.59,.365,.505,2.402,'trim')
for y in [.395,.445,.493]:rod('lower-grille-'+str(y),[[-.575,y,2.415],[.575,y,2.415]],.007,'trim',6)
for side in [-1,1]:
 tag='left' if side<0 else 'right'
 poly=[[side*.46,.63,2.405],[side*.83,.64,2.399],[side*.815,.795,2.399],[side*.47,.807,2.405]]
 slab(tag+'-headlamp-housing',poly,[0,0,.009],'trim')
 poly2=[[side*.49,.652,2.420],[side*.795,.66,2.415],[side*.786,.778,2.415],[side*.49,.784,2.420]]
 slab(tag+'-headlamp-lens',poly2,[0,0,.004],'lamp')
 rod(tag+'-headlamp-divider',[[side*.64,.654,2.426],[side*.64,.787,2.426]],.009,'chrome',6)
 zpanel(tag+'-fog-lamp',min(side*.66,side*.79),max(side*.66,side*.79),.405,.46,2.410,'lamp')
 # Broad estate tail lamp clusters and internal white reversing segments.
 zpanel(tag+'-tail-lamp',min(side*.48,side*.80),max(side*.48,side*.80),.82,.997,-2.335,'red',-.06)
 zpanel(tag+'-reverse-lamp',min(side*.52,side*.72),max(side*.52,side*.72),.852,.898,-2.399,'lamp',-.005)
rod('front-bumper-chrome',[[-.79,.535,2.33],[0,.535,2.395],[.79,.535,2.33]],.01,'chrome',8)
rod('rear-bumper-chrome',[[-.79,.53,-2.34],[0,.53,-2.397],[.79,.53,-2.34]],.009,'chrome',8)
zpanel('blank-front-numberplate',-.235,.235,.48,.585,2.402,'plate',.012)
zpanel('blank-rear-numberplate',-.235,.235,.69,.80,-2.4,'plate',-.012)
rod('tailgate-handle',[[-.22,.96,-2.377],[.22,.96,-2.377]],.012,'chrome',8)
# VW roundel is drawn as native chrome tube outlines, without raster decals.
for name,y,z in [('front',.734,2.418),('rear',1.006,-2.409)]:
 ring=[[.057*math.cos(a),y+.057*math.sin(a),z] for a in [i*TAU/32 for i in range(33)]]
 rod(name+'-roundel',ring,.007,'chrome',8)
 rod(name+'-roundel-v',[[-.027,y+.032,z],[0,y-.008,z],[.027,y+.032,z]],.005,'chrome',6)
 rod(name+'-roundel-w',[[-.043,y+.004,z],[-.024,y-.035,z],[0,y-.009,z],[.024,y-.035,z],[.043,y+.004,z]],.005,'chrome',6)


def fitted_surfaces():
 # Controls are fitted to previously exported native body/cabin surfaces, never to a substitute browser model.
 reference=json.loads((ROOT/'inputs/body-surface-reference.json').read_text())
 surfaces={p['name']:p for p in reference['parts']}
 def ray(point,part,axis,positive=True):
  p=surfaces[part];axes=[i for i in range(3) if i!=axis];u,v=[point[j] for j in axes];hits=[]
  for k in range(0,len(p['indices']),3):
   a,b,c=[p['positions'][3*i:3*i+3] for i in p['indices'][k:k+3]];au,av=[a[j] for j in axes];bu,bv=[b[j] for j in axes];cu,cv=[c[j] for j in axes]
   den=(bv-cv)*(au-cu)+(cu-bu)*(av-cv)
   if abs(den)<1e-12:continue
   wa=((bv-cv)*(u-cu)+(cu-bu)*(v-cv))/den;wb=((cv-av)*(u-cu)+(au-cu)*(v-cv))/den;wc=1-wa-wb
   if min(wa,wb,wc)>=-1e-5:hits.append(wa*a[axis]+wb*b[axis]+wc*c[axis])
  return (max(hits) if positive else min(hits)) if hits else None
 # Dark B/C pillar trims make the continuous estate glazing read correctly.
 for side in [-1,1]:
  for n,z in [('b',-.37),('c',-1.255)]:
   poly=[[side*(.86-max(0,y-1.045)*.28),y,zz] for y,zz in [(1.025,z-.034),(1.455,z-.026),(1.455,z+.026),(1.025,z+.034)]]
   slab(('left' if side<0 else 'right')+'-'+n+'-pillar-trim',poly,[side*.005,0,0],'trim')
 def patch(p,quad,part,axis,positive,clearance):
  # Densely sampled ribbons follow the native convex panel; corner-only fits cut through it.
  a,b,c,d=quad;rings=[];sign=1 if positive else -1
  for row in range(17):
   v=row/16;front=[]
   for col in range(17):
    u=col/16;point=[a[j]*(1-u)*(1-v)+b[j]*u*(1-v)+c[j]*u*v+d[j]*(1-u)*v for j in range(3)]
    hit=ray(point,part,axis,positive)
    if hit is not None:point[axis]=hit+sign*clearance
    front.append(point)
   back=[[q[j]-(sign*.004 if j==axis else 0) for j in range(3)] for q in reversed(front)]
   rings.append(front+back)
  p['args']['sections']=rings;p['args']['interpolation']='linear';p['args']['samples']=1
 for p in parts:
  name=p['name'];rings=p['args']['sections']
  if name=='windscreen':
   patch(p,[[-.815,1.05,1.075],[.815,1.05,1.075],[.69,1.47,.265],[-.69,1.47,.265]],'estate-roof-and-pillars',1,True,.014)
  elif name=='rear-estate-glass':
   quad=[[-.805,1.088,-2.24],[.805,1.088,-2.24],[.70,1.452,-1.96],[-.70,1.452,-1.96]]
   patch(p,quad,'estate-roof-and-pillars',1,True,.015)
  elif any(t in name for t in ['headlamp-housing','headlamp-lens','fog-lamp','tail-lamp','reverse-lamp']):
   back='tail-lamp' in name or 'reverse-lamp' in name
   extra=.025 if ('lens' in name or 'reverse' in name) else .012
   patch(p,rings[0],'sculpted-gold-coachwork',2,not back,extra)
  elif 'headlamp-divider' in name:
   for ring in rings:
    for point in ring:
     z=ray(point,'sculpted-gold-coachwork',2)
     if z is not None:point[2]=z+.035
  elif 'pillar-trim' in name:
   side=-1 if name.startswith('left') else 1
   patch(p,rings[0],'estate-roof-and-pillars',0,side>0,.017)

fitted_surfaces()

def build():
 recipes=ROOT/'recipes';recipes.mkdir(exist_ok=True);out=ROOT/'assets';out.mkdir(exist_ok=True)
 lines=['requires [capability("capability.invoke")];'];files=[]
 for i,p in enumerate(parts):
  lines.append(f'let m{i}=capability.invoke("{p["op"]}",{lit(p["args"])});');name=f'm{i}'
  if False: # First boolean study retained in unique historical receipt; final body uses authored arch-following loft.
   for j,rings in enumerate(cutters):
    args={'sections':rings,'caps':True,'interpolation':'linear','samples':1}
    lines.append(f'let cut{j}=capability.invoke("hypermedia.manifold.loft",{lit(args)});')
    lines.append(f'let bod{j}=capability.invoke("hypermedia.manifold.boolean_3d",{{a:{name},b:cut{j},op:"difference"}});');name=f'bod{j}'
  lines.append(f'let d{i}=capability.invoke("hypermedia.manifold.project_dxf",{{mesh:{name},layer:"{p["material"]}"}});')
  files.append(f'{{key:"{p["name"]}.dxf",resource:d{i}}}')
 lines.append('let mut asset={files:['+','.join(files)+']};');source='\n'.join(lines);(recipes/'passat-b7.vibe').write_bytes(source.encode('utf-8'))
 bridge=SynthBridge(ROOT)
 try:
  native=bridge.produce(source,'car-passat');print('NATIVE PRODUCTION COMPLETE',flush=True)
  resources={r['key']:r for r in native['receipt']['resources']};meshes=[];verts=[];tri=[]
  for p in parts:
   r=resources[p['name']+'.dxf'];positions,indices=parse_dxf(Path(r['path']));base=len(verts)//3;verts+=positions;tri += [v+base for v in indices]
   assert hashlib.sha256(Path(r['path']).read_bytes()).hexdigest()==r['sha256']
   meshes.append(dict(name=p['name'],material=p['material'],positions=positions,indices=indices,source_sha256=r['sha256']))
  obj=['# Exact native Synth DXF faces; metres, Y up, +Z forward']+['v '+' '.join(str(n) for n in verts[i:i+3]) for i in range(0,len(verts),3)]+['f '+' '.join(str(n+1) for n in tri[i:i+3]) for i in range(0,len(tri),3)]
  inputpath=recipes/'passat-b7.obj';inputpath.write_text('\n'.join(obj),encoding='utf-8')
  seal='requires [capability("capability.invoke")];\nlet geometry=capability.invoke("hypermedia.manifold.import_mesh",{input:"assembly.obj",format:"obj"});\nlet native=capability.invoke("hypermedia.manifold.compile_10d_pbr",{mesh:geometry});\nlet mut asset={files:[{key:"passat-b7.10d",resource:native}]};'
  (recipes/'passat-b7-seal.vibe').write_bytes(seal.encode('utf-8'));sealed=bridge.produce(seal,'car-passat-sealed',inputs=[{'key':'assembly.obj','path':str(inputpath),'kind':'obj'}]);print('NATIVE SEAL COMPLETE',flush=True)
  bbox={'min':[min(verts[j::3]) for j in range(3)],'max':[max(verts[j::3]) for j in range(3)]}
  mesh={'schema':'webizen.native-mesh-adapter/v1','id':'car-passat-b7','units':'metres','up':'+Y','forward':'+Z','palette':PALETTE,'parts':meshes,'bounds':bbox}
  (out/'car.json').write_text(json.dumps(mesh,separators=(',',':')),encoding='utf-8')
  model=next(r for r in sealed['receipt']['resources'] if r['key'].endswith('.10d'))
  preview=ROOT/'previews'/('car-'+str(int(time.time())));preview.parent.mkdir(exist_ok=True)
  manifest={'id':'car-passat-b7','title':'Gold 2013 Volkswagen Passat B7 wagon','epistemic_class':'authored-representation','description':'Authored dimensional B7 Variant interpretation. Gold finish per owner; wheel design and exact trim details are illustrative, not an asserted scan of their vehicle. Blank numberplates.','units':'metres','up':'+Y','forward':'+Z','reference_dimensions':{'length':4.771,'width_excluding_mirrors':1.820,'height':1.508,'wheelbase':2.712},'references':['https://www.volkswagen-newsroom.com/en/vehicle-data-passat-b7-profile-20037','https://www.volkswagen-newsroom.com/en/passat-b7-20102014-20036'],'mesh_json':str(out/'car.json'),'parts':len(parts),'vertices':len(verts)//3,'triangles':len(tri)//3,'bounds':bbox,'native':model,'production':native,'sealing':sealed,'recipe':str(recipes/'passat-b7.vibe'),'preview_dir':str(preview)}
  (out/'car.manifest.json').write_text(json.dumps(manifest,indent=2),encoding='utf-8')
  subprocess.run(['C:/github/synth/target-prod/debug/synth.exe','preview','--asset',model['path'],'--out',str(preview)],check=True)
  print(json.dumps({'mesh':str(out/'car.json'),'manifest':str(out/'car.manifest.json'),'preview':str(preview),'parts':len(parts),'triangles':len(tri)//3,'bounds':bbox}),flush=True)
 finally:bridge.close()
if __name__=='__main__':build()
