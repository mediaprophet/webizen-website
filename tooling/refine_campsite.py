from pathlib import Path
import json,shutil
root=Path(__file__).resolve().parents[1]
p=root/'site/world/scene.js';s=p.read_text(encoding='utf-8')
s=s.replace("geo.setAttribute('color',new T.Float32BufferAttribute(colors,3));mesh(geo,new T.MeshStandardMaterial({vertexColors:true,roughness:1}),root);", """geo.setAttribute('color',new T.Float32BufferAttribute(colors,3));const groundUv=[];for(let i=0;i<ground.positions.length;i+=3)groundUv.push(ground.positions[i]/6,ground.positions[i+2]/6);geo.setAttribute('uv',new T.Float32BufferAttribute(groundUv,2));const groundMaterial=new T.MeshStandardMaterial({vertexColors:true,roughness:1});mesh(geo,groundMaterial,root);fetch('/production/assets/environment.json').then(r=>r.json()).then(async e=>{const loader=new T.TextureLoader();const normal=await loader.loadAsync(e.textures.ground.normal);normal.wrapS=normal.wrapT=T.RepeatWrapping;normal.anisotropy=4;groundMaterial.normalMap=normal;groundMaterial.normalScale.set(.45,.45);groundMaterial.needsUpdate=true;}).catch(console.error);""")
s=s.replace('const to=target.clone().add(new T.Vector3(...e.camera));',"const framing=innerWidth<650?Math.min(2.1,1.0/camera.aspect):1;const to=target.clone().add(new T.Vector3(...e.camera).multiplyScalar(framing));")
p.write_text(s,encoding='utf-8')
p=root/'site/world/app.js';s=p.read_text(encoding='utf-8').replace('el.hidden=!p.visible;','el.hidden=!p.visible||p.id===current;');p.write_text(s,encoding='utf-8')
terrain=root/'production/campsite/terrain/terrain-production.json'
if terrain.exists():shutil.copy2(terrain,root/'data/terrain-production.json')
print('Refined mobile framing, active marker and native ground normal texture.')
