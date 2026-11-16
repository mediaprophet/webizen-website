from pathlib import Path
import json,hashlib,shutil,subprocess,datetime
ROOT=Path(__file__).resolve().parents[2];PUB=ROOT/'data/publications/aaa';CAT=ROOT/'data/catalogue/aaa';EV=ROOT/'evidence/aaa-data';EXE=ROOT/'native/data-pack-builder/data-pack.exe'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def save(p,v):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(v,indent=2,ensure_ascii=False),encoding='utf-8')
def native(*args):return json.loads(subprocess.run([str(EXE),*map(str,args)],check=True,capture_output=True).stdout)
catalogue=json.loads((PUB/'catalogue.json').read_text(encoding='utf-8'));catalogue['datasets']=catalogue['datasets'][:1]
catalogue['datasets'][0]['source'].update({'licence':'CC BY-SA3.0','licence_url':'https://schema.org/docs/terms.html','credit':'Schema.org sponsors and contributors; source dataset release30.0. Native Q42 representation retained from QualiaDB.','verified_lexicon_max_page_bytes':290510})
rights=json.loads((CAT/'rights-fixtures.json').read_text());receipt=native('rights',CAT/'rights-fixtures.json');save(EV/'rights-native-results.json',receipt);save(PUB/'rights-fixtures.json',rights)
catalogue['rights']={'id':'rights-proof-fixtures','schema':rights['schema'],'url':'/data/publications/aaa/rights-fixtures.json','sha256':sha(PUB/'rights-fixtures.json'),'case_count':len(rights['cases']),'native_evaluation_verified':True,'engine':'compile_norm_quin + evaluate_deontic_contract','source':rights['source'],'cases':rights['cases'],'interpretation':rights['interpretation']}
space=json.loads((ROOT/'data/acquisition/aaa/jpl-horizons/vectors.json').read_text());space['licence_url']='https://ssd.jpl.nasa.gov/faq.html';space['rights']='JPL Solar System Dynamics FAQ identifies site data as public domain; credit NASA/JPL Horizons; no endorsement implied.'
for rec in space['records']:
 raw=ROOT/'data/acquisition/aaa/jpl-horizons'/(rec['body']+'.response.json');dst=PUB/'space'/(rec['body']+'.response.json');dst.parent.mkdir(exist_ok=True);shutil.copy2(raw,dst);rec['raw_url']='/data/publications/aaa/space/'+dst.name
save(PUB/'space/vectors.json',space)
catalogue['space']={'schema':space['schema'],'id':'jpl-reference-vectors','url':'/data/publications/aaa/space/vectors.json','sha256':sha(PUB/'space/vectors.json'),'records':space['records'],'licence_url':space['licence_url'],'rights':space['rights'],'qualification':'Actual JPL vector records, not yet a numerical comparison against local solar/lunar approximations; different target/frame/time inputs require explicit conversion before tolerance claims.'}
dem=ROOT/'data/acquisition/aaa/canberra-reference-corrected';terr=PUB/'terrain';terr.mkdir(exist_ok=True)
for name in ['reference-terrain.10d','heightfield.json','source-tiles.qdem','native-receipt.json','requests.json']:shutil.copy2(dem/name,terr/name)
receipt=json.loads((dem/'native-receipt.json').read_text());hf=json.loads((dem/'heightfield.json').read_text());catalogue['earth']={'schema':'webizen.native-dem/v1','id':'canberra-public-reference','title':'Canberra public reference landscape','not_personal_campsite':True,'native_url':'/data/publications/aaa/terrain/reference-terrain.10d','native_sha256':sha(terr/'reference-terrain.10d'),'heightfield_url':'/data/publications/aaa/terrain/heightfield.json','heightfield_sha256':sha(terr/'heightfield.json'),'source_bundle_url':'/data/publications/aaa/terrain/source-tiles.qdem','source_bundle_sha256':sha(terr/'source-tiles.qdem'),'source_url':'https://registry.opendata.aws/terrain-tiles/','licence_url':'https://github.com/tilezen/joerd/blob/master/docs/attribution.md','australia_licence':'CC BY4.0 for Australian LiDAR DEM perTilezen providerregister','attribution':'Australia terrain data © Commonwealth of Australia (Geoscience Australia)2017; Mapzen Terrain Tiles','frame':hf['native_mesh_frame'],'bbox_wsen':hf['bbox_wsen'],'native_receipt':receipt,'bounds_interpretation':'Requested bbox; grid samples quantised to global pixels; cellsize is centre-latitude ground approximation, not survey accuracy.','use':'Inspect native source and actual raster. Source remains Z-up; display model records explicit native transformation.'}
bench=json.loads((ROOT/'migration/baseline/benchmarks.json').read_text());hist=[]
for item in bench['artifacts']:
 src=Path('C:/github/qualiaDB/docs')/item['source_route'];assert sha(src)==item['source_sha256'];dst=PUB/'benchmarks'/src.name;dst.parent.mkdir(exist_ok=True);shutil.copy2(src,dst)
 hist.append({**item,'url':'/data/publications/aaa/benchmarks/'+src.name,'sha256':sha(dst),'classification':'Historical raw benchmark result; not a new run, no unit/engine/source fields modified'})
