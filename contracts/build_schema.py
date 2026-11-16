"""Build the checked-in website schema. No native API or receipt is rewritten."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def obj(properties,required=None):return {'type':'object','properties':properties,'required':list(properties) if required is None else required,'additionalProperties':False}
def arr(items,minimum=0):return {'type':'array','items':items,'minItems':minimum}
def enum(*values):return {'enum':list(values)}
def ref(name):return {'$ref':'#/$defs/'+name}
def nullable(schema):return {'anyOf':[schema,{'type':'null'}]}
STR={'type':'string','minLength':1}
NUM={'type':'number'}
INT={'type':'integer','minimum':0}
BOOL={'type':'boolean'}
ID={'type':'string','pattern':'^[a-z][a-z0-9._-]{0,127}$'}
HASH={'type':'string','pattern':'^[a-f0-9]{64}$'}
LOCALE={'type':'string','pattern':'^[A-Za-z]{2,8}(?:-[A-Za-z0-9]{1,8})*$'}
URI={'type':'string','format':'public-reference'}
TIME={'type':'string','format':'date-time'}
VEC={'type':'array','items':NUM,'minItems':3,'maxItems':3}
defs={}
defs['Source']=obj({'repository':STR,'revision':STR,'path':{'type':'string','format':'repository-path'},'sha256':HASH})
defs['Evidence']=obj({'kind':enum('source','runtime','recorded','visual-review','test-fixture'),'uri':URI,'sha256':HASH,'observed_at':nullable(TIME)})
defs['Runtime']=obj({'id':ID,'profile':enum('browser','native','ci','recorded'),'version':STR,'device':nullable(STR),'backend':nullable(STR)})
def record(name,properties,optional=()):
 props={'schema':{'const':'webizen.'+name+'/v1'},'id':ID,**properties}
 return obj(props,[key for key in props if key not in optional])
defs['Module']=record('module',{'name':STR,'purpose':STR,'difference':STR,'human_significance':STR,'qualification':enum('source-indexed','runtime-verified','not-yet-qualified'),'sources':arr(ref('Source'),1),'capabilities':arr(STR),'runtime_ids':arr(ID),'related_modules':arr(ID),'evidence':arr(ref('Evidence'))})
defs['Exhibit']=record('exhibit',{'title_key':STR,'body_key':STR,'module_ids':arr(ID,1),'experiment_id':nullable(ID),'environment':ID,'position':VEC,'camera':VEC,'reading_route':URI,'asset_ids':arr(ID),'related_exhibits':arr(ID)})
defs['Input']=obj({'id':ID,'type':enum('number','integer','boolean','select','string'),'label_key':STR,'unit':nullable(STR),'minimum':NUM,'maximum':NUM,'max_length':{'type':'integer','minimum':1},'options':arr(STR,1)},['id','type','label_key','unit'])
defs['Output']=obj({'name':ID,'media_type':STR,'unit':nullable(STR),'description':STR})
defs['Experiment']=record('experiment',{'module_ids':arr(ID,1),'runner_id':ID,'runner_version':STR,'execution':enum('browser','native-opt-in','ci-only','recorded'),'source':ref('Source'),'inputs':arr(ref('Input')),'outputs':arr(ref('Output'),1),'epistemic_class':enum('measured','model-estimate','artistic','executable-logic'),'runtime_ids':arr(ID,1),'max_duration_ms':{'type':'integer','minimum':1},'max_input_bytes':{'type':'integer','minimum':1},'supports_cancel':BOOL,'supports_reset':BOOL,'evidence':arr(ref('Evidence'))})
defs['TestRun']=record('test-run',{'suite_id':ID,'runtime_id':ID,'source':ref('Source'),'status':enum('passed','failed','skipped','not-run','unsupported'),'assertion_count':INT,'duration_ms':nullable({'type':'number','minimum':0}),'executed_at':nullable(TIME),'reason':nullable(STR),'evidence':arr(ref('Evidence'))})
defs['Benchmark']=record('benchmark',{'workload_id':ID,'runtime_id':ID,'source':ref('Source'),'classification':enum('measured','historical','proxy','not-run'),'correctness':enum('passed','failed','not-checked'),'dataset_id':STR,'dataset_sha256':nullable(HASH),'method':STR,'sample_unit':STR,'samples':nullable(arr({'type':'number','minimum':0})),'sample_count':INT,'warmup_count':INT,'summary':nullable(obj({'min':NUM,'max':NUM,'mean':NUM,'p50':NUM,'p95':NUM,'p99':NUM},['min','max','mean'])),'executed_at':nullable(TIME),'exclusions':arr(STR),'evidence':arr(ref('Evidence'))})
defs['Message']=obj({'value':{'type':'string'},'language':LOCALE,'review_state':enum('draft','reviewed','stale'),'reviewer':nullable(STR)})
defs['Locale']=record('locale',{'locale':LOCALE,'default':BOOL,'source_locale':{'const':'en-AU'},'source_revision':STR,'direction':enum('ltr','rtl'),'messages':{'type':'object','additionalProperties':ref('Message')},'fallbacks':arr(LOCALE),'evidence':arr(ref('Evidence'))})
defs['Asset']=record('asset',{'uri':URI,'media_type':STR,'bytes':INT,'sha256':HASH,'origin':enum('native-production','imported','source-generated'),'recipe':nullable(ref('Source')),'receipt':nullable(ref('Evidence')),'preview_uri':nullable(URI),'visual_acceptance':enum('pending','accepted','rejected','not-applicable'),'evidence':arr(ref('Evidence'))})
defs['Observer']=record('observer',{'purpose':ID,'locale':LOCALE,'support_preferences':arr(ID),'authority':obj({'status':enum('anonymous','verified'),'grant_refs':arr(URI),'evidence':arr(ref('Evidence'))}),'graph_sha256':HASH,'policy_version':STR,'selected_exhibits':arr(ID),'reasons':arr(STR),'authority_from_purpose':{'const':False}})
defs['Migration']=record('migration',{'legacy_route':URI,'source_sha256':HASH,'destination':nullable(URI),'disposition':enum('preserve','recreate','improve','archive','unavailable'),'module_ids':arr(ID),'experiment_ids':arr(ID),'qualification':enum('source-only','parity-verified','improvement-verified','archive-reviewed','unavailable-reviewed'),'reason':nullable(STR),'evidence':arr(ref('Evidence'))})
collections={'modules':'Module','exhibits':'Exhibit','experiments':'Experiment','test_runs':'TestRun','benchmarks':'Benchmark','locales':'Locale','assets':'Asset','observers':'Observer','migrations':'Migration'}
defs['Publication']=record('publication',{'stage':enum('draft','review','release'),'default_locale':{'const':'en-AU'},'created_at':TIME,'sources':arr(ref('Source'),1),'runtimes':arr(ref('Runtime')),**{key:arr(ref(name)) for key,name in collections.items()},'graph':nullable(obj({'uri':URI,'sha256':HASH,'format':{'const':'q42'},'reader_identity':STR,'roundtrip_evidence':ref('Evidence')}))})
schema={'$schema':'https://json-schema.org/draft/2020-12/schema','$id':'https://webizen.au/schemas/publication-v1.schema.json','title':'Webizen website publication v1','$ref':'#/$defs/Publication','$defs':defs}
if __name__=='__main__':
 out=ROOT/'schemas/publication-v1.schema.json';out.parent.mkdir(parents=True,exist_ok=True);out.write_text(json.dumps(schema,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print(out)
