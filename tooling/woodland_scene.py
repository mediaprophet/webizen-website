from pathlib import Path
import shutil
root=Path(__file__).resolve().parents[1]
p=root/'site/world/scene.js';s=p.read_text(encoding='utf-8')
s=s.replace('Math.min(1.65,1.0/camera.aspect)','Math.min(1.35,1.0/camera.aspect)')
if 'async function nativeWoodland(' not in s:
 s=s.replace('function material(color,roughness=.9)',"""async function nativeWoodland(parent){const url='/production/campsite/woodland/assets/eucalyptus.json';const data=await fetch(url).then(r=>{if(!r.ok)throw Error('Native woodland artwork unavailable');return r.json();});const locations=[[-35,19,.95,.3],[-24,11,1.05,1.7],[-10,16,.85,2.4],[-38,32,1.0,3.2],[-35,44,.8,4.4],[-18,4,.9,5.2]];const dummy=new T.Object3D();for(const p of data.parts){const geometry=new T.BufferGeometry();geometry.setAttribute('position',new T.Float32BufferAttribute(p.positions,3));geometry.setIndex(p.indices);geometry.computeVertexNormals();const material=new T.MeshStandardMaterial({...data.palette[p.material],side:T.DoubleSide});const mesh=new T.InstancedMesh(geometry,material,locations.length);for(let i=0;i<locations.length;i++){const[x,z,scale,angle]=locations[i];dummy.position.set(x,heightAt(x,z),z);dummy.rotation.set(0,angle,0);dummy.scale.setScalar(scale);dummy.updateMatrix();mesh.setMatrixAt(i,dummy.matrix);}mesh.castShadow=true;mesh.receiveShadow=true;parent.add(mesh);}nativeAssets.push({id:'eucalyptus',parts:data.parts.length,instances:locations.length,source:'Native Synth woodland artwork, exact exported faces instanced in the browser',mesh:url});}
function material(color,roughness=.9)""")
 s=s.replace('loadCampsite(root).catch(console.error);','loadCampsite(root).catch(console.error);nativeWoodland(root).catch(console.error);')
p.write_text(s,encoding='utf-8')
p=root/'site/world/app.js';s=p.read_text(encoding='utf-8')
if "badge.id='journeyRegion'" not in s:
 s=s.replace("$('location').hidden=false;","let badge=$('journeyRegion');if(!badge){badge=node('p','journey-region');badge.id='journeyRegion';badge.lang='en-AU';$('locationTitle').after(badge);}badge.hidden=current!=='field-studio'||!activeChapter;badge.textContent=activeChapter?.title||'';$('location').hidden=false;",1)
p.write_text(s,encoding='utf-8')
p=root/'site/world/style.css'
if not p.exists():shutil.copy2(Path('C:/github/webizen-website/site/world/style.css'),p)
s=p.read_text(encoding='utf-8')
if '.journey-region' not in s:s+='\n.journey-region{display:none}@media(max-width:650px){#location .journey-region:not([hidden]){display:block;font-size:11px;line-height:1.5;margin:8px 0 14px;max-width:210px;color:#d7dfc2}}\n'
p.write_text(s,encoding='utf-8')
p=root/'tests/campsite-review.cjs';s=p.read_text(encoding='utf-8').replace('scene.nativeAssets.length===7','scene.nativeAssets.length===8');p.write_text(s,encoding='utf-8')
print('Prepared six native eucalyptus instances and mobile region label. Integrate only after woodland asset acceptance.')
