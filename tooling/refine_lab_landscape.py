from pathlib import Path
root=Path(__file__).resolve().parents[1];p=root/'site/lab/scene.js';s=p.read_text(encoding='utf-8')
s=s.replace("scene.add(ground);","// Native terrain is the visible ground; this helper plane is not added.")
s=s.replace('tex.repeat.set(20,20)','tex.repeat.set(1.5,1.5)')
s=s.replace('terrain.position.set(0,-6,-23);terrain.scale.setScalar(1.1)','terrain.position.set(0,-.5,-15);terrain.scale.set(2,1.2,2)')
s=s.replace(" const assets=[];", " function terrainHeight(x,z){const sx=x/2,sz=(z+15)/2;let best=Infinity,y=0;for(const part of land.parts){for(let i=0;i<part.positions.length;i+=3){const d=(part.positions[i]-sx)**2+(part.positions[i+2]-sz)**2;if(d<best){best=d;y=part.positions[i+1]*1.2-.5;}}}return y;}\n const assets=[];")
s=s.replace("group.position.set(12,0,8)","group.position.set(12,0,8)").replace("group.position.set(-12,0,-7)","group.position.set(-16,0,8)")
s=s.replace("  scene.add(group);assets.push", "  const low=new T.Box3().setFromObject(group).min.y;group.position.y+=terrainHeight(group.position.x,group.position.z)-low+.04;scene.add(group);assets.push")
s=s.replace("scene.add(sky);","scene.add(sky);const environmentScene=new T.Scene();environmentScene.add(sky.clone());const pmrem=new T.PMREMGenerator(renderer);scene.environment=pmrem.fromScene(environmentScene,.04,.1,300).texture;pmrem.dispose();")
p.write_text(s,encoding='utf-8')
p=root/'site/lab/lab.css';s=p.read_text(encoding='utf-8');s+='\n@media(max-width:700px){.hero{min-height:820px;height:95vh}.hero-shade{background:linear-gradient(180deg,rgba(9,35,29,.85),rgba(9,35,29,.5) 48%,rgba(9,35,29,.05) 72%)}.hero-copy{inset:130px 7% auto}.hero-copy h1{font-size:44px}.hero-copy>p:not(.eyebrow){font-size:14px}.scene-caption{bottom:75px}.scene-controls{bottom:28px}}\n'
p.write_text(s,encoding='utf-8');print('Native terrain now supports artwork placement; environment reflections added.')
