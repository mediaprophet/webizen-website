"""An authored 1980-era wind-up camper produced by Synth lofts.

Silhouette follows Wikimedia Jay Series 1006 packed and set-up photographs
plus Jayco 14SO / fold-down open stills: low white enamel tub, thin hard lid,
taut white canvas as thin frames only, large grey mesh as the cabin wall,
navy valance along the canvas hem, front and rear slide-out bunks with a
shallow barrel-vault canvas roof, two-piece side door, white steel rims,
small fender eyebrows, roof air-conditioner, A-frame with diamond-plate box,
gas and jockey, spare on the rear bumper.
"""
from pathlib import Path
import sys,math,json
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parents[1]))
import build_art as native
from native_bridge import SynthBridge
native.ROOT=HERE
native.PALETTE={
 'enamel':{'color':'#f3f0e8','roughness':.58,'metalness':.08},
 'roof':{'color':'#f8f6f1','roughness':.52,'metalness':.08},
 'canvas':{'color':'#f4efe6','roughness':1,'metalness':0},
 'canvas-roof':{'color':'#f7f4ee','roughness':1,'metalness':0},
 'screen':{'color':'#5f665e','roughness':1,'metalness':0},
 'screen-wire':{'color':'#7a8278','roughness':1,'metalness':0},
 'canvas-seam':{'color':'#1e3a5f','roughness':1,'metalness':0},
 'ochre':{'color':'#c4a04a','roughness':.75,'metalness':.02},
 'olive':{'color':'#2f5c4e','roughness':.75,'metalness':.02},
 'rubber':{'color':'#252824','roughness':.95,'metalness':0},
 'steel':{'color':'#8a918c','roughness':.45,'metalness':.7},
 'chassis':{'color':'#2a2e2c','roughness':.8,'metalness':.35},
 'chrome':{'color':'#c5cbc4','roughness':.3,'metalness':.8},
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
def roundrect(w,l,r,steps=4):
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
 x,y,z=center;path=[];tread=min(.08,radius*.24)
 for i in range(49):
  a=i*math.tau/48
  path.append([x,y+(radius-tread)*math.cos(a),z+(radius-tread)*math.sin(a)] if axis=='x' else [x+(radius-tread)*math.cos(a),y+(radius-tread)*math.sin(a),z])
 loft(name+'-rubber',native.tube(path,[(tread,width/2)]*len(path),12),'rubber',False)
 if axis=='x':a=[x-width/2,y,z];b=[x+width/2,y,z]
 else:a=[x,y,z-width/2];b=[x,y,z+width/2]
 rod(name+'-painted-hub',a,b,radius*.72,'enamel',32)
 aa=a.copy();bb=b.copy();axis_index=0 if axis=='x' else 2;aa[axis_index]-=.006;bb[axis_index]+=.006
 rod(name+'-hubcap',aa,bb,radius*.22,'chrome',24)
def arch_xy(width,y_eave,rise,n=10):
 hw=width/2
 return [[-hw+i/n*width, y_eave+rise*math.sin(math.pi*i/n)] for i in range(n+1)]
def bunk_roof_skin(width,y_eave,rise,thick=.018,n=10):
 outer=arch_xy(width,y_eave,rise,n)
 inner=[[x,y-thick] for x,y in reversed(outer)]
 return outer+inner
def bunk_end_outline(width,y_bot,y_eave,rise,n=10):
 hw=width/2
 return [[-hw,y_bot]]+arch_xy(width,y_eave,rise,n)+[[hw,y_bot]]

TUB_TOP=1.34
NAVY_BOT=1.32
NAVY_TOP=1.54
MESH_TOP=2.50
LID_Y=2.62
FRAME=.055

# Black chassis, single axle, period compact tyres with white steel rims.
for x in [-.74,.74]:box('chassis-rail-'+str(x),[x,.46,0],[.09,.12,3.42],'chassis')
for z in [-1.45,-.15,1.45]:box('chassis-crossmember-'+str(z),[0,.46,z],[1.57,.08,.08],'chassis')
box('chassis-skirt',[0,.545,0],[2.12,.05,3.30],'chassis')
rod('single-axle',[-1.15,.36,-.42],[1.15,.36,-.42],.055,'chassis')
for side in [-1,1]:
 wheel(('left' if side<0 else 'right')+'-wheel',[side*1.06,.36,-.42],'x',.34,.17)
 path=[[side*1.07,.72+.18*math.sin(i*math.pi/10),-.42+.38*math.cos(i*math.pi/10)] for i in range(11)]
 loft('rolled-wheel-guard-'+str(side),native.tube(path,[(.028,.13)]*len(path),8),'enamel',True)

# Cream tub: upper run full length, lower run split around the wheel well.
loft('cream-upper-body',[[[x,y,z] for x,z in roundrect(w,l,r)] for y,w,l,r in [
 (.70,2.08,3.24,.08),(.78,2.10,3.28,.10),(1.28,2.10,3.28,.10),(TUB_TOP,2.04,3.22,.08)
]],'enamel')
loft('cream-lower-front',[[[x,y,z] for x,z in roundrect(w,l,r)] for y,w,l,r in [
 (.56,2.04,1.50,.06),(.70,2.08,1.54,.08)
]],'enamel')
for ring in parts[-1]['args']['sections']:
 for p in ring:p[2]+=.85
loft('cream-lower-rear',[[[x,y,z] for x,z in roundrect(w,l,r)] for y,w,l,r in [
 (.56,2.04,.78,.06),(.70,2.08,.82,.08)
]],'enamel')
for ring in parts[-1]['args']['sections']:
 for p in ring:p[2]-=1.22

# Dual pinstripes and a teal double-wave on the rear quarter.
for side in [-1,1]:
 box('period-stripe-side-'+str(side)+'-upper',[side*1.054,1.10,0],[.010,.016,3.08],'chassis')
 box('period-stripe-side-'+str(side)+'-lower',[side*1.054,1.04,0],[.010,.012,3.08],'ochre')
 rod('lower-aluminium-trim-'+str(side),[side*1.055,.72,-1.52],[side*1.055,.72,1.52],.010,'chrome',8)
 rod('tub-rim-trim-'+str(side),[side*1.052,TUB_TOP,-1.52],[side*1.052,TUB_TOP,1.52],.009,'chrome',8)
 for k,(y0,amp) in enumerate([(1.18,.055),(1.11,.040)]):
  rings=[]
  z0,z1=-1.42,-.55
  n=10
  for i in range(n+1):
   t=i/n;z=z0+t*(z1-z0);yy=y0+amp*math.sin(t*math.pi)
   x=side*1.056
   rings.append([[x,yy-.012,z],[x+side*.012,yy-.012,z],[x+side*.012,yy+.012,z],[x,yy+.012,z]])
  loft('chevron-wave-'+str(side)+'-'+str(k),rings,'olive')
for end in [-1,1]:
 box('period-stripe-end-'+str(end)+'-upper',[0,1.10,end*1.648],[1.86,.016,.010],'chassis')
 box('period-stripe-end-'+str(end)+'-lower',[0,1.04,end*1.648],[1.86,.012,.010],'ochre')
for side in [-1,1]:
 for end in [-1,1]:
  box('roof-latch-'+str(side)+'-'+str(end),[side*.98,1.37,end*1.52],[.045,.06,.07],'steel')
  rod('wind-up-lift-post-'+str(side)+'-'+str(end),[side*.96,TUB_TOP,end*1.50],[side*.96,MESH_TOP+.04,end*1.50],.011,'roof',8)

# Thin hard lid, almost flat, with a small drip edge. Roof AC toward the hitch.
loft('raised-roof',[[[x,y,z] for x,z in roundrect(w,l,r)] for y,w,l,r in [
 (2.56,2.20,3.42,.10),(2.60,2.24,3.46,.12),(2.64,2.16,3.36,.13),(2.68,2.02,3.22,.14)
]],'roof')
for side in [-1,1]:
 rod('roof-edge-gutter-'+str(side),[side*1.12,2.57,-1.58],[side*1.12,2.57,1.58],.010,'chrome',8)
 box('lid-drip-edge-'+str(side),[side*1.12,2.54,0],[.016,.06,3.34],'roof')
box('lid-drip-edge-front',[0,2.54,1.70],[2.18,.06,.016],'roof')
box('lid-drip-edge-rear',[0,2.54,-1.70],[2.18,.06,.016],'roof')
box('roof-ac-housing',[0,2.80,.28],[.78,.22,.92],'roof')
box('roof-ac-cap',[0,2.92,.28],[.70,.04,.80],'roof')
for i,z in enumerate([.00,.18,.36,.54]):
 box('roof-ac-vent-'+str(i),[0,2.93,z],[.62,.018,.06],'chrome')

# Cabin long walls: the wall is grey mesh. White canvas is thin posts and a top strip.
# Navy hem sits on the tub rim. Door splits the right (+X) wall toward the hitch.
CABIN_BAYS={
 -1:[(-1.00,.90,'left-rear'),(0.00,.90,'left-mid'),(1.00,.90,'left-front')],
  1:[(-0.54,1.84,'right-rear'),(1.18,.56,'right-front')]
}
for side,bays in CABIN_BAYS.items():
 for z,width,tag in bays:
  box('cabin-mesh-'+tag,[side*1.036,(NAVY_TOP+MESH_TOP)/2,z],[.010,MESH_TOP-NAVY_TOP,width],'screen')
  rod('cabin-mesh-bar-'+tag,[side*1.042,NAVY_TOP+.06,z],[side*1.042,MESH_TOP-.06,z],.003,'screen-wire',6)
  box('cabin-top-strip-'+tag,[side*1.030,(MESH_TOP+2.56)/2,z],[.016,2.56-MESH_TOP,width+.04],'canvas')
for side in [-1,1]:
 box('cabin-valance-'+str(side),[side*1.032,(NAVY_BOT+NAVY_TOP)/2,0],[.018,NAVY_TOP-NAVY_BOT,3.16],'canvas-seam')
 zs=[-1.50,-0.50,0.50,1.50] if side<0 else [-1.50,0.38,0.86,1.50]
 for z in zs:
  box('cabin-upright-'+str(side)+'-'+str(z),[side*1.038,(NAVY_BOT+2.56)/2,z],[.018,2.56-NAVY_BOT,FRAME],'canvas')

# Two-piece door: hard enamel leaf in the tub, screened upper leaf from the lid.
DOOR_Z=.62
box('door-lower-panel',[1.074,.98,DOOR_Z],[.022,.66,.44],'enamel')
box('door-lower-window',[1.086,1.10,DOOR_Z],[.010,.20,.26],'screen')
box('door-join-rail',[1.080,1.35,DOOR_Z],[.020,.030,.46],'enamel')
box('door-upper-screen',[1.078,(NAVY_TOP+MESH_TOP)/2,DOOR_Z],[.012,MESH_TOP-NAVY_TOP,.38],'screen')
for z in [DOOR_Z-.22,DOOR_Z+.22]:
 box('door-post-'+str(z),[1.070,(NAVY_BOT+2.56)/2,z],[.016,2.56-NAVY_BOT,.040],'canvas')
box('door-head',[1.070,2.54,DOOR_Z],[.016,.06,.48],'canvas')
box('door-navy-sill',[1.072,(NAVY_BOT+NAVY_TOP)/2,DOOR_Z],[.016,NAVY_TOP-NAVY_BOT,.42],'canvas-seam')
rod('door-handle',[1.118,1.12,.78],[1.118,1.12,.88],.013,'chassis')
box('fold-down-step',[1.32,.26,DOOR_Z],[.44,.035,.56],'step')
for z in [.42,.82]:rod('step-brace-'+str(z),[1.04,.52,z],[1.46,.24,z],.015,'steel',8)

# Hitch-side storage hatch in the tub (ochre, as in the Jay Series stills).
box('storage-hatch',[1.068,.92,1.22],[.016,.36,.42],'ochre')
rod('storage-hatch-latch',[1.086,.92,1.22],[1.100,.92,1.22],.008,'chrome',8)

# Slide-out bunks: thin platforms, shallow barrel-vault canvas roof, mesh sides and
# arched mesh end with a white canvas frame, navy hem wrapping the bottom.
for end in [-1,1]:
 label='front' if end==1 else 'rear'
 box(label+'-slide-out-bed-platform',[0,1.325,end*2.22],[2.04,.055,1.28],'canvas-roof')
 stations=[]
 for z,eave,rise,w in [(1.62,2.54,.04,2.18),(1.92,2.26,.08,2.14),(2.22,2.08,.11,2.10),(2.54,1.96,.12,2.06),(2.86,1.90,.12,2.04)]:
  stations.append([[x,y,end*z] for x,y in bunk_roof_skin(w,eave,rise,thick=.014)])
 loft(label+'-deployed-canvas-bed-tent',stations,'canvas-roof',False)
 for side in [-1,1]:
  trap=[[NAVY_TOP,end*1.64],[2.48,end*1.64],[1.86,end*2.84],[NAVY_TOP,end*2.84]]
  flatshape(label+'-side-mesh-'+str(side),trap,'x',side*1.030,.010,'screen')
  top=[[2.48,end*1.64],[2.56,end*1.64],[1.92,end*2.84],[1.86,end*2.84]]
  flatshape(label+'-side-top-canvas-'+str(side),top,'x',side*1.028,.012,'canvas')
  box(label+'-side-valance-'+str(side),[side*1.030,(NAVY_BOT+NAVY_TOP)/2,end*2.24],[.016,NAVY_TOP-NAVY_BOT,1.24],'canvas-seam')
  rod(label+'-sliding-bed-rail-'+str(side),[side*.84,1.255,end*1.38],[side*.84,1.255,end*2.72],.020,'canvas-roof',8)
  rod(label+'-diagonal-bed-support-'+str(side),[side*.76,.56,end*1.48],[side*.76,1.24,end*2.58],.020,'canvas-roof',8)
  rod(label+'-corner-post-'+str(side),[side*1.02,NAVY_BOT,end*2.84],[side*1.02,1.90,end*2.84],.010,'canvas',8)
 outline=bunk_end_outline(2.00,NAVY_TOP,1.90,.12)
 flatshape(label+'-bed-end-mesh-window',outline,'z',end*2.855,.010,'screen')
 box(label+'-end-valance',[0,(NAVY_BOT+NAVY_TOP)/2,end*2.855],[2.00,NAVY_TOP-NAVY_BOT,.016],'canvas-seam')
 for i in range(len(outline)-1):
  p=outline[i];q=outline[i+1]
  rod(label+'-bed-end-frame-'+str(i),[p[0],p[1],end*2.868],[q[0],q[1],end*2.868],.008,'canvas',8)

# Grounded corner stabilisers.
for side in [-1,1]:
 for end in [-1,1]:
  x=side*.9;z=end*1.40
  rod('stabiliser-leg-'+str(side)+'-'+str(end),[x,.50,z],[x*1.05,.07,z*1.04],.024,'steel',8)
  box('stabiliser-foot-'+str(side)+'-'+str(end),[x*1.05,.04,z*1.04],[.16,.03,.16],'chassis')

# A-frame drawbar, coupling, jockey, diamond-plate box and gas.
for side in [-1,1]:rod('a-frame-drawbar-'+str(side),[side*.72,.46,1.55],[0,.46,3.28],(.055,.04),'chassis',4)
box('drawbar-crossplate',[0,.46,2.12],[.74,.05,.32],'steel')
rod('tow-coupling',[0,.46,3.16],[0,.46,3.48],.06,'chrome',16)
box('coupler-latch',[0,.55,3.32],[.055,.045,.20],'steel')
box('front-boot',[.18,.78,2.20],[.72,.38,.52],'steel')
box('front-boot-lid',[.18,.98,2.20],[.70,.04,.50],'steel')
rod('jockey-wheel-post',[.22,.16,2.94],[.22,.66,2.94],.024,'chrome',12)
wheel('jockey-wheel',[.22,.12,2.94],'x',.11,.06)
rod('jockey-winder',[.22,.66,2.94],[.36,.66,2.94],.013,'steel',8)
rod('winder-grip',[.36,.66,2.94],[.36,.73,2.94],.018,'chassis',8)
loft('gas-bottle',native.tube([[-.34,.48,2.14],[-.34,.52,2.14],[-.34,.82,2.14],[-.34,.86,2.14]],[.09,.12,.12,.08],32),'enamel')
rod('gas-valve',[-.34,.85,2.14],[-.34,.92,2.14],.02,'chrome',12)
rod('gas-valve-guard',[-.42,.93,2.14],[-.26,.93,2.14],.01,'steel',8)

# Rear diamond-plate panel, spare on the bumper, lamps.
box('rear-diamond-plate',[0,.92,-1.66],[1.70,.70,.018],'steel')
wheel('rear-spare',[0,.70,-1.82],'z',.29,.12)
for side in [-1,1]:
 box('rear-red-lamp-'+str(side),[side*.72,.74,-1.675],[.14,.08,.022],'red-lens')
 box('front-amber-marker-'+str(side),[side*.84,.80,1.655],[.08,.05,.022],'amber-lens')

# Small authored letter-form on the rear quarter; the wave graphic carries the period look.
letters={'J':[(0,1,1,1),(.8,1,.8,.15),(.8,.15,.5,0),(.5,0,.1,.15)],'A':[(0,0,.5,1),(.5,1,1,0),(.25,.45,.75,.45)],'Y':[(0,1,.5,.5),(1,1,.5,.5),(.5,.5,.5,0)]}
for i,ch in enumerate('JAY'):
 for j,(u,v,uu,vv) in enumerate(letters[ch]):
  rod('jay-badge-'+str(i)+'-'+str(j),[1.068,1.18+v*.09,-1.05-i*.09-u*.06],[1.068,1.18+vv*.09,-1.05-i*.09-uu*.06],.004,'olive',6)

if __name__=='__main__':
 print('AUTHORED PARTS',len(parts),flush=True);bridge=SynthBridge(HERE)
 try:
  result=native.produce(bridge,'vintage-windup-camper','1980-era Jayco wind-up camper, deployed',parts)
  result['coordinate_system']={'units':'metres','up':'+Y','tow_forward':'+Z','origin':'ground under body centre'}
  result['dimensions']={'rigid_body_width':2.10,'rigid_body_length':3.28,'deployed_bed_span':5.72,'raised_roof_height':2.68,'overall_with_drawbar_length':6.30}
  result['attachment_points']={'starlink_roof_mount':[.70,2.70,-.65]}
  result['qualification']='Original authored reconstruction of an approximately 1980 Jayco wind-up camper; specific model/serial dimensions not asserted. Silhouette follows Jay Series 1006 and 14SO packed/set-up photographs: thin hard lid, taut white canvas frames, large grey mesh walls, navy hem, shallow barrel-vault bunks, roof air-conditioner, white rims, spare on the rear bumper.'
  result['reference_urls']=['https://commons.wikimedia.org/wiki/File:JaycoPopupCamperClosed-2009-11-19.jpg','https://commons.wikimedia.org/wiki/File:JaycoPopupCamper-2009-11-17.jpg','https://commons.wikimedia.org/wiki/File:JaycoPopupCamper-Backside-2009-11-12.jpg','https://commons.wikimedia.org/wiki/File:JaycoPopupCamper-RearBunk-2009-11-12.jpg','https://commons.wikimedia.org/wiki/File:Jayco_14SO_open_(front_view).jpg']
  (HERE/'assets/vintage-windup-camper.manifest.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
  print(json.dumps({'parts':result['parts'],'triangles':result['triangles'],'preview_dir':result['preview_dir'],'production_replay':result['production']['receipt']['strict_replay_verified'],'seal_replay':result['sealing']['receipt']['strict_replay_verified']}),flush=True)
 finally:bridge.close()
