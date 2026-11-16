"""Attach scoped native source provenance; original produced containers remain immutable."""
from pathlib import Path
import json,hashlib,subprocess
ROOT=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def attest(record):
 original=Path(record['native']['path']);source=ROOT/'inputs'/(record['id']+'.glb');output=ROOT/'assets'/(record['id']+'.10d')
 evidence=ROOT/'provenance'/(record['id']+'.json')
 prior=json.loads(evidence.read_text()) if evidence.exists() else {}
 if prior.get('native_helper_sha256')==sha(ROOT/'provenance/attest.exe') and prior.get('original_sha256')==sha(original) and prior.get('source_sha256')==sha(source) and output.exists() and prior.get('attested_sha256')==sha(output): return prior
 result=subprocess.run([str(ROOT/'provenance/attest.exe'),str(original),str(source),str(output)],check=True,capture_output=True,text=True)
 report=json.loads(result.stdout)
 assert report['vertices']==record['vertices'] and report['triangles']==record['triangles'],(record['id'],report,record['vertices'],record['triangles'])
 report.update({'id':record['id'],'original_sha256':sha(original),'source_sha256':sha(source),'attested_sha256':sha(output),'native_helper_sha256':sha(ROOT/'provenance/attest.exe'),'native_helper_source_sha256':sha(ROOT/'provenance/main.rs'),'source_media_type':'model/gltf-binary','rights':'User-commissioned artwork for the Webizen website project; local preview and inclusion in this project authorised by the project owner. No general reuse licence or third-party endorsement asserted.','credential_asserted':False})
 evidence.write_text(json.dumps(report,indent=2));return report

def update(manifestpath):
 manifest=json.loads(manifestpath.read_text())
 for asset in manifest['assets']:
  record=json.loads((ROOT/'cache'/(asset['id']+'.json')).read_text());report=attest(record)
  asset.update({'sha256':report['attested_sha256'],'bytes':report['bytes'],'original_native_sha256':report['original_sha256'],'provenance_receipt':'/production/native-world/provenance/'+asset['id']+'.json'})
 manifest['totals']['bytes']=sum(x['bytes'] for x in manifest['assets'])
 manifest['vertex_count']=sum(x.get('vertices',0) for x in manifest['assets']);manifest['triangle_count']=sum(x.get('triangles',0) for x in manifest['assets'])
 manifest['provenance']='Native QualiaDB ProvenanceSidecar binds actual GLB source bytes and scoped project permission; no VC asserted; original sections and header policy preserved.'
 manifestpath.write_text(json.dumps(manifest,indent=2));print(json.dumps({'manifest':manifestpath.name,'assets':len(manifest['assets']),'bytes':manifest['totals']['bytes'],'passed':True}))
if __name__=='__main__':
 import sys
 update(ROOT/(sys.argv[1] if len(sys.argv)>1 else 'scene.json'))
