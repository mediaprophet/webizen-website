"""Copy a reviewed native proof and its exact runtime dependencies to the website repo.

This is local packaging, not deployment. It retains the homepage HTML but updates
the shared native runtime, styles and publication used by that page.
"""
from pathlib import Path
import hashlib, json, shutil, sys, os
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[1]
DEST = Path('C:/github/webizen-website').resolve()
files = set()

def add(relative):
    p = (ROOT / relative).resolve()
    if not p.is_relative_to(ROOT) or not p.is_file():
        raise ValueError(f'Invalid source: {relative}')
    files.add(p.relative_to(ROOT).as_posix())

def tree(relative, suffixes=None):
    for directory, children, names in os.walk(ROOT / relative):
        children[:] = [name for name in children if name not in ('target', '__pycache__', '.git')]
        for name in names:
            p=Path(directory)/name
            if suffixes is None or p.suffix in suffixes:
                add(p.relative_to(ROOT))

for r in ['site/aaa-proof.html','site/aaa-full-proof.html','site/aaa-instanced-proof.html','site/terrain-proof.html',
          'site/atlas/index.html','site/atlas/ecosystem.css','site/journey/index.html','site/journey/journey.css',
          'site/world/native.css','site/world/living-world.vibe','site/world/terrain-proof.vibe',
          'production/aaa-proof/scene.json','production/aaa-proof/scene-full.json',
          'production/aaa-field/manifest.json','production/aaa-field/light-field.vibe',
          'production/aaa-field/admission.json','production/aaa-terrain/integration.json',
          'production/native_bridge.py']:
    add(r)
for p in (ROOT/'site/pkg/native-world').iterdir():
    if p.is_file() and p.name != '.gitignore':
        add(p.relative_to(ROOT))
for r in ['native/world-host','native/data-pack-builder','publication/generated',
          'plans/aaa-overhaul','production/aaa-field/assets',
          'production/aaa-environment/inputs','production/aaa-terrain/inputs',
          'production/native-world/inputs']:
    tree(r)
for row in json.loads((ROOT/'evidence/aaa-data/handoff.json').read_text(encoding="utf-8"))['files']:
    p=ROOT/row['relative_path']
    if hashlib.sha256(p.read_bytes()).hexdigest()!=row['sha256']:
        raise ValueError(f'Data handoff changed: {p}')
    add(row['relative_path'])
for row in json.loads((ROOT/'tooling/native-host/production-handoff.json').read_text(encoding='utf-8'))['files']:
    p=ROOT/row['path']
    if hashlib.sha256(p.read_bytes()).hexdigest()!=row['sha256']:
        raise ValueError(f'Production handoff changed: {p}')
    add(row['path'])
for r in ['production/aaa-proof/scene.json','production/aaa-proof/scene-full.json',
          'production/aaa-instancing/scene.json',
          'data/publications/aaa/terrain-model-v2/scene.json']:
    scene=json.loads((ROOT/r).read_text(encoding="utf-8"))
    for asset in scene['assets']+scene.get('textures',[]):
        if not isinstance(asset,dict) or 'url' not in asset: continue
        rel=asset['url'].lstrip('/')
        if hashlib.sha256((ROOT/rel).read_bytes()).hexdigest()!=asset['sha256']:
            raise ValueError(f'Scene dependency changed: {rel}')
        add(rel)
        for key in ('recipe', 'provenance_receipt', 'material_namespace_receipt'):
            link=asset.get(key)
            if isinstance(link,str) and link.startswith('/'):
                add(link.lstrip('/'))
for r in ['production/aaa-proof','production/aaa-terrain','production/aaa-environment','production/aaa-instancing','production/native-world',
          'production/aaa-field','tooling','tests']:
    tree(r,{'.rs','.toml','.lock','.py','.ps1','.vibe','.md','.json','.cjs'})
for r in ['evidence/aaa-browser','evidence/aaa-full-browser','evidence/aaa-terrain-browser',
          'evidence/aaa-art','evidence/aaa-data','evidence/aaa-field','evidence/aaa-performance',
          'evidence/aaa-instanced-browser','evidence/aaa-production-http','evidence/aaa-production-browser']:
    tree(r,{'.json','.md','.png'})
add('evidence/AAA-IMPLEMENTATION-REVIEW.md')
def pin(relative,digest):
    if hashlib.sha256((ROOT/relative).read_bytes()).hexdigest()!=digest:
        raise ValueError(f'Publication pin changed: {relative}')
field=json.loads((ROOT/'production/aaa-field/manifest.json').read_text(encoding="utf-8"))['emf']
pin(field['url'].lstrip('/'),field['sha256'])
identity=json.loads((ROOT/'site/pkg/native-world/runtime-identity.json').read_text(encoding="utf-8"))
pin('site'+identity['wasm']['url'],identity['wasm']['sha256'])
pin('site/pkg/native-world/webizen_world.js',identity['generated_loader_sha256'])
current=json.loads((ROOT/'publication/generated/current.json').read_text(encoding="utf-8"))
pin('publication/generated/'+current['q42'],current['sha256'])
if '--check' in sys.argv:
    print(json.dumps({'preflight':True,'files':len(files),'bytes':sum((ROOT/r).stat().st_size for r in files)}))
    raise SystemExit(0)
receipt=[]
backup=DEST/'evidence'/'prior-local-files'/datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')
for rel in sorted(files):
    src=ROOT/rel; dst=(DEST/rel).resolve()
    if not dst.is_relative_to(DEST): raise ValueError('Destination escaped repository')
    dst.parent.mkdir(parents=True,exist_ok=True)
    digest=hashlib.sha256(src.read_bytes()).hexdigest()
    if not dst.exists() or hashlib.sha256(dst.read_bytes()).hexdigest()!=digest:
        if dst.exists():
            saved=backup/rel
            saved.parent.mkdir(parents=True,exist_ok=True)
            shutil.copy2(dst,saved)
        shutil.copy2(src,dst)
    if hashlib.sha256(dst.read_bytes()).hexdigest()!=digest: raise ValueError(rel)
    receipt.append({'path':rel,'bytes':src.stat().st_size,'sha256':digest})
result={'schema':'webizen.local-proof-copy/v1','destination':str(DEST),
        'homepage_html_replaced':False,'shared_homepage_dependencies_updated':True,
        'external_publication':False,'previous_files_backup':str(backup),'files':receipt}
for base in [ROOT,DEST]:
    (base/'evidence/aaa-local-copy.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'files':len(receipt),'bytes':sum(x['bytes'] for x in receipt),'destination':str(DEST)}))
