from pathlib import Path
import json
p=Path('production/webizen-swarm/evidence/aaa-art/module-editorial-review.json');d=json.loads(p.read_text());d['improvements']+=['Final copy now correctly calls the Canberra dataset public elevation data and explicitly distinguishes native recipe editing from a live website wavelength control.'];d['remaining_recommendations']=d['remaining_recommendations'][2:];p.write_text(json.dumps(d,indent=2),encoding='utf8')
