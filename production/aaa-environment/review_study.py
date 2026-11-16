from assemble import *
def node(id,mesh,x,y,z,s=1,yaw=0):return {'id':id,'mesh':mesh,'translation':[x,y,z],'rotation':[0,math.sin(yaw/2),0,math.cos(yaw/2)],'scale':[s,s,s]}
bridge=SynthBridge(ROOT)
try:
 parts=[];pal={};maps={};nodes=[]
 specs=[('bank-rock-near',[(-1.6,0,.4,.7,.4)]),('grass-near',[(-.5,0,1.3,.9,.2),(1.6,0,.4,.8,.9)]),('grass-mid',[(1,0,-.4,.65,1.5),(-2.3,0,1.3,.8,.1),(.3,0,2,.7,.5)]),('shrub-branches',[(.3,0,-.6,1,0)]),('shrub-leaves',[(.3,0,-.6,1,0)]),('leaf-litter',[(0,0,1,1.4,.2),(-1,0,0,1.2,.5)])]
 for m,(asset,ps) in enumerate(specs):
  data=json.loads((ROOT/'assets'/(asset+'.mesh.json')).read_text());pfx=asset+'-';pal.update({pfx+k:v for k,v in data['palette'].items()});maps.update({pfx+k:data['textures'] for k in data['palette']})
  parts += [{**p,'material':pfx+p['material'],'_mesh':m} for p in data['parts']]
  nodes += [node(asset+'-'+str(i),m,*p) for i,p in enumerate(ps)]
 record=compile_group(bridge,'understorey-study',parts,pal,nodes,maps)
 # Probe-based fill is material dependent; additive fallback remains zero.
 shot={'side':1024,'camera_yaw':.65,'camera_pitch':.28,'camera_distance':8,'camera_target':[0,.45,.4],'sun_direction':[-.4,.8,.6],'direct_intensity':2.4,'ambient_intensity':.5,'clear_color':[.10,.13,.14,1],'shadows':True,'ao_enabled':True,'ao_radius':.05,'ao_strength':.3,'ao_bias':.005,'ao_samples':12,'environment_fallback_rgb':[0,0,0],'environment_probes':[{'position':[0,2,0],'radius':50,'irradiance':[.22,.24,.20],'specular_levels':[[.15,.16,.14],[.12,.13,.11],[.10,.11,.09],[.08,.09,.07],[.06,.07,.05]],'dominant_direction':[0,1,0]}],'times':[0]}
 save(ROOT/'recipes/understorey-shot.json',shot);prev=ROOT/'previews'/('understorey-study-'+str(time.time_ns()));subprocess.run([EXE,'preview','--asset',record['native']['path'],'--out',str(prev),'--shot',str(ROOT/'recipes/understorey-shot.json'),'--hmc',record['production']['receipt']['hmc']['path'],'--asset-key','outputs/understorey-study.10d'],check=True);save(ROOT/'review/assembled-study.json',{'preview':str(prev),'asset':record,'purpose':'Native multi-material assembled plant/rock study, no floor or full world quality claim'});print('ASSEMBLED_STUDY',prev,flush=True)
finally:bridge.close()
