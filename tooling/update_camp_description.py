from pathlib import Path
import json
root=Path(__file__).resolve().parents[1]
for code in ['en','en-AU']:
 p=root/'site/locales'/f'{code}.json';data=json.loads(p.read_text(encoding='utf-8'))
 data['field-studioBody']='A gold 2012–13 Volkswagen Passat wagon with a streamlined charcoal roof box, a 1980 Jayco wind-up camper, two three-panel solar arrays and one two-panel array on the ground, and a Starlink Mini on the camper roof. This campsite reflects the circumstances in which this ecosystem has been developed: living, working, making and staying connected. The vehicle and camper are authored interpretations of the described models.'
 p.write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
p=root/'tests/campsite-review.cjs';s=p.read_text(encoding='utf-8').replace('scene.nativeAssets.length===8','scene.nativeAssets.length===10');p.write_text(s,encoding='utf-8')
print('Updated Australian English campsite description and expected assembled asset count.')
