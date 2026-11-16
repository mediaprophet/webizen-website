"""Publish authored public entities through the real QualiaDB Q42 codec."""
import argparse,hashlib,html,importlib.util,json,re,subprocess,sys,uuid
from datetime import datetime,timezone
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
BASE='https://webizen.au/'
W=BASE+'world/';R=BASE+'relation/';M=BASE+'module/';P=BASE+'purpose/'
J=BASE+'journey/'
JOURNEY_FIELDS=('id','title','month_bands','relative_order','stay_band','artistic_palette','landscape_copy')

def validate_journey(value):
 """Accept only the reviewed, broad public projection, never raw timeline fields."""
 if not isinstance(value,dict) or set(value)!={'schema','default_language','method','chapters'}:raise ValueError('Unknown public journey fields')
 if value['schema']!='webizen.journey/v1' or value['default_language']!='en-AU':raise ValueError('Unsupported journey version or default language')
 if not isinstance(value['method'],str) or not 1<=len(value['method'])<=4000:raise ValueError('Journey method is required')
 chapters=value['chapters']
 if not isinstance(chapters,list) or not 1<=len(chapters)<=64:raise ValueError('Bounded chapter list required')
 ids=set();orders=set()
 for c in chapters:
  if not isinstance(c,dict) or set(c)!=set(JOURNEY_FIELDS):raise ValueError('Unknown public chapter fields; exact locations are not publishable')
  if not isinstance(c['id'],str) or not re.fullmatch('[a-z][a-z0-9-]{0,79}',c['id']) or c['id'] in ids or c['id']=='publication':raise ValueError('Invalid or duplicate chapter id')
  ids.add(c['id'])
  if type(c['relative_order']) is not int or c['relative_order']<1 or c['relative_order'] in orders:raise ValueError('Invalid chapter order')
  orders.add(c['relative_order'])
  for key in ('title','stay_band','landscape_copy'):
   if not isinstance(c[key],str) or not 1<=len(c[key])<=2000:raise ValueError('Invalid chapter text')
  months=c['month_bands'];palette=c['artistic_palette']
  if not isinstance(months,list) or not 1<=len(months)<=24 or any(not isinstance(m,str) or not re.fullmatch(r'20\d{2}-(0[1-9]|1[0-2])',m) for m in months) or months!=sorted(set(months)):raise ValueError('Only ordered month bands are publishable')
  if not isinstance(palette,list) or not 1<=len(palette)<=16 or any(not isinstance(c,str) or not re.fullmatch('#[0-9a-fA-F]{6}',c) for c in palette):raise ValueError('Invalid artistic palette')
 if [c['relative_order'] for c in chapters]!=list(range(1,len(chapters)+1)):raise ValueError('Chapters must use contiguous reading order')
 return value

def decoded_journey(rows):
 """Project allowlisted chapter data exclusively from real native decoded terms."""
 index={}
 for s,p,o in rows:index.setdefault((s,p),[]).append(o)
 def one(s,p):
  found=index.get((s,R+p),[])
  if len(found)!=1:raise ValueError('Missing or ambiguous journey field '+p)
  return json.loads(found[0])
 chapters=[]
 linked=index.get((W+'field-studio',R+'hasChapter'),[])
 typed={s for s,p,o in rows if p==R+'kind' and o==J+'chapter'}
 if set(linked)!=typed or len(linked)!=len(typed):raise ValueError('Journey membership mismatch')
 for uri in linked:
  if not uri.startswith(J):raise ValueError('Chapter must use the journey namespace')
  c={key:one(uri,key) for key in JOURNEY_FIELDS}
  if uri!=J+c['id']:raise ValueError('Chapter URI does not match id')
  chapters.append(c)
 result={key:one(J+'publication',key) for key in ('schema','default_language','method')}
 result['chapters']=sorted(chapters,key=lambda c:c['relative_order'])
 return validate_journey(result)
