from pathlib import Path
import json,shutil
root=Path(__file__).resolve().parents[1];repo=Path('C:/github/webizen-website');data=root/'data';data.mkdir(exist_ok=True)
world=json.loads((repo/'data/world.json').read_text(encoding='utf-8'))
if not any(x['id']=='field-studio' for x in world['exhibits']):
 world['exhibits'].append({'id':'field-studio','position':[-22,2,28],'camera':[11,5.5,13],'kind':'campsite','modules':['Webizen','Synth','Hypermedia','VibeScript'],'links':['hypermedia-studio.html']})
 world['relationships']+=[{'from':'field-studio','to':'atelier','kind':'knowledge'},{'from':'field-studio','to':'commons','kind':'socioeconomic'},{'from':'river','to':'field-studio','kind':'information'}]
(data/'world.json').write_text(json.dumps(world,ensure_ascii=False,indent=2),encoding='utf-8')
for name in ['en.json','en-AU.json']:
 p=root/'site/locales'/name;d=json.loads(p.read_text(encoding='utf-8'));d.update({'field-studio':'The field studio','field-studioShort':'Living, working and making from a campsite.','field-studioBody':'A gold 2013 Volkswagen Passat wagon, a 1980 Jayco wind-up camper, solar panels on the ground and a Starlink Mini on the roof. This campsite reflects the circumstances in which this ecosystem has been developed: living, working, making and staying connected. The vehicle and camper are authored interpretations of the described models.'});p.write_text(json.dumps(d,ensure_ascii=False,indent=2),encoding='utf-8')
p=root/'site/world/scene.js';s=p.read_text(encoding='utf-8')
s=s.replace('if(d<7||h>26||','if(Math.hypot(x+22,z-28)<11||d<7||h>26||')
s=s.replace(" const timber=material", " loadCampsite(root).catch(console.error);\n const timber=material",1)
anchor="function material(color,roughness=.9)"
insert="""async function loadCampsite(parent){const manifest=await fetch('/production/campsite/campsite.json').then(r=>{if(!r.ok)throw Error('Campsite production manifest unavailable');return r.json();});const cx=-22,cz=28;const camperHeight=heightAt(cx+1,cz);for(const asset of manifest.assets){const data=await fetch(asset.mesh_url).then(r=>r.json());const group=new T.Group();const batches=new Map();for(const p of data.parts){let b=batches.get(p.material);if(!b){b={positions:[],indices:[]};batches.set(p.material,b);}const offset=b.positions.length/3;for(const n of p.positions)b.positions.push(n);for(const n of p.indices)b.indices.push(n+offset);}for(const[key,p]of batches){const g=new T.BufferGeometry();g.setAttribute('position',new T.Float32BufferAttribute(p.positions,3));g.setIndex(p.indices);g.computeVertexNormals();const m=new T.Mesh(g,new T.MeshStandardMaterial({...data.palette[key],side:T.DoubleSide}));m.castShadow=true;m.receiveShadow=true;group.add(m);}const[x,y,z]=asset.position;group.position.set(cx+x,(asset.anchor==='camper'?camperHeight:heightAt(cx+x,cz+z))+y,cz+z);group.rotation.y=asset.rotation_y||0;parent.add(group);nativeAssets.push({id:asset.id,parts:data.parts.length,source:'Synth-produced campsite artwork',mesh:asset.mesh_url});}}
"""
if 'async function loadCampsite(' not in s:s=s.replace(anchor,insert+anchor)
s=s.replace('sky.castShadow=false;sky.receiveShadow=false;',"sky.castShadow=false;sky.receiveShadow=false;const reflectionScene=new T.Scene();reflectionScene.add(sky.clone());const reflection=new T.PMREMGenerator(renderer);scene.environment=reflection.fromScene(reflectionScene,.05,.1,500).texture;reflection.dispose();")
p.write_text(s,encoding='utf-8')
p=root/'site/world/app.js'
if not p.exists():shutil.copy2(repo/'site/world/app.js',p)
s=p.read_text(encoding='utf-8');needle="body.append(tags);experiment(body,e);"
s=s.replace(needle,needle+"if(id==='field-studio'){const journey=node('a');journey.href='/journey/';journey.textContent='Follow the journey through broad landscapes ↗';journey.lang='en-AU';body.append(journey);const source=node('a');source.href='/production/campsite/campsite.json';source.textContent='Inspect the native campsite artworks ↗';source.lang='en-AU';body.append(node('br'),source);}")
p.write_text(s,encoding='utf-8')
p=root/'tooling/integrate.py';s=p.read_text(encoding='utf-8').replace("'tests','lab']","'tests','lab','data']");p.write_text(s,encoding='utf-8')
print('Prepared a seventh place: The field studio. Native assets integrate when their manifest is complete.')
