"""Metre-scale camping equipment, modelled by native Synth morphisms.

No imported/render-only stand-in meshes. Author controls -> native loft -> DXF
faces -> indexed browser adapter, and the same faces sealed as native .10d.
"""
from pathlib import Path
import sys,math,json
HERE=Path(__file__).resolve().parent
PRODUCTION=HERE.parents[1]
sys.path.insert(0,str(PRODUCTION))
import build_art as art
from native_bridge import SynthBridge
art.ROOT=HERE
art.PALETTE={
 'aluminium':{'color':'#b6bdbb','roughness':.28,'metalness':.9},
 'panel':{'color':'#142f4b','roughness':.27,'metalness':.48},
 'cell-line':{'color':'#637a8d','roughness':.4,'metalness':.6},
 'rubber':{'color':'#202422','roughness':.84,'metalness':0},
 'backing':{'color':'#353d3d','roughness':.65,'metalness':.25},
 'dish-front':{'color':'#e9e9df','roughness':.68,'metalness':0},
 'dish-back':{'color':'#aeb4b0','roughness':.68,'metalness':.12},
 'mount':{'color':'#797f7b','roughness':.45,'metalness':.75},
}

def loft(name,rings,material,caps=True):
 return dict(name=name,op='hypermedia.manifold.loft',args=dict(sections=rings,caps=caps,interpolation='linear'),material=material)

def box(name,low,high,material):
 x0,y0,z0=low;x1,y1,z1=high
 return loft(name,[[[x0,y,z0],[x1,y,z0],[x1,y,z1],[x0,y,z1]] for y in [y0,y1]],material)

def rod(name,path,radius,material,sides=12):return loft(name,art.tube(path,radius,sides),material)

def slab(name,point,u0,u1,v0,v1,n0,n1,material):
 return loft(name,[[point(u,v,n) for u,v in [(u0,v0),(u1,v0),(u1,v1),(u0,v1)]] for n in [n0,n1]],material)

def solar():
 parts=[];angle=math.radians(55)
 for panel,cx in enumerate([-.62,0,.62]):
  def pt(u,v,n):return [cx+u,.06+v*math.sin(angle)+n*math.cos(angle),.42-v*math.cos(angle)+n*math.sin(angle)]
  prefix=f'panel-{panel+1}-'
  parts.append(slab(prefix+'rear',pt,-.30,.30,0,1.12,-.03,-.006,'backing'))
  parts.append(slab(prefix+'photovoltaic-face',pt,-.277,.277,.022,1.098,-.006,.002,'panel'))
  for label,u0,u1,v0,v1 in [('left',-.30,-.277,0,1.12),('right',.277,.30,0,1.12),('lower',-.277,.277,0,.022),('upper',-.277,.277,1.098,1.12)]:
   parts.append(slab(prefix+'frame-'+label,pt,u0,u1,v0,v1,-.031,.008,'aluminium'))
  # Six columns by ten rows. Fine physical strips leave a visible cell grid.
  for i in range(1,6):
   u=-.277+i*.554/6;parts.append(slab(prefix+'cell-column-'+str(i),pt,u-.0015,u+.0015,.023,1.097,.002,.003,'cell-line'))
  for j in range(1,10):
   v=.022+j*1.076/10;parts.append(slab(prefix+'cell-row-'+str(j),pt,-.276,.276,v-.0015,v+.0015,.002,.003,'cell-line'))
  for sign in [-1,1]:
   top=pt(sign*.22,.78,-.04);foot=[cx+sign*.22,.035,-.62]
   parts.append(rod(prefix+'kickstand-'+str(sign),[top,foot],.012,'aluminium'))
   parts.append(box(prefix+'stand-foot-'+str(sign),[foot[0]-.045,.01,foot[2]-.035],[foot[0]+.045,.04,foot[2]+.035],'rubber'))
  parts.append(rod(prefix+'stand-crossbar',[[cx-.22,.08,-.58],[cx+.22,.08,-.58]],.01,'aluminium'))
  parts.append(slab(prefix+'junction-box',pt,-.06,.06,.56,.68,-.065,-.03,'rubber'))
 for cx in [-.31,.31]:
  for v in [.22,.84]:
   a=[cx,.06+v*math.sin(angle),.42-v*math.cos(angle)]
   parts.append(rod('folding-hinge-'+str(cx)+'-'+str(v),[[a[0],a[1]-.045,a[2]+.015],[a[0],a[1]+.045,a[2]+.015]],.012,'rubber'))
 cable=[[.84,.45,-.02],[.91,.28,.10],[.97,.08,.31],[1.02,.016,.47],[1.18,.016,.55],[1.36,.016,.43],[1.56,.016,.53],[1.74,.016,.48],[2.02,.016,.63],[2.25,.016,.65]]
 parts.append(rod('solar-power-lead',cable,.005,'rubber',10))
 parts.append(rod('solar-lead-connector',[[2.25,.016,.65],[2.33,.016,.65]],.013,'rubber'))
 return parts