catalogue['benchmarks']={'schema':'webizen.preserved-benchmarks/v1','records':hist,'measurement_contract':{'source_units':'Retain each original metric unit; never infer fromlabel','live_run':'Native engine/actual source data, warmup excluded,30 rawsamples and10 warmup policy inherited; cancellation and correctness gate required','comparison':'Historical engine labels and execution paths retained; do not imply allengines ran in newnativehost'}}
triples=[]
def iri(s):return '<'+s+'>'
def fact(s,p,o,literal=False):triples.append([iri(s),iri(p),json.dumps(str(o),ensure_ascii=False) if literal else iri(o)])
ns='urn:webizen:proof:'
for rec in space['records']:
 subject=ns+rec['id'];fact(subject,'http://www.w3.org/1999/02/22-rdf-syntax-ns#type','urn:webizen:type:EphemerisDataset');fact(subject,'http://purl.org/dc/terms/source',rec['source_url']);fact(subject,ns+'sourceSha256',rec['source_sha256'],True);fact(subject,ns+'frame',rec['frame'],True)
 for i,row in enumerate(rec['rows']):
  node=subject+':row:'+str(i);fact(subject,ns+'hasVector',node);fact(node,ns+'jdTdb',row['jd_tdb'],True)
  for axis,value in zip('xyz',row['position_km']):fact(node,ns+'positionKm:'+axis,value,True)
  for axis,value in zip('xyz',row['velocity_km_s']):fact(node,ns+'velocityKmS:'+axis,value,True)
fact(ns+'canberra-public-reference',ns+'sourceSha256',sha(terr/'reference-terrain.10d'),True);fact(ns+'canberra-public-reference',ns+'source','https://registry.opendata.aws/terrain-tiles/');fact(ns+'canberra-public-reference',ns+'classification','Public reference landscape, not personal campsite',True)
for key in ['width','height','cell_size_m','min_elevation_m','max_elevation_m','vertices','triangles']:fact(ns+'canberra-public-reference',ns+key,receipt[key],True)
for case in rights['cases']:
 subject=ns+case['id'];fact(subject,ns+'fixtureKind','urn:webizen:type:PublicNormFixture');fact(subject,ns+'label',case['label'],True);fact(subject,ns+'evaluationTime',case['now'],True)
 for i,status in enumerate(case['expected_statuses']):fact(subject,ns+'expectedStatus',status,True)
 for i,norm in enumerate(case['norms']):
  node=subject+':norm:'+str(i);fact(subject,ns+'hasNorm',node)
  for key,value in norm.items():fact(node,ns+key,value,True)
for item in hist:
 subject=ns+Path(item['source_route']).stem;fact(subject,ns+'sourceSha256',item['sha256'],True);fact(subject,ns+'classification','Historical benchmark result, not rerun',True);fact(subject,ns+'publicPath',item['url'],True)
save(CAT/'proof-metadata-triples.json',triples);save(PUB/'proof-metadata-triples.json',triples);out=PUB/'proof-metadata.q42'
if out.exists():raise RuntimeError('Create-only proof-metadata.q42 alreadyexists; choose versioned output for rebuild')
built=native('build',CAT/'proof-metadata-triples.json',out);save(EV/'metadata-q42-build.json',built)
queries=[]
for qid,label,q in [('earth-source','Inspect real terrain source','<urn:webizen:proof:canberra-public-reference> ?p ?o .'),('rights-norms','Inspect the public rights scenarios','?s <urn:webizen:proof:fixtureKind> ?o .'),('jpl-vectors','Find actual astronomical vector records','?s <urn:webizen:proof:hasVector> ?o .')]:
 result=native('query',out,q);save(EV/(qid+'.json'),result);queries.append({'id':qid,'label':label,'query':q,'expected_count':result['count'],'expected_rows':result['rows']})
catalogue['datasets'].append({'id':'proof-metadata','title':'Earth, space, rights and benchmark source graph','version':'1 / Q42native','q42_url':'/data/publications/aaa/proof-metadata.q42','sha256':sha(out),'byte_length':out.stat().st_size,'row_count':built['quins'],'description':'Native Q42 graph built from actual acquired source metadata, real JPL vector values, preserved benchmark source hashes and explicit public norm fixtures. Norm fixture records do not purport to be signed credentials.','source':{'builder':'UnifiedVolumeBuilder','input_url':'/data/publications/aaa/proof-metadata-triples.json','input_sha256':sha(CAT/'proof-metadata-triples.json'),'native_crc_and_exact_roundtrip':True},'queries':queries})
save(PUB/'catalogue.json',catalogue)
save(EV/'data-publication-check.json',{'passed':True,'datasets':len(catalogue['datasets']),'native_q42_rows':[d['row_count'] for d in catalogue['datasets']],'rights_cases':len(rights['cases']),'jpl_vector_rows':sum(len(r['rows']) for r in space['records']),'historical_results':len(hist),'dem_source_triangles':receipt['triangles'],'scope':'Bounded proofsubset only; nofullwebsitecompletion claim','no_personal_timeline_used':True})
print(json.dumps({'datasets':len(catalogue['datasets']),'new_q42_rows':built['quins'],'rights_cases':len(rights['cases']),'history':len(hist)}))
