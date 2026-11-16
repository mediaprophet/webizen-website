from pathlib import Path
p=Path('C:/github/synth/production/webizen-swarm/production/aaa-environment/assemble.py')
s=p.read_text(encoding='utf-8-sig')
lines=s.splitlines()
for i,line in enumerate(lines):
 if line.startswith(' tangent='):lines[i]=" tangent='' # Native GLB already supplies MikkTSpace corner splits."
 if line.startswith(' tanarg='):lines[i]=" tanarg=''"
s='\n'.join(lines)+'\n'
s=s.replace("assert report['vertices']==nv and report['triangles']==nt","assert nv <= report['vertices'] <= nt*3 and report['triangles']==nt,(report,nv,nt);source_nv=nv;nv=report['vertices']")
s=s.replace("'vertices':nv,'triangles':nt,'colour'","'vertices':nv,'source_vertices':source_nv,'native_attribute_policy':'Native GLB MikkTSpace corner splits retained; exact triangle count preserved','triangles':nt,'colour'")
p.write_text(s,encoding='utf8')
