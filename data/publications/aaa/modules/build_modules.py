"""Source-backed editorial graph; native codecs and query VM do all Q42 work."""
from pathlib import Path
import json,hashlib,subprocess,urllib.parse
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[3]; PUB=HERE.parent; EXE=ROOT/'native/data-pack-builder/data-pack.exe'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def write(p,v):p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
def native(*args):return json.loads(subprocess.run([str(EXE),*map(str,args)],capture_output=True,check=True).stdout)
def git(repo,*args):return subprocess.check_output(['git','--git-dir='+str(repo/'.git'),'--work-tree='+str(repo),*args],stderr=subprocess.DEVNULL).decode('utf8').strip()
editorial=read(HERE/'editorial-input.en-AU.json')
baseline=read(ROOT/'migration/baseline/modules.json')['modules']; original=read(ROOT/'content/modules/profiles.en-AU.json')['modules']; relations={m['id']:m.get('related_modules',[]) for m in original}
assert set(editorial)=={m['id'] for m in baseline} and len(editorial)==16
old_handoff=read(ROOT/'evidence/aaa-data/handoff.json')
# Preserve every previously qualified file except the consciously extended catalogue.
for row in old_handoff['files']:
 if row['relative_path']!='data/publications/aaa/catalogue.json' and not row['relative_path'].startswith('data/publications/aaa/modules/'):assert sha(ROOT/row['relative_path'])==row['sha256'],row['relative_path']
repo_meta={}; sources={}; records=[]
previous_sources=read(HERE/'narratives.en-AU.json').get('sources',{}) if (HERE/'narratives.en-AU.json').exists() else {}
for ident,m in editorial.items():
 refs=[]
 for repo_name,relative in m.pop('refs'):
  repo=Path('C:/github')/repo_name; p=repo/relative
  assert p.is_file(),str(p)
  if repo_name not in repo_meta:
   revision=git(repo,'rev-parse','HEAD')
   origin=git(repo,'config','--get','remote.origin.url')
   if origin.startswith('git@github.com:'):origin='https://github.com/'+origin.split(':',1)[1]
   if origin.endswith('.git'):origin=origin[:-4]
   if not origin.startswith('https://github.com/'):origin=None
   repo_meta[repo_name]={'repository':repo_name,'revision':revision,'repository_url':origin}
  meta=repo_meta[repo_name]
  previous=previous_sources.get(repo_name+':'+relative,{})
  if previous.get('revision')==meta['revision'] and previous.get('sha256')==sha(p):
   changed=previous['working_tree_modified_or_untracked']
  else:
   committed=subprocess.run(['git','--git-dir='+str(repo/'.git'),'show','HEAD:'+relative],capture_output=True)
   changed=committed.returncode!=0 or hashlib.sha256(committed.stdout).hexdigest()!=sha(p)
  source_id=repo_name+':'+relative
  ref={**meta,'path':relative,'sha256':sha(p),'working_tree_modified_or_untracked':changed,'evidence_kind':'inspected current local source','reference_url':(meta['repository_url']+'/blob/'+meta['revision']+'/'+urllib.parse.quote(relative,safe='/')) if meta['repository_url'] else None,'reference_scope':'Revision link identifies repository baseline; recorded SHA256 identifies inspected working bytes. Local changes may not be in the linked revision.'}
  sources[source_id]=ref;refs.append(source_id)
 rec={'id':ident,'locale':'en-AU',**m,'related_modules':relations.get(ident,[]),'source_refs':refs,'narrative_status':'source-reviewed editorial explanation','native_demo_status':m['status'],'native_demo_scope':m['demo'],'application_owner_status':'separate Poet application owner not reconciled; shared QualiaDB host mapped' if ident=='poet' else 'source repositories identified; runtime scope separately qualified'}
 records.append(rec)
