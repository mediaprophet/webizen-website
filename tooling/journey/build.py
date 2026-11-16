"""Read a local timeline and export only coarse, curated landscape chapters.

The input is never copied, hashed or included in public output. No network calls.
Run: python tooling/journey/build.py --source <local timeline JSON>
"""
from pathlib import Path
from datetime import datetime, timezone, timedelta
import argparse, json, math, re
from html import escape

ROOT=Path(__file__).resolve().parents[2]
TZ=timezone(timedelta(hours=11))
START=datetime(2025,11,16,tzinfo=TZ)
END=datetime(2026,11,17,tzinfo=TZ)  # Inclusive through 16 November.
RULES=[
 ('southern-tablelands','Southern Tablelands & Canberra region',[(-36,148)],['#556856','#b3b298','#d9cfa8','#637b88'],'Open grasslands meet wooded ridges. A chapter of broad skies, cool light and the meeting of country and city.'),
 ('northern-rivers','Northern Rivers & far north coast',[(-30,152)],['#294e43','#73926b','#b7c9a0','#92b7bd'],'Rain-green hills, river valleys and coastal light suggest a softer, more abundant landscape. Water connects the forest with the sea.'),
 ('central-victoria','Central Victoria',[(-38,144)],['#626c4c','#b29262','#d6c49c','#81989b'],'Dry woodland and open country carry a quieter palette of straw, bark and stone. Long light draws out the folds of the land.'),
 ('western-slopes','North-west NSW & western slopes',[(-30,148),(-32,148)],['#756b4c','#ae8257','#d4b68b','#889caa'],'The horizon opens across plains and slopes. Earth, grass and changing skies give this chapter its sense of space.'),
 ('high-country','High country & eastern Victoria',[(-38,146),(-38,148)],['#455e57','#84917c','#c6c9b5','#8a9fab'],'Layered ranges, tall forest and cooler air shape a mountain palette. Distance appears in successive shades of blue and green.'),
 ('south-coast','South Coast',[(-38,150),(-36,150)],['#356a68','#729c8b','#d7d0ae','#a5bdc7'],'Forest approaches the shore. Sea light, weathered wood and sheltered waterways bring the living coast into the field studio.'),
 ('riverina','Riverina',[(-36,146)],['#69724f','#b29d65','#d8cba4','#8faaa7'],'River country and open plains invite a slower view. Muted greens and warm ground colours follow the changing light.'),
]
METHOD=('These seven landscape chapters draw on longer visits in a locally held timeline, '
        'from mid-November 2025 through mid-November 2026. The available record ends in October 2026. '
        'Nearby visits are grouped into broad artistic regions; their names are approximate, not mapped boundaries. '
        'Month bands show when a chapter appears, and the reading order follows its first appearance. '
        'A visit record is not a campsite. The field studio is a shared artistic setting for exploring these landscapes.')

def parse_time(value):
    result=datetime.fromisoformat(value.replace('Z','+00:00'))
    if result.tzinfo is None: raise ValueError('Timeline timestamps must include a timezone')
    return result

def clipped_visit(segment):
    if 'visit' not in segment: return None
    a=max(parse_time(segment['startTime']),START)
    b=min(parse_time(segment['endTime']),END)
    if (b-a).total_seconds()<6*3600: return None
    text=segment['visit'].get('topCandidate',{}).get('placeLocation',{}).get('latLng','')
    numbers=re.findall(r'-?\d+(?:\.\d+)?',text)
    if len(numbers)!=2: return None
    lat,lon=map(float,numbers)
    if not (-90<=lat<=90 and -180<=lon<=180): return None
    return a,b,(math.floor(lat/2)*2,math.floor(lon/2)*2)

def month_bands(a,b):
    # Retain source offset for month boundaries; never export a day or time.
    last=(b-timedelta(microseconds=1)).astimezone(a.tzinfo)
    year,month=a.year,a.month
    result=[]
    while (year,month)<=(last.year,last.month):
        result.append(f'{year:04d}-{month:02d}')
        year,month=(year+1,1) if month==12 else (year,month+1)
    return result

