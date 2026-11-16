"""Local-only timeline analysis. Emits coarse aggregate cells, never exact visit coordinates."""
from pathlib import Path
from datetime import datetime,timezone,timedelta
import json,re,math,collections
source=Path('C:/Users/Hp/Downloads/Timeline.json');data=json.loads(source.read_text(encoding='utf-8'));cut=datetime(2025,11,16,tzinfo=timezone(timedelta(hours=11)));end=datetime(2026,11,17,tzinfo=timezone(timedelta(hours=11)))
bins=collections.defaultdict(lambda:{'visits':0,'hours':0,'months':collections.Counter(),'semantic':collections.Counter()});count=0;unknown=0
for segment in data['semanticSegments']:
 if 'visit' not in segment:continue
 a=datetime.fromisoformat(segment['startTime'].replace('Z','+00:00'));b=datetime.fromisoformat(segment['endTime'].replace('Z','+00:00'))
 if b<cut or a>end:continue
 a=max(a,cut);b=min(b,end);hours=(b-a).total_seconds()/3600
 if hours<6:continue
 visit=segment['visit'];candidate=visit.get('topCandidate',{});location=candidate.get('placeLocation',{}).get('latLng','');nums=re.findall(r'-?\d+(?:\.\d+)?',location)
 if len(nums)<2:unknown+=1;continue
 lat,lon=map(float,nums[:2]);latbin=math.floor(lat/2)*2;lonbin=math.floor(lon/2)*2
 item=bins[(latbin,lonbin)];item['visits']+=1;item['hours']+=hours;item['months'][a.strftime('%Y-%m')]+=1;item['semantic'][candidate.get('semanticType','UNKNOWN')]+=1;count+=1
print(json.dumps({'period':'2025-11 to latest source record in2026-10','long_stays':count,'unlocated':unknown,'coarse_cells':[{'latitude_band':[k[0],k[0]+2],'longitude_band':[k[1],k[1]+2],**{**v,'hours':round(v['hours'])}} for k,v in sorted(bins.items(),key=lambda kv:-kv[1]['hours'])]},indent=2))
