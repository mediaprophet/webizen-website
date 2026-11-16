import json,struct,pathlib
P=pathlib.Path('production/webizen-swarm/production/aaa-environment')
for p in sorted((P/'groups').glob('ecology*.json')):
 g=json.loads(p.read_text());b=pathlib.Path(g['input']).read_bytes();n=struct.unpack_from('<I',b,12)[0];d=json.loads(b[20:20+n]);pts=[v['translation'] for v in d['nodes']];print(g['id'],g['vertices'],'xz',[(round(x[0],1),round(x[2],1)) for x in pts])
