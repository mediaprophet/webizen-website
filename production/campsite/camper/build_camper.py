"""An authored 1980-era wind-up camper: every surface is produced by Synth lofts."""
from pathlib import Path
import sys,math,json
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parents[1]))
import build_art as native
from native_bridge import SynthBridge
native.ROOT=HERE
native.PALETTE={
 'enamel':{'color':'#e4dbc1','roughness':.68,'metalness':.08},
 'roof':{'color':'#eee6d2','roughness':.62,'metalness':.08},
 'canvas':{'color':'#c5b88e','roughness':1,'metalness':0},
 'canvas-roof':{'color':'#d6caa7','roughness':1,'metalness':0},
 'screen':{'color':'#3c4b3e','roughness':1,'metalness':0},
 'screen-wire':{'color':'#87927b','roughness':1,'metalness':0},
 'canvas-seam':{'color':'#8b805e','roughness':1,'metalness':0},
 'ochre':{'color':'#bc903a','roughness':.75,'metalness':.02},
 'olive':{'color':'#586a3c','roughness':.75,'metalness':.02},
 'rubber':{'color':'#252824','roughness':.95,'metalness':0},
 'steel':{'color':'#737b77','roughness':.55,'metalness':.65},
 'chassis':{'color':'#404744','roughness':.8,'metalness':.35},
 'chrome':{'color':'#b3b9b1','roughness':.3,'metalness':.8},
 'red-lens':{'color':'#a62f25','roughness':.35,'metalness':.1},
 'amber-lens':{'color':'#cf842e','roughness':.35,'metalness':.1},
 'step':{'color':'#60665d','roughness':.9,'metalness':.4}}
parts=[]
def loft(name,rings,material,caps=True):
 parts.append({'name':name,'op':'hypermedia.manifold.loft','args':{'sections':rings,'caps':caps,'interpolation':'linear','samples':2},'material':material})
def box(name,c,size,material):
 x,y,z=c;w,h,l=[v/2 for v in size]
 loft(name,[[[x-w,yy,z-l],[x+w,yy,z-l],[x+w,yy,z+l],[x-w,yy,z+l]] for yy in [y-h,y+h]],material)
def rod(name,a,b,r,material,sides=12):loft(name,native.tube([a,b],r,sides),material)
def roundrect(w,l,r,steps=5):
 result=[]
 for cx,cz,start in [(w/2-r,l/2-r,0),(-w/2+r,l/2-r,math.pi/2),(-w/2+r,-l/2+r,math.pi),(w/2-r,-l/2+r,3*math.pi/2)]:
  for i in range(steps+1):
   a=start+i/steps*math.pi/2;result.append([cx+r*math.cos(a),cz+r*math.sin(a)])
 return result
def flatshape(name,points,axis,offset,thick,material):
 rings=[]
 for position in [offset-thick/2,offset+thick/2]:
  if axis=='x':rings.append([[position,u,v] for u,v in points])
  elif axis=='z':rings.append([[u,v,position] for u,v in points])
 loft(name,rings,material)
def wheel(name,center,axis,radius=.35,width=.16):
 x,y,z=center;path=[];tread=min(.09,radius*.27)
 for i in range(49):
  a=i*math.tau/48
  path.append([x,y+(radius-tread)*math.cos(a),z+(radius-tread)*math.sin(a)] if axis=='x' else [x+(radius-tread)*math.cos(a),y+(radius-tread)*math.sin(a),z])
 loft(name+'-rubber',native.tube(path,[(tread,width/2)]*len(path),12),'rubber',False)
 if axis=='x':a=[x-width/2,y,z];b=[x+width/2,y,z]
 else:a=[x,y,z-width/2];b=[x,y,z+width/2]
 rod(name+'-painted-hub',a,b,radius*.53,'enamel',32)
 aa=a.copy();bb=b.copy();axis_index=0 if axis=='x' else 2;aa[axis_index]-=.008;bb[axis_index]+=.008
 rod(name+'-hubcap',aa,bb,radius*.28,'chrome',24)
