from pathlib import Path
p=Path('C:/github/synth/production/webizen-swarm/production/aaa-environment/assemble.py');s=p.read_text(encoding='utf-8-sig')
s=s.replace("'mesh':0,**{k:n[k]", "'mesh':n.get('mesh',0),**{k:n[k]")
s=s.replace("doc={'asset':", "meshgroups=collections.defaultdict(list)\n for part,prim in zip(parts,prims):meshgroups[part.get('_mesh',0)].append(prim)\n meshes=[{'primitives':meshgroups[i]} for i in range(max(meshgroups)+1)]\n doc={'asset':")
s=s.replace("'meshes':[{'primitives':prims}]", "'meshes':meshes")
s=s.replace("nv=sum(len(p['positions'])//3 for p in parts)*len(nodes);nt=sum(len(p['indices'])//3 for p in parts)*len(nodes);", "nv=sum(sum(len(p['positions'])//3 for p in parts if p.get('_mesh',0)==n.get('mesh',0)) for n in nodes);nt=sum(sum(len(p['indices'])//3 for p in parts if p.get('_mesh',0)==n.get('mesh',0)) for n in nodes);")
p.write_text(s,encoding='utf8')
