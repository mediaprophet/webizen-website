from build import *
from ecology import shrub,litter
PALETTES.update({'twig':{'color':'#85765c','roughness':.94,'metalness':0,'tint':[.46,.39,.29],'seed':292,'relief':.004,'bands':[1,16],'band_strength':.2},'shrub-leaf':{'color':'#6d8559','roughness':.84,'metalness':0,'tint':[.37,.48,.29],'seed':912,'relief':.001,'bands':[2,15],'band_strength':.15},'litter':{'color':'#958462','roughness':.95,'metalness':0,'tint':[.54,.43,.27],'seed':122,'relief':.002,'bands':[2,12],'band_strength':.2}})
def shape(id):
 if id.startswith('bank-rock'):
  seed={'bank-rock-near':19,'bank-rock-mid':57,'bank-rock-far':93}[id];p=rock(seed)[0]
  # Soften bedding transitions while retaining asymmetric stratified shoulders.
  p['args'].update(interpolation='cubic',samples=3 if id.endswith('near') else 1)
  if not id.endswith('near'):p['args']['sections']=[r[::2] for r in p['args']['sections']]
  phase=(seed-19)*.017
  for j,r in enumerate(p['args']['sections']):
   for v in r:
    a=math.atan2(v[2],v[0]);v[0]*=1+.10*math.sin(a*2+phase);v[2]*=1+.08*math.cos(a*3+phase);v[1]*=1+.12*math.sin(phase)
  return [p]
 if id.startswith('grass-'):
  count={'grass-near':72,'grass-mid':28,'grass-far':12}[id];parts=grass(71,count)
  if id=='grass-near':
   for p in parts:
    if p['name'].startswith('arched'):p['args'].update(interpolation='cubic',samples=2)
  return parts
 if id.startswith('shrub-'):
  mat='twig' if id=='shrub-branches' else 'shrub-leaf';return [p for p in shrub() if p['material']==mat]
 if id=='leaf-litter':return litter()
 raise ValueError(id)
if __name__=='__main__':
 bridge=SynthBridge(ROOT)
 try:
  for id in sys.argv[1:]:
   path=ROOT/'assets'/(id+'.manifest.json')
   if path.exists():print('RETAINED',id,flush=True);continue
   g=geometry(bridge,id,shape(id));save(ROOT/'assets'/(id+'.geometry.json'),g);seal(bridge,g)
 finally:bridge.close()
