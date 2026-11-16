from pathlib import Path
import json,shutil,sys
root=Path(__file__).resolve().parents[1];repo=Path('C:/github/webizen-website')
def copy(relative):
 src=root/relative;dst=repo/relative;dst.parent.mkdir(parents=True,exist_ok=True)
 if src.is_dir():shutil.copytree(src,dst,dirs_exist_ok=True)
 else:shutil.copy2(src,dst)
for relative in ['site/native.html','site/world/native.css','site/world/living-world.vibe','site/pkg/native-world','publication/generated','production/native-world/prototype.json']:
 copy(relative)
prototype=json.loads((root/'production/native-world/prototype.json').read_text(encoding='utf-8'))
for asset in prototype['assets']:copy(asset['url'].lstrip('/'))
if '--full' in sys.argv:
 copy('production/native-world/scene.json')
 scene=json.loads((root/'production/native-world/scene.json').read_text(encoding='utf-8'))
 for asset in scene['assets']:copy(asset['url'].lstrip('/'))
html=(root/'site/native.html').read_text(encoding='utf-8').replace('id="native-world" aria-label','id="native-world" data-scene="/production/native-world/prototype.json" aria-label').replace('<title>Webizen — The living world</title>','<title>Native renderer engineering check</title>')
(repo/'site/native-prototype.html').write_text(html,encoding='utf-8')
print('Native WASM package, real Q42 and explicitly labelled prototype synced. Main homepage not changed.')
