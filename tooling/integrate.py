"""Copy accepted owned outputs into the dedicated website without deleting existing work."""
from pathlib import Path
import shutil,json,hashlib
stage=Path(__file__).resolve().parents[1];repo=Path('C:/github/webizen-website')
def copy_changed(src,dst):
 src,dst=Path(src),Path(dst)
 if dst.exists() and src.stat().st_size==dst.stat().st_size:
  if hashlib.sha256(src.read_bytes()).digest()==hashlib.sha256(dst.read_bytes()).digest():return str(dst)
 return shutil.copy2(src,dst)
for name in ['site','schemas','contracts','migration','evidence','production','design','publication','adapters','content','tests','lab','data','plans','native']:
 source=stage/name
 if source.exists():shutil.copytree(source,repo/name,dirs_exist_ok=True,copy_function=copy_changed,ignore=shutil.ignore_patterns('__pycache__','target','node_modules','*.pdb'))
routes=stage/'migration/baseline/routes.json'
if routes.exists():
 raw=json.loads(routes.read_text(encoding='utf-8'));data=json.loads((repo/'data/demos.json').read_text(encoding='utf-8'));data['items']=[dict(source_route=x['source_route'],title=x.get('title'),section=x.get('section'),verification='source inventoried; runtime qualification recorded separately',source_sha256=x.get('source_sha256')) for x in raw['routes']];(repo/'data/demos.json').write_text(json.dumps(data,ensure_ascii=False,indent=2),encoding='utf-8')
print('Integrated additive website outputs from',stage)
shutil.copy2(stage/'tooling/world_server.py',repo/'tooling/world_server.py')
for name in ['baseline','journey','native-host']:
 source=stage/'tooling'/name
 if source.exists():shutil.copytree(source,repo/'tooling'/name,dirs_exist_ok=True,copy_function=copy_changed,ignore=shutil.ignore_patterns('__pycache__'))
index=repo/'site/index.html';html=index.read_text(encoding='utf-8')
if 'href="/lab/"' not in html:
 html=html.replace('</nav>','<a href="/lab/">Infosphere Lab ↗</a></nav>',1)
 index.write_text(html,encoding='utf-8')
