from pathlib import Path
import urllib.request,urllib.parse,json,hashlib,datetime,csv
ROOT=Path(__file__).resolve().parents[2]
out=ROOT/'data/acquisition/aaa/jpl-horizons';out.mkdir(parents=True,exist_ok=True)
records=[]
for body,code in [('earth','399'),('mars','499')]:
 params={'format':'json','COMMAND':"'%s'"%code,'MAKE_EPHEM':"'YES'",'EPHEM_TYPE':"'VECTORS'",'CENTER':"'500@10'",'START_TIME':"'2026-11-16'",'STOP_TIME':"'2026-11-17'",'STEP_SIZE':"'1 d'",'OUT_UNITS':"'KM-S'",'VEC_TABLE':"'2'",'REF_SYSTEM':"'ICRF'",'REF_PLANE':"'FRAME'",'TIME_TYPE':"'TDB'",'CSV_FORMAT':"'YES'"}
 url='https://ssd.jpl.nasa.gov/api/horizons.api?'+urllib.parse.urlencode(params)
 dest=out/(body+'.response.json')
 with urllib.request.urlopen(url,timeout=45) as response:
  raw=response.read(250001);headers=dict(response.headers)
 if len(raw)>250000:raise RuntimeError('bounded download exceeded')
 data=json.loads(raw)
 if 'error' in data:raise RuntimeError(data['error'])
 text=data['result'];assert '$$SOE' in text and '$$EOE' in text
 rows=[]
 for row in csv.reader(text.split('$$SOE')[1].split('$$EOE')[0].strip().splitlines()):
  rows.append({'jd_tdb':float(row[0]),'calendar_tdb':row[1].strip(),'position_km':[float(v) for v in row[2:5]],'velocity_km_s':[float(v) for v in row[5:8]]})
 assert len(rows)==2
 dest.write_bytes(raw)
 record={'id':'jpl-'+body+'-20261116','body':body,'target_id':code,'source_url':url,'source_sha256':hashlib.sha256(raw).hexdigest(),'source_bytes':len(raw),'retrieved_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'api_signature':data.get('signature'),'frame':'ICRF frame plane','centre':'Sun centre500@10','time_scale':'TDB','position_unit':'km','velocity_unit':'km/s','rows':rows,'response_headers':headers,'source_text_header':text.split('$$SOE')[0]}
 records.append(record)
(out/'vectors.json').write_text(json.dumps({'schema':'webizen.jpl-vectors/v1','records':records},indent=2),encoding='utf-8')
print(json.dumps({'records':len(records),'source_bytes':sum(r['source_bytes'] for r in records),'path':str(out/'vectors.json')}))