def rounded(width,height,radius,segments=8):
 points=[]
 for cx,cy,start in [(width/2-radius,height/2-radius,0),(-width/2+radius,height/2-radius,90),(-width/2+radius,-height/2+radius,180),(width/2-radius,-height/2+radius,270)]:
  for i in range(segments):
   a=math.radians(start+i*90/segments);points.append([cx+radius*math.cos(a),cy+radius*math.sin(a)])
 return points

def mini():
 parts=[];angle=math.radians(20)
 def pt(u,v,n):return [.08+u,.30+v*math.sin(angle)+n*math.cos(angle),-v*math.cos(angle)+n*math.sin(angle)]
 # Official front dimensions: 259mm width,298.5mm length,38.5mm depth.
 for name,width,length,n0,n1,material in [('rear-housing',.259,.2985,-.0305,0,'dish-back'),('white-radome',.259,.2985,0,.008,'dish-front')]:
  profile=rounded(width,length,.022)
  parts.append(loft('mini-'+name,[[pt(u,v,n) for u,v in profile] for n in [n0,n1]],material))
 # Dark rear connector recess and four moulded rear structural ribs.
 parts.append(slab('mini-rear-recess',pt,-.045,.045,-.05,.05,-.035,-.0305,'backing'))
 for a,b in [((-.103,-.12),(-.05,-.04)),((.103,-.12),(.05,-.04)),((-.103,.12),(-.05,.04)),((.103,.12),(.05,.04))]:
  parts.append(rod('mini-rear-rib-'+str(len(parts)),[pt(*a,-.033),pt(*b,-.033)],.003,'dish-back',8))
 # Generic short roof-edge mount, authored as an installation assumption.
 parts.append(box('roof-mount-base',[-.055,0,-.05],[.055,.015,.05],'mount'))
 parts.append(rod('short-roof-edge-pole',[[0,.012,0],[.035,.15,0],[.08,.25,0]],.016,'mount'))
 parts.append(rod('dish-pivot',[[.035,.25,0],[.125,.25,0]],.022,'rubber'))
 for x in [-.037,.037]:
  for z in [-.034,.034]:parts.append(rod('base-fastener-'+str(x)+'-'+str(z),[[x,.015,z],[x,.022,z]],.005,'aluminium',8))
 port=pt(.028,-.035,-.035)
 parts.append(rod('mini-dc-cable',[port,[.13,.22,.06],[.15,.10,.12],[.15,.015,.16],[.15,-.38,.16]],.0033,'rubber',10))
 return parts

if __name__=='__main__':
 HERE.mkdir(parents=True,exist_ok=True);bridge=SynthBridge(HERE);records=[]
 try:
  for id,title,fn in [('solar-array','Folding ground solar array',solar),('starlink-mini','Starlink Mini with short roof-edge mount',mini)]:
   if len(sys.argv)>1 and sys.argv[1]!=id:continue
   record=art.produce(bridge,id,title,fn());record['scale']={'units':'metres','up':'+Y','front':'+Z'}
   record['mounting']={'origin':'bottom of mount foot','camper_local_position':[.70,2.72,-.65]} if id=='starlink-mini' else {'origin':'array centreline at ground','approximate_panel_width':.60,'panel_length':1.12,'panel_count':3,'inclination_degrees':55}
   record['qualification']='Authored visual reconstruction; not a certified engineering installation or equipment performance simulation.'
   (HERE/'assets'/f'{id}.manifest.json').write_text(json.dumps(record,indent=2),encoding='utf-8');records.append(record)
  (HERE/'assets/collection.json').write_text(json.dumps({'schema':'webizen.campsite-equipment/v1','catalog_count':bridge.catalog.get('count'),'assets':records,'reference':'https://starlink.com/public-files/specification_sheet_mini.pdf'},indent=2),encoding='utf-8')
 finally:bridge.close()