def export(data):
    buckets={r[0]:{'first':None,'seconds':0,'months':set()} for r in RULES}
    lookup={cell:r[0] for r in RULES for cell in r[2]}
    for segment in data.get('semanticSegments',[]):
        item=clipped_visit(segment)
        if item is None: continue
        a,b,cell=item
        if cell not in lookup:
            raise ValueError('An eligible visit falls outside the approved chapter mapping; review locally before export')
        bucket=buckets[lookup[cell]]
        bucket['first']=a if bucket['first'] is None else min(a,bucket['first'])
        bucket['seconds']+=(b-a).total_seconds()
        bucket['months'].update(month_bands(a,b))
    active=sorted((r for r in RULES if buckets[r[0]]['first'] is not None),key=lambda r:buckets[r[0]]['first'])
    chapters=[]
    for order,(id,title,cells,palette,copy) in enumerate(active,1):
        b=buckets[id];days=b['seconds']/86400
        chapters.append({'id':id,'title':title,'month_bands':sorted(b['months']),
                         'relative_order':order,'stay_band':'An extended chapter' if days>=30 else 'A settled chapter' if days>=7 else 'A brief chapter',
                         'artistic_palette':palette,'landscape_copy':copy})
    return {'schema':'webizen.journey/v1','default_language':'en-AU','method':METHOD,'chapters':chapters}

def human_month(value):
    return datetime.strptime(value,'%Y-%m').strftime('%B %Y')

def page(data):
    cards=[];nav=[]
    for c in data['chapters']:
        id=escape(c['id']);title=escape(c['title']);palette=c['artistic_palette']
        nav.append(f'<a href="#{id}">{title}</a>')
        months=' · '.join(human_month(m) for m in c['month_bands'])
        swatches=''.join(f'<span style="background:{p}"></span>' for p in palette)
        cards.append(f'''<article class="chapter" id="{id}" style="--earth:{palette[0]};--leaf:{palette[1]};--light:{palette[2]};--sky:{palette[3]}">
          <div class="landscape" aria-hidden="true"><span class="chapter-number">{c['relative_order']:02d}</span><div class="swatches">{swatches}</div></div>
          <div class="chapter-copy"><p class="eyebrow">{escape(c['stay_band'])}</p><h2>{title}</h2>
          <p>{escape(c['landscape_copy'])}</p><p class="months"><span>In the record</span>{escape(months)}</p>
          <a class="text-link" href="/?chapter={id}#field-studio">Explore this chapter in the field studio <span aria-hidden="true">↗</span></a></div>
        </article>''')
    return f'''<!doctype html>
<html lang="en-AU"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="description" content="Seven broad landscape chapters, and a field studio for exploring a human centric computing ecosystem."><title>A year in landscapes · Webizen</title><link rel="stylesheet" href="/lab/lab.css"><link rel="stylesheet" href="/journey/journey.css"></head>
<body><a class="skip" href="#chapters">Skip to landscape chapters</a><header><a class="brand" href="/">w<span>webizen<small>Human centric computing</small></span></a><nav aria-label="Main"><a href="/#field-studio">The field studio</a><a href="/lab/">Infosphere lab</a></nav><span class="locale">English · AU</span></header>
<main><section class="journey-intro"><p class="eyebrow">The field studio · A year in landscapes</p><h1>A place to pause.<br>A world to explore.</h1><p>Landscapes change as we travel. The questions we carry with us—how to live, learn, create and stay connected—travel too.</p><p>These broad chapters bring a personal journey into the field studio: an invitation to explore human centric computing in the company of the natural world.</p><a class="primary" href="/#field-studio">Enter the field studio <span aria-hidden="true">↗</span></a></section>
<nav class="chapter-nav" aria-label="Landscape chapters">{''.join(nav)}</nav><section id="chapters" aria-label="A year in landscape chapters">{''.join(cards)}</section>
<section class="method section"><p class="eyebrow">About this journey</p><h2>A landscape reading</h2><p class="reading-width">{escape(data['method'])}</p><p class="reading-width">The colours and landscape studies are artistic interpretations. The chapters carry broad regions and months; the original timeline stays local.</p></section></main>
<footer><span>Webizen · Human centric computing</span><a href="/">Explore the ecosystem</a><a href="/lab/">Open the infosphere lab</a></footer></body></html>'''

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--source',required=True,type=Path);parser.add_argument('--check',action='store_true');args=parser.parse_args()
    data=export(json.loads(args.source.read_text(encoding='utf-8')))
    outputs={ROOT/'data/journey.json':json.dumps(data,ensure_ascii=False,indent=2)+'\n',ROOT/'site/journey/index.html':page(data)}
    for path,content in outputs.items():
        if args.check:
            assert path.read_text(encoding='utf-8')==content, f'Generated output differs: {path.name}'
        else:
            path.parent.mkdir(parents=True,exist_ok=True);path.write_text(content,encoding='utf-8')
    print(json.dumps({'passed':True,'chapters':len(data['chapters']),'mode':'check' if args.check else 'write'}))

if __name__=='__main__':main()
