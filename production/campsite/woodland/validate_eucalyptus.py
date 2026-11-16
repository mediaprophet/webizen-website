from pathlib import Path
import sys,json,hashlib,math,datetime
ROOT=Path(__file__).resolve().parent;sys.path.insert(0,str(ROOT.parents[1]));from build_art import parse_dxf
m=json.loads((ROOT/'assets/eucalyptus.manifest.json').read_text());data=json.loads((ROOT/'assets/eucalyptus.json').read_text());rmap={r['key']:r for r in m['production']['receipt']['resources']};tri=0
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
for batch in data['parts']:
 for source in batch['source_parts']:
  r=rmap[source['name']+'.dxf'];p,i=parse_dxf(Path(r['path']));vs=source['vertex_start'];vc=source['vertex_count'];start=source['index_start'];count=source['index_count'];assert p==batch['positions'][vs*3:(vs+vc)*3];assert i==[j-vs for j in batch['indices'][start:start+count]];assert sha(r['path'])==r['sha256']==source['source_sha256'];tri+=len(i)//3
 assert all(math.isfinite(v) for v in batch['positions']);assert min(batch['indices'])>=0 and max(batch['indices'])<len(batch['positions'])//3
for phase in ['production','sealing']:
 r=m[phase]['receipt'];assert r['strict_replay_verified'];assert sha(r['hmc']['path'])==r['hmc']['sha256'];assert r['catalog']['count']==150
assert tri==m['integration']['triangles'];assert sha(ROOT/'recipes/eucalyptus.vibe')==m['production']['receipt']['source_sha256'];model=m['sealing']['receipt']['resources'][0];assert sha(model['path'])==model['sha256'];assert sha(ROOT/'recipes/eucalyptus.obj')==m['sealing']['receipt']['inputs'][0]['sha256']
(ROOT/'review').mkdir(exist_ok=True);report={'schema':'webizen.native-art-validation/v1','validated_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'triangles':tri,'batches':len(data['parts']),'source_parts':len(rmap),'exact_dxf_faces':True,'hashes_match':True,'strict_replay_verified':True};(ROOT/'review/geometry-validation.json').write_text(json.dumps(report,indent=2));print(json.dumps(report))