article={'schema':'webizen.module-narratives/v1','locale':'en-AU','review_date':'2026-11-16','scope':'All 16 module groups explained; editorial and graph-data migration only. This does not complete their outstanding interactive demonstrations.','rights_interpretation':'Human significance and safeguard explanations are editorial design reasoning, not legal advice, clinical validation or an assertion of compliance.','modules':records,'sources':sources,'inventory_source':{'url':'/plans/aaa-overhaul/CAPABILITY-AND-DEMO-COVERAGE.json','module_inventory_sha256':sha(ROOT/'migration/baseline/modules.json')},'working_tree_policy':'Current source bytes are identified by SHA256 as well as repository revision; a dirty source is never represented as the clean commit.'}
write(HERE/'narratives.en-AU.json',article)
U=lambda x:'<'+x+'>'
L=lambda x:json.dumps(x,ensure_ascii=False)+'@en-AU'
BASE='urn:webizen:module:'; P='urn:webizen:module-property:'; triples=[]
def add(s,p,o):triples.append([U(s),U(p),o])
for m in records:
 subject=BASE+m['id'];add(subject,'http://www.w3.org/1999/02/22-rdf-syntax-ns#type',U('urn:webizen:type:EcosystemModule'))
 for field in ['title','what','distinctive','importance','rights','example','native_demo_scope','application_owner_status']:add(subject,P+field,L(m[field]))
 add(subject,P+'locale',L('en-AU'));add(subject,P+'nativeDemoStatus',U('urn:webizen:demo-status:'+m['native_demo_status']))
 for related in m['related_modules']:assert related in editorial;add(subject,P+'worksWith',U(BASE+related))
 for source in m['source_refs']:add(subject,'http://purl.org/dc/terms/source',U('urn:webizen:module-source:'+urllib.parse.quote(source,safe='')))
for ident,ref in sources.items():
 subject='urn:webizen:module-source:'+urllib.parse.quote(ident,safe='')
 for field in ['repository','revision','path','sha256']:add(subject,P+field,L(ref[field]))
 add(subject,P+'workingTreeState',U('urn:webizen:source-state:'+('modified-or-untracked' if ref['working_tree_modified_or_untracked'] else 'matches-revision')))
 if ref['reference_url']:add(subject,P+'revisionReference',U(ref['reference_url']))
assert len(triples)==len({tuple(t) for t in triples})
write(HERE/'triples.json',triples)
input_hash=sha(HERE/'triples.json'); q42=HERE/('module-narratives-'+input_hash[:12]+'.q42')
if q42.exists():build={'passed':True,'cache_reuse':True,'cache_input_sha256':input_hash,'note':'Existing create-only Q42 revalidated by full native decoded query below.'}
else:build=native('build',HERE/'triples.json',q42)
all_rows=native('query',q42,'?s ?p ?o .')
assert all_rows['count']==len(triples) and {tuple(t) for t in all_rows['rows']}=={tuple(t) for t in triples}
fixtures=[('all-modules','Read all 16 module names','?s <'+P+'title> ?o .',lambda t:t[1]==U(P+'title')),('human-purpose','Why each part matters','?s <'+P+'importance> ?o .',lambda t:t[1]==U(P+'importance')),("rights-and-agency","Inspect each module's human-agency considerations",'?s <'+P+'rights> ?o .',lambda t:t[1]==U(P+'rights')),('demo-status','Which demonstrations are actually migrated?','?s <'+P+'nativeDemoStatus> ?o .',lambda t:t[1]==U(P+'nativeDemoStatus')),('module-relationships','How the module groups work together','?s <'+P+'worksWith> ?o .',lambda t:t[1]==U(P+'worksWith')),('poet-boundary','Inspect Poet and its ownership boundary','<'+BASE+'poet> ?p ?o .',lambda t:t[0]==U(BASE+'poet')),('qdnf-boundary','Inspect networking scope without a public-mesh claim','<'+BASE+'qdnf> ?p ?o .',lambda t:t[0]==U(BASE+'qdnf')),('unmigrated','Find module groups whose demonstrations remain to be built','?s <'+P+'nativeDemoStatus> <urn:webizen:demo-status:not-yet-migrated> .',lambda t:t[1]==U(P+'nativeDemoStatus') and t[2]==U('urn:webizen:demo-status:not-yet-migrated')),('missing-module','Verify that an unknown module is not invented','<'+BASE+'not-a-real-module> ?p ?o .',lambda t:False)]
queries=[];checks=[]
for ident,label,q,predicate in fixtures:
 expected=[t for t in triples if predicate(t)]; actual=native('query',q42,q)
 assert actual['count']==len(expected) and {tuple(t) for t in actual['rows']}=={tuple(t) for t in expected},ident
 queries.append({'id':ident,'label':label,'query':q,'expected_count':len(expected),'expected_rows':actual['rows']})
 checks.append({'id':ident,'passed':True,'count':actual['count'],'vm_cycles':actual['vm_cycles'],'expected_results_source':'independently selected exact authored input triples; native decoded rows compared as sets'})
