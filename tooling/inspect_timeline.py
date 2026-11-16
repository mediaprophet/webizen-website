"""Read timeline structure locally without printing location values or copying raw data."""
import json,collections,datetime
from pathlib import Path
p=Path('C:/Users/Hp/Downloads/Timeline.json');data=json.loads(p.read_text(encoding='utf-8'));segments=data['semanticSegments'];kinds=collections.Counter();sample={};times=[]
for s in segments:
 for key,value in s.items():
  if key not in ('startTime','endTime','startTimeTimezoneUtcOffsetMinutes','endTimeTimezoneUtcOffsetMinutes'):
   kinds[key]+=1
   if key not in sample:
    sample[key]={k:list(v) if isinstance(v,dict) else type(v).__name__ for k,v in value.items()} if isinstance(value,dict) else (list(value[0]) if isinstance(value,list) and value and isinstance(value[0],dict) else type(value).__name__)
 if s.get('startTime'):times.append(s['startTime'])
print(json.dumps({'segment_kinds':kinds,'schema_samples':sample,'date_range':[min(times),max(times)]},indent=2))
visits=[s for s in segments if 'visit' in s];types=collections.Counter(s['visit'].get('topCandidate',{}).get('semanticType','unknown') for s in visits);print('Visit semantic type counts:',dict(types))
