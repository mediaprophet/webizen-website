from pathlib import Path
root=Path(__file__).resolve().parents[1]
p=root/'site/world/scene.js';s=p.read_text(encoding='utf-8')
s=s.replace('if(Math.hypot(x+22,z-28)<11||d<7||',"const dx=x+22,dz=z-28,viewCorridor=dx*.591+dz*.806>0&&dx*.591+dz*.806<44&&Math.abs(dx*.806-dz*.591)<9;if(Math.hypot(dx,dz)<28||viewCorridor||d<7||")
s=s.replace('new T.PerspectiveCamera(49,','new T.PerspectiveCamera(innerWidth<650?58:49,')
s=s.replace('camera.aspect=innerWidth/innerHeight;camera.updateProjectionMatrix();','camera.aspect=innerWidth/innerHeight;camera.fov=innerWidth<650?58:49;camera.updateProjectionMatrix();')
s=s.replace('Math.min(2.1,1.0/camera.aspect)','Math.min(1.65,1.0/camera.aspect)')
p.write_text(s,encoding='utf-8')
print('Cleared obstructing near-camera vegetation; mobile lens and distance fit whole camp.')
