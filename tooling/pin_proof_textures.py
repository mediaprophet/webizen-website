"""Pin exact native texture dependencies by content digest for stable review URLs."""
from pathlib import Path
import hashlib,json,shutil
ROOT=Path(__file__).resolve().parents[1]
DEST=ROOT/'production/aaa-proof/textures'
def pin(textures):
    output=[];DEST.mkdir(parents=True,exist_ok=True)
    for texture in textures:
        source=ROOT/texture['url'].lstrip('/')
        target=DEST/(texture['sha256']+'.png')
        if target.exists():
            assert hashlib.sha256(target.read_bytes()).hexdigest()==texture['sha256'],'Pinned texture changed'
        else:
            if hashlib.sha256(source.read_bytes()).hexdigest()!=texture['sha256']:
                # Recover the published version from immutable native producer
                # exports while a new version is being authored at its work path.
                matches=list((ROOT/'production/aaa-terrain/artifacts').glob('*/textures/sha256/'+texture['sha256']))
                source=next((p for p in matches if hashlib.sha256(p.read_bytes()).hexdigest()==texture['sha256']),source)
            assert hashlib.sha256(source.read_bytes()).hexdigest()==texture['sha256'],'No exact native texture version available'
            shutil.copyfile(source,target)
        output.append({**texture,'url':'/production/aaa-proof/textures/'+target.name,'source_url':texture.get('source_url',texture['url'])})
    return output
if __name__=='__main__':
    for name in ['scene.json','scene-full.json']:
        path=ROOT/'production/aaa-proof'/name
        data=json.loads(path.read_text());data['textures']=pin(data.get('textures',[]));path.write_text(json.dumps(data,indent=2))
    print('Pinned native texture dependencies')