def digest(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def canonical(value):return json.dumps(value,ensure_ascii=False,sort_keys=True,separators=(',',':'))
def save(path,value):path.parent.mkdir(parents=True,exist_ok=True);path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
def native(helper,*args):
 run=subprocess.run([str(helper),*map(str,args)],capture_output=True,text=True,encoding='utf-8',timeout=90)
 if run.returncode:raise RuntimeError(run.stderr.strip() or run.stdout.strip())
 return json.loads(run.stdout)
def source(path,repo,path_in_repo):return {'repository':repo,'revision':'content-sha256:'+digest(path),'path':path_in_repo,'sha256':digest(path)}
def publish(website,baseline,out,helper,journey_path=None):
 world_path=website/'data/world.json';locale_path=website/'site/locales/en-AU.json';modules_path=baseline/'modules.json'
 world=json.loads(world_path.read_text(encoding='utf-8'));language=json.loads(locale_path.read_text(encoding='utf-8'));modules=json.loads(modules_path.read_text(encoding='utf-8'))['modules']
 journey_path=Path(journey_path or website/'data/journey.json');journey=validate_journey(json.loads(journey_path.read_text(encoding='utf-8')))
 layout_path=website/'data/native-world-layout.json';vibe_path=website/'site/world/living-world.vibe'
 layout=json.loads(layout_path.read_text(encoding='utf-8'))
 if layout.get('schema')!='webizen.vibe-world-layout/v1' or layout.get('source_sha256')!=digest(vibe_path):raise ValueError('Native Vibe layout is stale or invalid')
 placements={p['id']:p for p in layout['places']}
 if set(placements)!={e['id'] for e in world['exhibits']}:raise ValueError('Native Vibe layout must cover every exhibit exactly')
 if 'field-studio' not in {e['id'] for e in world['exhibits']}:raise ValueError('Journey requires the public field-studio exhibit')
 triples=[];entities=[]
 def fact(s,p,o):triples.append([s,R+p,o])
 for key in ('schema','default_language','method'):fact(J+'publication',key,canonical(journey[key]))
 for chapter in journey['chapters']:
  uri=J+chapter['id'];fact(uri,'kind',J+'chapter');fact(W+'field-studio','hasChapter',uri)
  for key in JOURNEY_FIELDS:fact(uri,key,canonical(chapter[key]))
 for e in world['exhibits']:
  entity={'id':e['id'],'uri':W+e['id'],'title':language.get(e['id'],e['id']),'description':language.get(e['id']+'Body',language.get(e['id']+'Short','')),'reading_route':'/publication/reading.html#'+e['id'],'source_links':['/source/'+x for x in e['links']]};entities.append(entity)
  fact(entity['uri'],'kind',W+e['kind']);fact(entity['uri'],'title',json.dumps(entity['title'],ensure_ascii=False));fact(entity['uri'],'description',json.dumps(entity['description'],ensure_ascii=False));fact(entity['uri'],'readingRoute',json.dumps(entity['reading_route']))
  for key in ('position','camera'):
   values=placements[e['id']][key]
   if len(values)!=3 or any(type(v) not in (int,float) for v in values):raise ValueError('Invalid native camera coordinates')
   fact(entity['uri'],key,canonical(values));entity[key]=values
  for link in entity['source_links']:fact(entity['uri'],'source',BASE.rstrip('/')+link)
 module_lookup={m['name'].casefold():m['id'] for m in modules};module_lookup.update({'hypermedia':'hypermedia','webizen':'webizen','web civics':'web-civics','ecopunk':'ecopunk'})
 for e,entity in zip(world['exhibits'],entities):
  missing=[name for name in e['modules'] if name.casefold() not in module_lookup]
  if missing:raise ValueError('Unmapped module names: '+str(missing))
  entity['module_ids']=[module_lookup[name.casefold()] for name in e['modules']]
  for module_id in entity['module_ids']:fact(W+e['id'],'module',M+module_id)
 for edge in world['relationships']:fact(W+edge['from'],edge['kind'],W+edge['to'])
 # Authored graph policy facts choose anchors and relationship priorities, not hardcoded complete routes.
 policies={'learn':('river','knowledge'),'research':('observatory','information'),'create':('atelier','knowledge'),'participate':('commons','socioeconomic')}
 for purpose,(start,relation) in policies.items():fact(P+purpose,'startsAt',W+start);fact(P+purpose,'prefersRelation',R+relation)
 for m in modules:fact(M+m['id'],'kind',M+'module');fact(M+m['id'],'title',json.dumps(m['name'],ensure_ascii=False));fact(M+m['id'],'purpose',json.dumps(m['purpose'],ensure_ascii=False))
 triples=sorted({tuple(x) for x in triples});triples=[list(x) for x in triples]
 canonical_digest=hashlib.sha256(canonical(triples).encode()).hexdigest();out.mkdir(parents=True,exist_ok=True)
 triple_path=out/'triples.json';save(triple_path,triples)
 q42=out/('world-'+canonical_digest[:16]+'.q42')
 if q42.exists():build=native(helper,'inspect',q42)
 else:build=native(helper,'build',triple_path,q42)
 loaded=native(helper,'inspect',q42)
 if sorted(loaded['rows'])!=triples:raise RuntimeError('Native Q42 decoded triples differ from authored publication')
 replay=out/('replay-'+uuid.uuid4().hex[:12]+'.q42');native(helper,'build',triple_path,replay);replay_equal=digest(q42)==digest(replay);replay_decoded=native(helper,'inspect',replay);canonical_replay_equal=sorted(replay_decoded['rows'])==triples
 if not canonical_replay_equal:raise RuntimeError('Replay changed canonical graph triples')
 # Retain replay bytes as evidence; no byte-level format construction occurs in Python.
 def ttl_term(term):return term if term.startswith('"') else json.dumps(term,ensure_ascii=False) if term.startswith(('[','{')) else '<'+term+'>'
 (out/'world.ttl').write_text('\n'.join(' '.join(map(ttl_term,row))+' .' for row in triples)+'\n',encoding='utf-8')
 graph_uri='/publication/'+q42.name
 decoded=decoded_journey(loaded['rows'])
 if decoded!=journey:raise RuntimeError('Native journey projection changed sanitised source semantics')
 input_sources=[(world_path,'data/world.json'),(locale_path,'site/locales/en-AU.json'),(modules_path,'migration/baseline/modules.json'),(journey_path,'data/journey.json'),(layout_path,'data/native-world-layout.json'),(vibe_path,'site/world/living-world.vibe')]
 revision='content-sha256:'+hashlib.sha256(''.join(digest(path) for path,name in input_sources).encode()).hexdigest()
 sources=[{'repository':'webizen-website','revision':revision,'path':name,'sha256':digest(path)} for path,name in input_sources]
 receipt={'schema':'webizen.q42-publication-evidence/v1','observed_at':datetime.now(timezone.utc).isoformat(),'canonical_triples_sha256':canonical_digest,'q42_uri':graph_uri,'q42_sha256':digest(q42),'q42_bytes':q42.stat().st_size,'replay_uri':'/publication/'+replay.name,'replay_sha256':digest(replay),'strict_replay_verified':replay_equal,'canonical_replay_equal':canonical_replay_equal,'native_byte_replay_note':'QualiaDB UnifiedVolumeBuilder records current timestamps in the DAG and header; native bytes are not expected to replay identically. No header/DAG bytes are rewritten.','decoded_triples_equal':True,'native':build,'readback':{'reader':loaded['reader'],'version':loaded['version'],'verified_blocks':loaded['verified_blocks'],'total_quins':loaded['total_quins']},'adapter_binary_sha256':digest(helper),'scope':'Authored public website entities; no personal data or authority grants; actual QualiaDB codec/reader, native token hashing and embedded lexicon.'}
 receipt['sources']=sources
 receipt['journey']={'chapters':len(decoded['chapters']),'source_sha256':digest(journey_path),'semantic_roundtrip_equal':True,'projection_uri':'/publication/journey.json','default_language':'en-AU','method':'Allowlisted chapter properties decoded from the native Q42 reader; field-studio hasChapter membership, ordered by relative_order.'}
 save(out/'q42-receipt.json',receipt)
 journey_projection={**decoded,'graph':{'format':'q42','uri':graph_uri,'sha256':digest(q42),'receipt_uri':'/publication/q42-receipt.json','reader':loaded['reader'],'verified_blocks':loaded['verified_blocks']},'projection':{'schema':'webizen.journey-q42-projection/v1','source_sha256':digest(journey_path),'semantic_roundtrip_equal':True,'method':receipt['journey']['method']}}
 save(out/'journey.json',journey_projection)
 projection={'schema':'webizen.public-entities/v1','default_locale':'en-AU','graph':{'uri':graph_uri,'sha256':digest(q42),'receipt_uri':'/publication/q42-receipt.json'},'entities':entities,'modules':[{k:m[k] for k in ('id','name','purpose','publication_status')} for m in modules],'relationships':world['relationships'],'purposes':list(policies)};save(out/'entities.json',projection)
 (out/'reading.html').write_text('<!doctype html><html lang="en-AU"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Living-world publication</title><style>body{font:18px/1.7 system-ui;max-width:850px;margin:auto;padding:2rem;color:#203c32;background:#f3efdf}section{margin:3rem 0}a{color:#15594e}</style><a href="/">Return to the world</a><h1>A living world, shared understanding</h1>'+''.join('<section id="'+html.escape(e['id'])+'"><h2>'+html.escape(e['title'])+'</h2><p>'+html.escape(e['description'])+'</p><p>'+''.join('<a href="'+html.escape(link)+'">Explore the source demonstration</a> ' for link in e['source_links'])+'</p></section>' for e in entities)+'</html>',encoding='utf-8')
 # The source identity is the combined reviewed input snapshot; each input keeps its own byte hash.
 module_ids={m['name'].casefold():m['id'] for m in modules};module_ids.update({'hypermedia':'hypermedia','webcivics':'web-civics','webizen host':'webizen-host','vibe':'vibescript','webizen':'webizen','web civics':'web-civics','ecopunk':'ecopunk'})
 def rec(kind,id,**kwargs):return {'schema':'webizen.'+kind+'/v1','id':id,**kwargs}
 bundle=rec('publication','living-world',stage='draft',default_locale='en-AU',created_at=receipt['observed_at'],sources=sources,runtimes=[{'id':'native-q42','profile':'native','version':'qualia-core-db/0.0.41; adapter '+digest(helper)[:16],'device':None,'backend':'native Q42 writer/reader'}],modules=[rec('module',m['id'],name=m['name'],purpose=m['purpose'],difference='Module-specific comparison awaits source-qualified editorial review.',human_significance='Part of the human-centric computing ecosystem; detailed human-facing profile is in editorial development.',qualification='not-yet-qualified',sources=[sources[2]],capabilities=[],runtime_ids=[],related_modules=[],evidence=[]) for m in modules],exhibits=[],experiments=[],test_runs=[],benchmarks=[],locales=[rec('locale','locale-en-au',locale='en-AU',default=True,source_locale='en-AU',source_revision=revision,direction='ltr',messages={k:{'value':v,'language':'en-AU','review_state':'draft','reviewer':None} for k,v in language.items()},fallbacks=[],evidence=[])],assets=[],observers=[],migrations=[],graph={'uri':graph_uri,'sha256':digest(q42),'format':'q42','reader_identity':loaded['reader'],'roundtrip_evidence':{'kind':'runtime','uri':'/publication/q42-receipt.json','sha256':digest(out/'q42-receipt.json'),'observed_at':receipt['observed_at']}})
 for e in world['exhibits']:
  mids=sorted({module_ids.get(n.casefold()) for n in e['modules']} - {None})
  if not mids:raise RuntimeError('No source-qualified module mapping for '+e['id'])
  bundle['exhibits'].append(rec('exhibit',e['id'],title_key=e['id'],body_key=e['id']+'Body',module_ids=mids,experiment_id=None,environment=world['environment'],position=placements[e['id']]['position'],camera=placements[e['id']]['camera'],reading_route='/publication/reading.html#'+e['id'],asset_ids=[],related_exhibits=sorted({x['to'] for x in world['relationships'] if x['from']==e['id']})))
 spec=importlib.util.spec_from_file_location('publication_validator',ROOT/'contracts/validate.py');validator=importlib.util.module_from_spec(spec);spec.loader.exec_module(validator);validator.validate(bundle);save(out/'publication.json',bundle)
 save(out/'current.json',{'q42':q42.name,'sha256':digest(q42),'entities':'entities.json','publication':'publication.json','receipt':'q42-receipt.json','journey':'journey.json'})
 print(json.dumps({'q42':str(q42),'quins':loaded['total_quins'],'roundtrip':True,'replay':replay_equal,'entities':len(entities),'modules':len(modules),'validated_publication':True}));return receipt
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--website',type=Path,default=Path('C:/github/webizen-website'));p.add_argument('--baseline',type=Path,default=ROOT/'migration/baseline');p.add_argument('--out',type=Path,default=ROOT/'publication/generated');p.add_argument('--helper',type=Path,default=ROOT/'adapters/qualiadb/q42_web.exe');p.add_argument('--journey',type=Path);a=p.parse_args();publish(a.website,a.baseline,a.out,a.helper,a.journey)