def window(name,x,y,z,width,height):
 # Side screen and stitched border are actual native surfaces/lines, not a texture decal.
 box(name+'-screen',[x,y,z],[.012,height,width],'screen')
 for yy in [y-height/2,y+height/2]:rod(name+'-seam-y-'+str(yy),[x*1.006,yy,z-width/2],[x*1.006,yy,z+width/2],.012,'canvas-seam',8)
 for zz in [z-width/2,z+width/2]:rod(name+'-seam-z-'+str(zz),[x*1.006,y-height/2,zz],[x*1.006,y+height/2,zz],.012,'canvas-seam',8)
 for n in [-1,0,1]:
  zz=z+n*width/4;rod(name+'-mesh-'+str(n),[x*1.009,y-height/2+.02,zz],[x*1.009,y+height/2-.02,zz],.0035,'screen-wire',6)
 rod(name+'-mesh-cross',[x*1.009,y,z-width/2+.02],[x*1.009,y,z+width/2-.02],.003,'screen-wire',6)
# Chassis, single axle and period compact tyres.
for x in [-.74,.74]:box('chassis-rail-'+str(x),[x,.49,0],[.09,.12,3.45],'chassis')
for z in [-1.45,-.15,1.45]:box('chassis-crossmember-'+str(z),[0,.49,z],[1.57,.09,.08],'chassis')
rod('single-axle',[-1.15,.36,-.45],[1.15,.36,-.45],.055,'chassis')
for side in [-1,1]:
 wheel(('left' if side<0 else 'right')+'-wheel',[side*1.065,.37,-.45],'x',.35,.18)
 # Shallow rolled wheel guards sit over tyres, without off-road exaggeration.
 path=[[side*1.075,.37+.42*math.sin(i*math.pi/16),-.45+.42*math.cos(i*math.pi/16)] for i in range(17)]
 loft('rolled-wheel-guard-'+str(side),native.tube(path,[(.025,.115)]*len(path),8),'enamel',True)
# Cream lower body with softened vertical corners; recognisably a low camping-trailer tub.
rings=[]
for y,w,l,r in [(.56,2.06,3.25,.09),(.64,2.10,3.30,.10),(1.30,2.10,3.30,.10),(1.35,2.06,3.27,.08)]:rings.append([[x,y,z] for x,z in roundrect(w,l,r)])
loft('cream-lower-body',rings,'enamel')
for side in [-1,1]:
 for y,h,material in [(1.02,.11,'ochre'),(.91,.06,'olive')]:box('period-stripe-side-'+str(side)+'-'+material,[side*1.054,y,0],[.014,h,3.13],material)
 rod('lower-aluminium-trim-'+str(side),[side*1.055,.68,-1.54],[side*1.055,.68,1.54],.012,'chrome',8)
for end in [-1,1]:
 for y,h,material in [(1.02,.11,'ochre'),(.91,.06,'olive')]:box('period-stripe-end-'+str(end)+'-'+material,[0,y,end*1.657],[1.89,h,.015],material)
for side in [-1,1]:
 for end in [-1,1]:
  x=side*.95;z=end*1.53
  box('wind-up-lift-post-'+str(side)+'-'+str(end),[x,1.96,z],[.035,1.37,.035],'chrome')
# Raised rigid roof: rounded perimeter and slim roll-over lip.
loft('raised-roof',[[[x,y,z] for x,z in roundrect(w,l,r)] for y,w,l,r in [(2.58,2.26,3.54,.12),(2.65,2.26,3.54,.12),(2.72,2.16,3.44,.15)]],'roof')
for side in [-1,1]:rod('roof-edge-gutter-'+str(side),[side*1.13,2.61,-1.61],[side*1.13,2.61,1.61],.015,'chrome',8)
# Canvas upper cabin and generous mesh windows.
box('main-canvas-cabin',[0,1.96,0],[2.035,1.20,3.23],'canvas')
for side in [-1,1]:
 for z,width in ([(-.91,1.03),(.91,1.03)] if side<0 else [(-.72,1.34),(1.30,.42)]):window('canvas-window-'+str(side)+'-'+str(z),side*1.025,1.98,z,width,.78)
