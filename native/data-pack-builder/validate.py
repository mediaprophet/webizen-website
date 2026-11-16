from pathlib import Path
import json,hashlib,subprocess
ROOT=Path(__file__).resolve().parents[2];PUB=ROOT/'data/publications/aaa';EXE=ROOT/'native/data-pack-builder/data-pack.exe'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def local(url):
 assert url.startswith('/data/publications/aaa/'),url
 path=ROOT/url.lstrip('/');assert path.is_file(),url;return path
cat=json.loads((PUB/'catalogue.json').read_text(encoding='utf-8'));checks=[]
for dataset in cat['datasets']:
 path=local(dataset['q42_url']);assert sha(path)==dataset['sha256']
 for q in dataset['queries']:
  actual=json.loads(subprocess.run([str(EXE),'query',str(path),q['query']],capture_output=True,check=True).stdout)
  assert actual['total_quins']==dataset['row_count'];assert actual['count']==q['expected_count']
  if 'expected_rows' in q:assert actual['rows']==q['expected_rows']
  assert all(all(x is not None for x in row) for row in actual['rows'])
  checks.append({'id':dataset['id']+':'+q['id'],'count':actual['count'],'passed':True})
for rec in cat['space']['records']:assert sha(local(rec['raw_url']))==rec['source_sha256']
assert sum(len(x['rows']) for x in cat['space']['records'])==4
for rec in cat['benchmarks']['records']:assert sha(local(rec['url']))==rec['sha256']
assert len(cat['benchmarks']['records'])==5
rights=json.loads(subprocess.run([str(EXE),'rights',str(PUB/'rights-fixtures.json')],capture_output=True,check=True).stdout);assert rights['passed']
asset=cat['earth']['display_model']['asset'];assert sha(local(asset['url']))==asset['sha256']
model=json.loads(local(cat['earth']['display_model']['manifest_url']).read_text());assert model['production']['receipt']['strict_replay_verified'];assert model['admission']['passed'] and not model['admission']['native_governance_refused']
raw=json.loads((ROOT/'data/acquisition/aaa/canberra-reference-corrected/mesh-inspection.json').read_text());fixed=json.loads((ROOT/'data/acquisition/aaa/canberra-reference-corrected/mesh-inspection-fixed.json').read_text());assert raw['positions']==fixed['positions'];assert len(raw['indices'])==len(fixed['indices'])==33462*3
for i in range(0,len(raw['indices']),3):assert fixed['indices'][i:i+3]==[raw['indices'][i],raw['indices'][i+2],raw['indices'][i+1]]
for src in [ROOT/'data/acquisition/aaa/jpl-horizons/vectors.json',PUB/'rights-fixtures.json',PUB/'catalogue.json']:
 text=src.read_text(encoding='utf-8');assert 'Timeline.json' not in text and 'placeId' not in text and 'semanticSegments' not in text
out={'schema':'webizen.aaa-data-validation/v1','passed':True,'query_checks':checks,'rights_cases':len(rights['cases']),'historical_benchmark_files':5,'actual_ephemeris_rows':4,'source_positions_unchanged':16992,'native_winding_fixed_triangles':33462,'native_asset_sha256':asset['sha256'],'native_asset_admitted':True,'scope':'Boundedproofdata; nofullsitecompletionornewbenchmarkclaim'}
(ROOT/'evidence/aaa-data/final-validation.json').write_text(json.dumps(out,indent=2),encoding='utf-8');print(json.dumps(out))
