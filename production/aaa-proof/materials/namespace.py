"""Orchestrate native content-identity material namespacing; no geometry/codec implementation."""
from pathlib import Path
import hashlib,json,subprocess
HERE=Path(__file__).resolve().parent
STAGE=HERE.parents[2]
ASSETS=HERE.parent/'assets'
EXE=HERE/'material-namespace.exe'
REGISTRY=HERE/'identity-registry.json'
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def _check_identities(registry,receipt):
    for item in receipt['materials']:
        key=item['new_id']; canonical=item['canonical_hex']
        if key in registry and registry[key]['canonical_hex']!=canonical:
            raise ValueError(f'Material content identity collision: {key}')
        registry[key]={'canonical_hex':canonical,'canonical_sha256':item['canonical_sha256']}
def namespace_asset(record,input_path,output_path,*,output_url=None):
    """Return updated asset record after native immutable-derivative packaging.

    input_path/output_path may be str or Path. Output must remain under aaa-proof/assets.
    Cached reuse requires exact input/output/helper identities and a prior passing receipt.
    """
    source=Path(input_path).resolve();target=Path(output_path).resolve()
    target.relative_to(ASSETS.resolve())
    if source==target:raise ValueError('Source output must remain immutable')
    source_hash=sha(source)
    if record.get('sha256') and record['sha256']!=source_hash:
        raise ValueError('Input asset differs from its manifest SHA256')
    helper_hash=sha(EXE);receipt_path=target.with_suffix('.materials.json')
    target.parent.mkdir(parents=True,exist_ok=True)
    if target.exists():
        if not receipt_path.exists():raise ValueError('Existing derivative has no native receipt; choose a new output path')
        receipt=json.loads(receipt_path.read_text(encoding='utf-8'))
        if not(receipt.get('passed') and receipt['source_sha256']==source_hash and receipt['sha256']==sha(target) and receipt.get('native_helper_sha256')==helper_hash):
            raise ValueError('Existing derivative identity changed; choose a new output path')
    else:
        done=subprocess.run([str(EXE),str(source),str(target)],check=True,capture_output=True)
        receipt=json.loads(done.stdout)
        receipt['native_helper_sha256']=helper_hash
    if not(receipt['passed'] and receipt['source_sha256']==source_hash and receipt['sha256']==sha(target) and receipt['non_material_sections_byte_exact'] and receipt['header_flags_preserved'] and not receipt['native_governance_refused']):
        raise ValueError('Native material namespace verification failed')
    for key in ['vertices','triangles']:
        if key in record and record[key]!=receipt[key]:raise ValueError('Native decoded '+key+' differs from original manifest')
    registry=json.loads(REGISTRY.read_text(encoding='utf-8')) if REGISTRY.exists() else {}
    _check_identities(registry,receipt)
    temp=REGISTRY.with_suffix('.tmp');temp.write_text(json.dumps(registry,indent=2),encoding='utf-8');temp.replace(REGISTRY)
    receipt_path.write_text(json.dumps(receipt,indent=2),encoding='utf-8')
    url=output_url or '/'+target.relative_to(STAGE).as_posix()
    if not(url.startswith('/production/aaa-proof/assets/') and '..' not in url and '\\' not in url):raise ValueError('Invalid derivative URL')
    return {**record,'url':url,'sha256':receipt['sha256'],'bytes':target.stat().st_size,'vertices':receipt['vertices'],'triangles':receipt['triangles'],'original_native_sha256':source_hash,'material_namespace_receipt':'/'+receipt_path.relative_to(STAGE).as_posix(),'material_identity_scheme':'native-canonical-material-sha256-highbit-u64/v1'}
if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser();p.add_argument('record_json');p.add_argument('input_path');p.add_argument('output_path');p.add_argument('--output-url');a=p.parse_args()
    print(json.dumps(namespace_asset(json.loads(Path(a.record_json).read_text(encoding='utf-8')),a.input_path,a.output_path,output_url=a.output_url),indent=2))