cat_path=PUB/'catalogue.json';cat=read(cat_path); old_catalogue_hash=sha(cat_path)
entry={'id':'ecosystem-module-narratives-en-au','title':'The ecosystem - 16 module groups explained','version':'2026-11-16 / '+input_hash[:12]+' / native Q42v4','q42_url':'/'+q42.relative_to(ROOT).as_posix(),'sha256':sha(q42),'byte_length':q42.stat().st_size,'row_count':len(triples),'description':'Human-readable Australian English explanations of what16 module groups do, why their design matters and how they relate to people, rights and the wider ecosystem. Native demo status and source revisions are data, not implied completion.','source':{'builder':'UnifiedVolumeBuilder','input_url':'/data/publications/aaa/modules/triples.json','input_sha256':input_hash,'narrative_url':'/data/publications/aaa/modules/narratives.en-AU.json','narrative_sha256':sha(HERE/'narratives.en-AU.json'),'native_crc_and_exact_roundtrip':True,'source_repository_count':len(repo_meta),'source_reference_count':len(sources),'module_count':16,'licence_scope':'New user-commissioned editorial explanations for this Webizen project. Source code licences remain with their owning repositories; no blanket relicensing is asserted.'},'queries':queries}
cat['datasets']=[d for d in cat['datasets'] if d['id']!=entry['id']]+[entry]
assert len(cat['datasets'])==3
write(cat_path,cat)
report={'schema':'webizen.native-module-data-acceptance/v1','passed':True,'module_count':16,'source_repository_count':len(repo_meta),'source_reference_count':len(sources),'build':build,'native_file_sha256':sha(q42),'native_binary_sha256':sha(EXE),'input_sha256':input_hash,'all_rows_roundtrip':{'passed':True,'count':len(triples),'lexicon_values_equal':True,'native_block_verification':True},'queries':checks,'prior_catalogue_sha256':old_catalogue_hash,'catalogue_sha256':sha(cat_path),'prior_datasets_preserved':True,'scope':'Narratives and native queryable graph only; no additional module UI/demo completion, no benchmark rerun.'}
write(HERE/'native-validation.json',report)
# Keep the prior whitelist, replacing only its explicitly extended catalogue hash.
paths={r['relative_path'] for r in old_handoff['files'] if not (r['relative_path'].startswith('data/publications/aaa/modules/module-narratives-') and r['relative_path'].endswith('.q42'))}
paths.update(p.relative_to(ROOT).as_posix() for p in HERE.iterdir() if p.is_file() and p.suffix in ['.json','.q42','.py','.md'] and (p.suffix!='.q42' or p==q42))
rows=[{'relative_path':p,'bytes':(ROOT/p).stat().st_size,'sha256':sha(ROOT/p)} for p in sorted(paths)]
handoff={**old_handoff,'files':rows,'total_bytes':sum(r['bytes'] for r in rows),'module_narratives':{'module_count':16,'dataset_id':entry['id'],'native_validation':'data/publications/aaa/modules/native-validation.json','prior_qualified_files_preserved_except_catalogue':True},'scope':'Selected proof data plus all 16 source-backed module explanations; no fullsite or unmigrated demo completion.'}
write(ROOT/'evidence/aaa-data/handoff.json',handoff)
print(json.dumps({'passed':True,'modules':16,'triples':len(triples),'q42':str(q42),'sha256':sha(q42),'queries':[(x['id'],x['count']) for x in checks],'handoff_files':len(rows),'catalogue_sha256':sha(cat_path)},ensure_ascii=False))