# Side entrance with rigid lower door and screened top.
box('door-aluminium-frame',[1.064,1.52,.65],[.025,1.84,.61],'chrome')
box('door-lower-panel',[1.086,1.01,.65],[.025,.74,.55],'enamel')
box('door-upper-screen',[1.09,1.94,.65],[.018,.99,.52],'screen')
box('door-central-rail',[1.102,1.43,.65],[.025,.045,.60],'enamel')
rod('door-handle',[1.124,1.22,.83],[1.124,1.22,.92],.015,'chassis')
box('fold-down-step',[1.36,.29,.65],[.52,.045,.64],'step')
for z in [.42,.88]:rod('step-brace-'+str(z),[1.04,.57,z],[1.54,.27,z],.018,'steel',8)
# Pull-out bed floors and their taut, slightly bowed tent ends.
for end in [-1,1]:
 label='front' if end==1 else 'rear';box(label+'-slide-out-bed-platform',[0,1.31,end*2.20],[2.06,.085,1.27],'enamel')
 a=[[-1.015,1.35],[1.015,1.35],[1.015,2.43],[.86,2.58],[-.86,2.58],[-1.015,2.43]]
 b=[[-1.01,1.35],[1.01,1.35],[1.01,2.10],[.78,2.40],[-.78,2.40],[-1.01,2.10]]
 loft(label+'-deployed-canvas-bed-tent',[[[x,y,end*z] for x,y in ring] for z,ring in [(1.61,a),(2.82,b)]],'canvas-roof')
 # Large end screen has chamfered upper corners beneath the tent bow.
 screen=[[-.78,1.53],[.78,1.53],[.78,2.08],[.64,2.24],[-.64,2.24],[-.78,2.08]]
 flatshape(label+'-bed-end-mesh-window',screen,'z',end*2.829,.013,'screen')
 for i in range(len(screen)):
  p=screen[i];q=screen[(i+1)%len(screen)];rod(label+'-bed-screen-seam-'+str(i),[p[0],p[1],end*2.842],[q[0],q[1],end*2.842],.009,'canvas-seam',8)
 for x in [-.4,0,.4]:rod(label+'-bed-screen-weave-'+str(x),[x,1.55,end*2.844],[x,2.21,end*2.844],.003,'screen-wire',6)
 for side in [-1,1]:
  points=[[1.53,end*1.80],[1.53,end*2.65],[2.05,end*2.65],[2.36,end*1.80]]
  flatshape(label+'-side-mesh-'+str(side),points,'x',side*1.025,.012,'screen')
  rod(label+'-sliding-bed-rail-'+str(side),[side*.85,1.245,end*1.35],[side*.85,1.245,end*2.73],.025,'steel',8)
  rod(label+'-diagonal-bed-support-'+str(side),[side*.78,.57,end*1.45],[side*.78,1.25,end*2.60],.025,'steel',8)
  rod(label+'-upper-canvas-seam-'+str(side),[side*.86,2.59,end*1.61],[side*.78,2.41,end*2.82],.009,'canvas-seam',8)
# Grounded corner stabilisers.
for side in [-1,1]:
 for end in [-1,1]:
  x=side*.9;z=end*1.40;rod('stabiliser-leg-'+str(side)+'-'+str(end),[x,.51,z],[x*1.06,.07,z*1.05],.027,'steel',8);box('stabiliser-foot-'+str(side)+'-'+str(end),[x*1.06,.045,z*1.05],[.18,.035,.18],'chassis')
