"""Promote the reviewed native entry locally, retaining the rejected entry as evidence."""
from pathlib import Path
import shutil,json,hashlib
ROOT=Path(__file__).resolve().parents[1]
REPO=Path('C:/github/webizen-website')
result=json.loads((ROOT/'evidence/native-world/interaction/result.json').read_text())
assert result['passed'] and result['full_scene_triangles']==597854
entry=(ROOT/'site/native.html').read_text(encoding='utf-8')
assert entry.count('<script')==1 and '/pkg/native-world/webizen_world.js' in entry
archive=REPO/'migration/rejected-js-world';archive.mkdir(parents=True,exist_ok=True)
old=REPO/'site/index.html'
if old.exists() and old.read_text(encoding='utf-8')!=entry:
 digest=hashlib.sha256(old.read_bytes()).hexdigest()[:16]
 shutil.copy2(old,archive/('index-'+digest+'.html'))
(ROOT/'site/index.html').write_text(entry,encoding='utf-8')
for relative in ['site/index.html','site/native.html','site/world/native.css','site/world/living-world.vibe','data/world.json','data/native-world-layout.json','site/lab/index.html','site/lab/lab.js','site/lab/lab.css','publication/publish.py','tests/native-policy.py','tests/native-world-boot.cjs','tests/native-world-interactions.cjs','tests/publication/test_publication.py','tooling/native_world_vibe.py','tooling/sync_native_preview.py','tooling/promote_native_preview.py']:
 src=ROOT/relative;dst=REPO/relative;dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(src,dst)
for relative in ['site/pkg/native-world','native/world-host','tooling/native-host','publication/generated','evidence/native-world','plans/swarm']:
 shutil.copytree(ROOT/relative,REPO/relative,dirs_exist_ok=True,ignore=shutil.ignore_patterns('target','__pycache__','artifacts'))
for name in ['README.md','integrity.json','scene.json','scene-authoring.vibe','authoring-evaluation.json','scene-authoring.json','lod-review.json','preview-review.json']:
 src=ROOT/'production/native-world'/name
 if src.exists():shutil.copy2(src,REPO/'production/native-world'/name)
for name in ['recipes','provenance']:
 shutil.copytree(ROOT/'production/native-world'/name,REPO/'production/native-world'/name,dirs_exist_ok=True,ignore=shutil.ignore_patterns('target','__pycache__'))
print('Promoted native homepage locally; previous entry retained in migration/rejected-js-world. No remote deployment.')