# Drawbar, tow coupling, winding/jockey assembly and gas bottle.
for side in [-1,1]:rod('a-frame-drawbar-'+str(side),[side*.73,.49,1.57],[0,.49,3.30],(.055,.04),'chassis',4)
box('drawbar-crossplate',[0,.49,2.15],[.77,.05,.36],'steel')
rod('tow-coupling',[0,.49,3.18],[0,.49,3.51],.065,'chrome',16)
box('coupler-latch',[0,.58,3.36],[.06,.05,.22],'steel')
rod('jockey-wheel-post',[.19,.18,2.97],[.19,.70,2.97],.025,'chrome',12)
wheel('jockey-wheel',[.19,.13,2.97],'x',.115,.07)
rod('jockey-winder',[.19,.70,2.97],[.34,.70,2.97],.014,'steel',8)
rod('winder-grip',[.34,.70,2.97],[.34,.77,2.97],.02,'chassis',8)
# LPG bottle with neck and protective loop, under the front extended bed.
loft('gas-bottle',native.tube([[0,.53,2.17],[0,.57,2.17],[0,.91,2.17],[0,.96,2.17]],[.12,.15,.15,.10],32),'enamel')
rod('gas-valve',[0,.95,2.17],[0,1.02,2.17],.025,'chrome',12)
rod('gas-valve-guard',[-.09,1.03,2.17],[.09,1.03,2.17],.012,'steel',8)
# Road fittings and rear spare below the deployed bunk.
wheel('rear-spare',[0,.79,-1.76],'z',.31,.15)
for side in [-1,1]:
 box('rear-red-lamp-'+str(side),[side*.81,.78,-1.675],[.18,.10,.025],'red-lens')
 box('front-amber-marker-'+str(side),[side*.86,.78,1.675],[.10,.06,.025],'amber-lens')
# Plain letter-form badge, authored metal strokes rather than an invented model logo.
letters={'J':[(0,1,1,1),(.8,1,.8,.15),(.8,.15,.5,0),(.5,0,.1,.15)],'A':[(0,0,.5,1),(.5,1,1,0),(.25,.45,.75,.45)],'Y':[(0,1,.5,.5),(1,1,.5,.5),(.5,.5,.5,0)],'C':[(1,1,.2,1),(.2,1,0,.75),(0,.75,0,.25),(0,.25,.2,0),(.2,0,1,0)],'O':[(.2,1,.8,1),(.8,1,1,.8),(1,.8,1,.2),(1,.2,.8,0),(.8,0,.2,0),(.2,0,0,.2),(0,.2,0,.8),(0,.8,.2,1)]}
for i,ch in enumerate('JAYCO'):
 for j,(u,v,uu,vv) in enumerate(letters[ch]):rod('jayco-badge-'+str(i)+'-'+str(j),[1.073,1.17+v*.10,-.55-i*.10-u*.07],[1.073,1.17+vv*.10,-.55-i*.10-uu*.07],.004,'olive',6)
if __name__=='__main__':
 print('AUTHORED PARTS',len(parts),flush=True);bridge=SynthBridge(HERE)
 try:
  result=native.produce(bridge,'vintage-windup-camper','1980-era Jayco wind-up camper, deployed',parts)
  result['coordinate_system']={'units':'metres','up':'+Y','tow_forward':'+Z','origin':'ground under body centre'}
  result['dimensions']={'rigid_body_width':2.10,'rigid_body_length':3.30,'deployed_bed_span':5.64,'raised_roof_height':2.72,'overall_with_drawbar_length':6.33}
  result['attachment_points']={'starlink_roof_mount':[.70,2.72,-.65]}
  result['qualification']='Original authored reconstruction of an approximately 1980 Jayco wind-up camper; specific model/serial dimensions not asserted. Deployed canvas sides, raised roof and slide-out beds follow user description.'
  result['reference_urls']=['https://www2.jayco.com.au/history','https://caravanworld.com.au/blogs/editorial/looking-back-offroad-adventures-january-1984']
  (HERE/'assets/vintage-windup-camper.manifest.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
  print(json.dumps({'parts':result['parts'],'triangles':result['triangles'],'preview_dir':result['preview_dir'],'production_replay':result['production']['receipt']['strict_replay_verified'],'seal_replay':result['sealing']['receipt']['strict_replay_verified']}),flush=True)
 finally:bridge.close()